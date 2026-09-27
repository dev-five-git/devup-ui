//! Vanilla-extract style file (.css.ts, .css.js) processor
//!
//! This module uses `boa_engine` to execute vanilla-extract style files
//! and extract style definitions for processing by the existing extract logic.

use crate::utils::keeps_bare_number;
use boa_engine::{
    Context, JsArgs, JsObject, JsResult, JsString, JsValue, NativeFunction, Source, js_string,
    object::{FunctionObjectBuilder, ObjectInitializer, builtins::JsArray},
    property::{Attribute, PropertyKey},
};
use css::file_map::get_file_num_by_filename;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};
use oxc_transformer::{TransformOptions, Transformer};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

/// A `style()` or `keyframes()` call
#[derive(Debug, Clone, Default)]
pub struct StyleEntry {
    /// The rule object JSON
    pub json: String,
    /// Whether the variable holding it is exported
    pub exported: bool,
    /// Styles composed before this one: placeholders, names once named
    pub bases: SmallVec<[String; 2]>,
    /// Class names composed as they are
    pub classes: SmallVec<[String; 1]>,
}

/// What a vanilla-extract stylesheet defines
#[derive(Debug, Default, Clone)]
pub struct CollectedStyles {
    /// `style()` calls, every `styleVariants()` variant included: name -> entry
    pub styles: FxHashMap<String, StyleEntry>,
    /// `keyframes()` calls: name -> entry
    pub keyframes: FxHashMap<String, StyleEntry>,
    /// Global rules in call order: selector -> rule object JSON. Themes declare
    /// their variables here, on their class or selector.
    pub global_styles: Vec<(String, String)>,
    /// `@font-face` rule objects, each naming its `fontFamily`
    pub font_faces: Vec<String>,
    /// Layers in declaration order
    pub layers: Vec<String>,
    /// `@property` rules registered by `createVar()`
    pub property_rules: Vec<String>,
    /// Exported values other than styles and keyframes: name -> code
    pub constant_exports: Vec<(String, String)>,
    /// `__style_N__` placeholder -> what it stands for, for placeholders left in
    /// selectors and values
    pub references: FxHashMap<String, Reference>,
}

/// Target of a placeholder that a selector or value interpolated
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Reference {
    /// `style()`: its name and the unique class it gains when referenced
    Style { name: String, class_name: String },
    /// `keyframes()`: its name, resolved to the generated keyframes name
    Keyframes(String),
}

/// State the mock API shares while a stylesheet runs
#[derive(Default)]
struct Collector {
    styles: CollectedStyles,
    file_num: usize,
    placeholders: usize,
    identifiers: usize,
}

type StyleCollector = Rc<RefCell<Collector>>;

impl Collector {
    fn placeholder(&mut self) -> String {
        let id = format!("__style_{}__", self.placeholders);
        self.placeholders += 1;
        id
    }

    /// A name no other stylesheet produces: `debug_id` (or `fallback`) made an
    /// identifier, then the file number and a per-file counter
    fn identifier(&mut self, debug_id: Option<String>, fallback: &str) -> String {
        let base = debug_id
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| fallback.to_string());
        let mut name: String = base
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if !name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') {
            name.insert(0, '_');
        }
        let id = format!("{name}-{}-{}", self.file_num, self.identifiers);
        self.identifiers += 1;
        id
    }
}

fn js_str(value: &JsValue) -> Option<String> {
    value.as_string().map(|s| s.to_std_string_escaped())
}

fn to_text(value: &JsValue, context: &mut Context) -> JsResult<String> {
    Ok(value.to_string(context)?.to_std_string_escaped())
}

/// String and index keys of `object`, in property order
fn own_keys(object: &JsObject, context: &mut Context) -> JsResult<Vec<(PropertyKey, String)>> {
    Ok(object
        .own_property_keys(context)?
        .into_iter()
        .filter_map(|key| {
            let name = match &key {
                PropertyKey::String(s) => s.to_std_string_escaped(),
                PropertyKey::Index(i) => i.get().to_string(),
                PropertyKey::Symbol(_) => return None,
            };
            Some((key, name))
        })
        .collect())
}

/// Elements of `value` when it is an array
fn array_items(value: &JsValue, context: &mut Context) -> JsResult<Option<Vec<JsValue>>> {
    let Some(array) = value.as_object().filter(JsObject::is_array) else {
        return Ok(None);
    };
    let length = array.get(js_string!("length"), context)?.to_u32(context)?;
    (0..length)
        .map(|index| array.get(index, context))
        .collect::<JsResult<Vec<_>>>()
        .map(Some)
}

/// The custom property `var(--x)` reads
fn var_name(reference: &str) -> &str {
    reference
        .strip_prefix("var(")
        .and_then(|name| name.strip_suffix(')'))
        .unwrap_or(reference)
}

fn json_string(text: &str) -> String {
    serde_json::Value::String(text.to_string()).to_string()
}

/// `JSON.stringify(value, replacer)`, `None` when it produces no string
fn stringify(value: &JsValue, replacer: JsValue, context: &mut Context) -> Option<String> {
    let json = context.intrinsics().objects().json();
    let stringify = json.get(js_string!("stringify"), context).ok()?;
    stringify
        .as_callable()?
        .call(&JsValue::undefined(), &[value.clone(), replacer], context)
        .ok()?
        .as_string()
        .map(|s| s.to_std_string_escaped())
}

/// Serialize `value` as it is
fn js_value_to_json(value: &JsValue, context: &mut Context) -> String {
    stringify(value, JsValue::undefined(), context).unwrap_or_else(|| "{}".to_string())
}

/// `JSON.stringify` replacer adding `px` to numbers the way vanilla-extract does,
/// so they are not read as Devup UI's spacing scale, and keying `vars` by the
/// custom properties their `var()` references name.
fn pixelify(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let key = to_text(args.get_or_undefined(0), context)?;
    let value = args.get_or_undefined(1);
    if key == "vars"
        && let Some(vars) = value.as_object()
    {
        let declared = ObjectInitializer::new(context).build();
        for (property, name) in own_keys(&vars, context)? {
            let value = vars.get(property, context)?;
            declared.set(js_string!(var_name(&name)), value, false, context)?;
        }
        return Ok(declared.into());
    }
    Ok(match value.as_number() {
        Some(number) if number != 0.0 && !keeps_bare_number(&key) => {
            js_string!(format!("{number}px")).into()
        }
        _ => value.clone(),
    })
}

/// Serialize a style rule object with vanilla-extract's number units.
fn style_to_json(value: &JsValue, context: &mut Context) -> String {
    let realm = context.realm().clone();
    let replacer =
        FunctionObjectBuilder::new(&realm, NativeFunction::from_fn_ptr(pixelify)).build();
    stringify(value, replacer.into(), context).unwrap_or_else(|| "{}".to_string())
}

/// Execute vanilla-extract style file and collect styles
pub fn execute_vanilla_extract(
    code: &str,
    package: &str,
    filename: &str,
) -> Result<CollectedStyles, String> {
    let file_num = get_file_num_by_filename(filename);
    let collector: StyleCollector = Rc::new(RefCell::new(Collector {
        file_num,
        ..Collector::default()
    }));
    let mut context = Context::default();
    register_vanilla_extract_apis(&mut context, &collector)?;

    context
        .eval(Source::from_bytes(
            preprocess_typescript(code, package).as_bytes(),
        ))
        .map_err(|e| format!("JS execution error: {e}"))?;

    let mut collected = std::mem::take(&mut collector.borrow_mut().styles);
    name_entries(
        &mut collected,
        &top_level_bindings(code),
        &mut context,
        file_num,
    );
    Ok(collected)
}

/// A name a top-level variable declaration of the stylesheet binds
struct Binding {
    name: String,
    exported: bool,
    /// Initializer source, when the declaration binds this name alone
    init: Option<String>,
}

fn top_level_bindings(code: &str) -> Vec<Binding> {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, code, SourceType::ts())
        .parse()
        .program;
    let mut bindings = Vec::new();
    for statement in &program.body {
        let (declaration, exported) = match statement {
            oxc_ast::ast::Statement::ExportDeclaration(export) => match &export.declaration {
                oxc_ast::ast::Declaration::VariableDeclaration(declaration) => (declaration, true),
                _ => continue,
            },
            oxc_ast::ast::Statement::VariableDeclaration(declaration) => (declaration, false),
            _ => continue,
        };
        for declarator in &declaration.declarations {
            let init = declarator
                .init
                .as_ref()
                .filter(|_| declarator.id.get_identifier_name().is_some())
                .map(|init| code[init.span().start as usize..init.span().end as usize].to_string());
            for identifier in declarator.id.get_binding_identifiers() {
                bindings.push(Binding {
                    name: identifier.name.to_string(),
                    exported,
                    init: init.clone(),
                });
            }
        }
    }
    bindings
}

fn placeholder_index(id: &str) -> usize {
    id.trim_start_matches("__style_")
        .trim_end_matches("__")
        .parse()
        .unwrap_or_default()
}

/// Name every style and keyframes after the variable holding it (`_veN` when
/// none does), and turn the other exported values into code.
///
/// Names come from the values the variables hold after the stylesheet ran, so
/// calls nested in objects, arrays or helpers do not shift them.
fn name_entries(
    collected: &mut CollectedStyles,
    bindings: &[Binding],
    context: &mut Context,
    file_num: usize,
) {
    let values: Vec<(&Binding, JsValue)> = bindings
        .iter()
        .filter_map(|binding| {
            context
                .eval(Source::from_bytes(binding.name.as_bytes()))
                .ok()
                .map(|value| (binding, value))
        })
        .collect();

    let mut names: FxHashMap<String, String> = FxHashMap::default();
    for (binding, value) in &values {
        if let Some(id) = js_str(value)
            && !names.contains_key(&id)
            && let Some(entry) = collected
                .styles
                .get_mut(&id)
                .or_else(|| collected.keyframes.get_mut(&id))
        {
            entry.exported = binding.exported;
            names.insert(id, binding.name.clone());
        }
    }

    let declared: FxHashSet<&str> = bindings
        .iter()
        .map(|binding| binding.name.as_str())
        .collect();
    let mut anonymous: Vec<String> = collected
        .styles
        .keys()
        .chain(collected.keyframes.keys())
        .filter(|id| !names.contains_key(*id))
        .cloned()
        .collect();
    anonymous.sort_by_key(|id| placeholder_index(id));
    for id in anonymous {
        let mut name = format!("_ve{}", placeholder_index(&id));
        while declared.contains(name.as_str()) {
            name.push('_');
        }
        names.insert(id, name);
    }

    for (binding, value) in &values {
        let names_entry = js_str(value).is_some_and(|id| names.get(&id) == Some(&binding.name));
        if binding.exported
            && !names_entry
            && let Some(code) = value_to_code(value, context, &names, &mut Vec::new())
                .or_else(|| binding.init.clone())
        {
            collected
                .constant_exports
                .push((binding.name.clone(), code));
        }
    }

    for (id, name) in &names {
        let reference = if let Some(mut entry) = collected.styles.remove(id) {
            for base in &mut entry.bases {
                if let Some(base_name) = names.get(base.as_str()) {
                    base_name.clone_into(base);
                }
            }
            collected.styles.insert(name.clone(), entry);
            Reference::Style {
                name: name.clone(),
                class_name: format!("f{file_num}_{name}"),
            }
        } else {
            if let Some(entry) = collected.keyframes.remove(id) {
                collected.keyframes.insert(name.clone(), entry);
            }
            Reference::Keyframes(name.clone())
        };
        collected.references.insert(id.clone(), reference);
    }
}

fn is_plain_object(object: &JsObject, context: &Context) -> bool {
    object.prototype().is_none_or(|prototype| {
        JsObject::equals(
            &prototype,
            &context.intrinsics().constructors().object().prototype(),
        )
    })
}

/// JavaScript for a data `value`, placeholders turned into the names of their
/// styles; `None` for functions, class instances, symbols and cycles
fn value_to_code(
    value: &JsValue,
    context: &mut Context,
    names: &FxHashMap<String, String>,
    seen: &mut Vec<JsObject>,
) -> Option<String> {
    if let Some(text) = js_str(value) {
        return Some(string_code(&text, names));
    }
    if value.is_null_or_undefined() || value.is_number() || value.as_boolean().is_some() {
        return to_text(value, context).ok();
    }
    let object = value.as_object()?;
    if object.is_callable()
        || seen
            .iter()
            .any(|visited| JsObject::equals(visited, &object))
    {
        return None;
    }
    seen.push(object.clone());
    let mut parts = Vec::new();
    let code = if let Some(items) = array_items(value, context).ok()? {
        for item in &items {
            parts.push(value_to_code(item, context, names, seen)?);
        }
        format!("[{}]", parts.join(", "))
    } else if is_plain_object(&object, context) {
        for (key, name) in own_keys(&object, context).ok()? {
            let item = object.get(key, context).ok()?;
            parts.push(format!(
                "{}: {}",
                json_string(&name),
                value_to_code(&item, context, names, seen)?
            ));
        }
        if parts.is_empty() {
            "{}".to_string()
        } else {
            format!("{{ {} }}", parts.join(", "))
        }
    } else {
        return None;
    };
    seen.pop();
    Some(code)
}

/// A string literal, or a template literal when it interpolates placeholders
fn string_code(text: &str, names: &FxHashMap<String, String>) -> String {
    if let Some(name) = names.get(text) {
        return name.clone();
    }
    let escaped = text
        .replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace("${", "\\${");
    replace_placeholders(&escaped, |token, _| {
        names.get(token).map(|name| format!("${{{name}}}"))
    })
    .map_or_else(|| json_string(text), |template| format!("`{template}`"))
}

/// Replace every `__name__` token that `lookup` resolves; `None` when nothing
/// was replaced.
fn replace_placeholders(
    source: &str,
    mut lookup: impl FnMut(&str, &str) -> Option<String>,
) -> Option<String> {
    let mut search_start = 0;
    let mut last_copied = 0;
    let mut output = None::<String>;

    while let Some(relative_start) = source[search_start..].find("__") {
        let start = search_start + relative_start;
        let Some(relative_end) = source[start + 2..].find("__") else {
            break;
        };
        let end = start + 2 + relative_end + 2;
        if let Some(replacement) = lookup(&source[start..end], &source[end..]) {
            let output = output.get_or_insert_with(|| String::with_capacity(source.len()));
            output.push_str(&source[last_copied..start]);
            output.push_str(&replacement);
            last_copied = end;
            search_start = end;
        } else {
            search_start = start + 2;
        }
    }

    output.map(|mut output| {
        output.push_str(&source[last_copied..]);
        output
    })
}

/// Convert TypeScript to JavaScript using Oxc Transformer and replace imports
fn preprocess_typescript(code: &str, package: &str) -> String {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();

    // Parse TypeScript
    let ret = Parser::new(&allocator, code, source_type).parse();
    let mut program = ret.program;

    // Build semantic info to get scoping
    let semantic_ret = SemanticBuilder::new().build(&program);
    let scoping = semantic_ret.semantic.into_scoping();

    // Transform: strip TypeScript types
    let options = TransformOptions::default();
    let path = Path::new("input.css.ts");
    let _ = Transformer::new(&allocator, path, &options).build_with_scoping(scoping, &mut program);

    // Generate JavaScript
    let js_code = Codegen::new().build(&program).code;

    // Bind imports of the package to the mock object instead, e.g.
    // `import { style } from '@devup-ui/react'` -> `const { style } = __vanilla_extract__;`
    // Import aliases (like @vanilla-extract/css) are already transformed by import_alias_visit
    let import_patterns = [format!("from \"{package}\""), format!("from '{package}'")];
    let mut transformed = String::with_capacity(js_code.len());
    for (idx, line) in js_code.lines().enumerate() {
        if idx > 0 {
            transformed.push('\n');
        }
        if import_patterns
            .iter()
            .any(|pattern| line.contains(pattern.as_str()))
            && let Some(binding) = mock_binding(line)
        {
            transformed.push_str("const ");
            transformed.push_str(&binding);
            transformed.push_str(" = __vanilla_extract__;");
        } else {
            transformed.push_str(strip_export_keyword(line));
        }
    }
    transformed
}

/// What an import of the package binds: `{ a, b: c }` for named imports, the
/// namespace for `import * as ns`
fn mock_binding(line: &str) -> Option<String> {
    if let (Some(start), Some(end)) = (line.find('{'), line.find('}')) {
        let names: Vec<String> = line[start + 1..end]
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| name.replacen(" as ", ": ", 1))
            .collect();
        return Some(format!("{{ {} }}", names.join(", ")));
    }
    line.strip_prefix("import * as ")
        .and_then(|rest| rest.split_whitespace().next())
        .map(ToString::to_string)
}

fn strip_export_keyword(line: &str) -> &str {
    // The transformer marks a module whose imports it all removed with `export {};`
    if line == "export {};" {
        return "";
    }
    line.strip_prefix("export ").map_or(line, |rest| {
        if rest.starts_with("const ")
            || rest.starts_with("let ")
            || rest.starts_with("var ")
            || rest.starts_with("function ")
        {
            rest
        } else {
            line
        }
    })
}

type Api = fn(&StyleCollector, &[JsValue], &mut Context) -> JsResult<JsValue>;

fn api(collector: &StyleCollector, function: Api) -> NativeFunction {
    let collector = collector.clone();
    // SAFETY: the closure captures only an `Rc<RefCell<Collector>>`, which holds no
    // garbage-collected value, and a stylesheet runs on a single thread
    unsafe {
        NativeFunction::from_closure(move |_this, args, context| {
            function(&collector, args, context)
        })
    }
}

/// Register vanilla-extract mock APIs in the JS context
fn register_vanilla_extract_apis(
    context: &mut Context,
    collector: &StyleCollector,
) -> Result<(), String> {
    let apis = [
        ("style", api(collector, style), 1),
        ("globalStyle", api(collector, global_style), 2),
        ("styleVariants", api(collector, style_variants), 1),
        (
            "keyframes",
            api(collector, |collector, args, context| {
                Ok(keyframes(collector, args, context))
            }),
            1,
        ),
        (
            "fontFace",
            api(collector, |collector, args, context| {
                Ok(font_face(collector, args, context))
            }),
            1,
        ),
        ("globalFontFace", api(collector, global_font_face), 2),
        ("createVar", api(collector, create_var), 0),
        ("fallbackVar", NativeFunction::from_fn_ptr(fallback_var), 2),
        (
            "createContainer",
            api(collector, |collector, args, _context| {
                Ok(create_container(collector, args))
            }),
            0,
        ),
        ("layer", api(collector, layer), 0),
        ("globalLayer", api(collector, global_layer), 1),
        (
            "createThemeContract",
            api(collector, create_theme_contract),
            1,
        ),
        (
            "createGlobalThemeContract",
            NativeFunction::from_fn_ptr(create_global_theme_contract),
            1,
        ),
        (
            "assignVars",
            NativeFunction::from_fn_ptr(assign_vars_api),
            2,
        ),
        ("createTheme", api(collector, create_theme), 1),
        ("createGlobalTheme", api(collector, create_global_theme), 2),
    ];
    let mut builder = ObjectInitializer::new(context);
    for (name, function, length) in apis {
        builder.function(function, JsString::from(name), length);
    }
    let mock = builder.build();
    context
        .register_global_property(js_string!("__vanilla_extract__"), mock, Attribute::all())
        .map_err(|e| format!("Failed to register __vanilla_extract__: {e}"))
}

fn style(collector: &StyleCollector, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let id = register_style(collector, args.get_or_undefined(0), context)?;
    Ok(js_string!(id).into())
}

fn register_style(
    collector: &StyleCollector,
    rule: &JsValue,
    context: &mut Context,
) -> JsResult<String> {
    let mut entry = StyleEntry::default();
    let mut rules = Vec::new();
    compose(rule, &mut entry, &mut rules, context)?;
    entry.json = format!("{{{}}}", rules.join(","));
    let mut collector = collector.borrow_mut();
    let id = collector.placeholder();
    collector.styles.styles.insert(id.clone(), entry);
    Ok(id)
}

fn is_placeholder(token: &str) -> bool {
    token.starts_with("__style_") && token.ends_with("__")
}

/// Sort a composition list into the styles and classes it composes and the
/// inner JSON of its rules
fn compose(
    value: &JsValue,
    entry: &mut StyleEntry,
    rules: &mut Vec<String>,
    context: &mut Context,
) -> JsResult<()> {
    if let Some(items) = array_items(value, context)? {
        for item in &items {
            compose(item, entry, rules, context)?;
        }
    } else if let Some(classes) = js_str(value) {
        for class in classes.split_whitespace() {
            if is_placeholder(class) {
                entry.bases.push(class.to_string());
            } else {
                entry.classes.push(class.to_string());
            }
        }
    } else if value.is_object() {
        let json = style_to_json(value, context);
        let inner = inner_json(&json);
        if !inner.is_empty() {
            rules.push(inner.to_string());
        }
    }
    Ok(())
}

fn global_style(
    collector: &StyleCollector,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let selector = to_text(args.get_or_undefined(0), context)?;
    let json = style_to_json(args.get_or_undefined(1), context);
    collector
        .borrow_mut()
        .styles
        .global_styles
        .push((selector, json));
    Ok(JsValue::undefined())
}

/// `styleVariants(variants, map?)`: a style per key, of the value or of what
/// `map(value, key)` returns
fn style_variants(
    collector: &StyleCollector,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let variants = ObjectInitializer::new(context).build();
    let map = args.get_or_undefined(1).as_callable();
    if let Some(data) = args.get_or_undefined(0).as_object() {
        for (key, name) in own_keys(&data, context)? {
            let value = data.get(key, context)?;
            let rule = match &map {
                Some(map) => map.call(
                    &JsValue::undefined(),
                    &[value, js_string!(name.as_str()).into()],
                    context,
                )?,
                None => value,
            };
            let id = register_style(collector, &rule, context)?;
            variants.set(js_string!(name), js_string!(id), false, context)?;
        }
    }
    Ok(variants.into())
}

fn keyframes(collector: &StyleCollector, args: &[JsValue], context: &mut Context) -> JsValue {
    let json = style_to_json(args.get_or_undefined(0), context);
    let mut collector = collector.borrow_mut();
    let id = collector.placeholder();
    collector.styles.keyframes.insert(
        id.clone(),
        StyleEntry {
            json,
            ..StyleEntry::default()
        },
    );
    js_string!(id).into()
}

/// `fontFace(rule, debugId?)`: a generated family for one rule or a list of them
fn font_face(collector: &StyleCollector, args: &[JsValue], context: &mut Context) -> JsValue {
    let family = collector
        .borrow_mut()
        .identifier(js_str(args.get_or_undefined(1)), "font");
    let faces = font_face_rules(&family, args.get_or_undefined(0), context);
    collector.borrow_mut().styles.font_faces.extend(faces);
    js_string!(family).into()
}

fn global_font_face(
    collector: &StyleCollector,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let family = to_text(args.get_or_undefined(0), context)?;
    let faces = font_face_rules(&family, args.get_or_undefined(1), context);
    collector.borrow_mut().styles.font_faces.extend(faces);
    Ok(JsValue::undefined())
}

fn font_face_rule(family_json: &str, rule: &JsValue, context: &mut Context) -> String {
    let json = js_value_to_json(rule, context);
    match inner_json(&json) {
        "" => format!("{{\"fontFamily\":{family_json}}}"),
        inner => format!("{{{inner},\"fontFamily\":{family_json}}}"),
    }
}

/// `@font-face` rule objects for `rule`, or each rule of a list, naming `family`
fn font_face_rules(family: &str, rule: &JsValue, context: &mut Context) -> Vec<String> {
    let family_json = json_string(family);
    match array_items(rule, context).ok().flatten() {
        Some(rules) => {
            let mut faces = Vec::with_capacity(rules.len());
            for rule in &rules {
                faces.push(font_face_rule(&family_json, rule, context));
            }
            faces
        }
        None => vec![font_face_rule(&family_json, rule, context)],
    }
}

/// `createVar(debugId?)` / `createVar(declaration, debugId?)`: a `var()` of a
/// generated custom property, registered with `@property` when declared
fn create_var(styles: &StyleCollector, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let first = args.get_or_undefined(0);
    let (declaration, debug_id) = match first.as_object() {
        Some(declaration) => (Some(declaration), args.get_or_undefined(1)),
        None => (None, first),
    };
    let name = format!(
        "--{}",
        styles.borrow_mut().identifier(js_str(debug_id), "var")
    );
    if let Some(declaration) = declaration {
        let rule = property_rule(&name, &declaration, ctx)?;
        styles.borrow_mut().styles.property_rules.push(rule);
    }
    Ok(js_string!(format!("var({name})")).into())
}

fn property_rule(name: &str, declaration: &JsObject, context: &mut Context) -> JsResult<String> {
    let syntax = declaration.get(js_string!("syntax"), context)?;
    let syntax = match array_items(&syntax, context)? {
        Some(items) => items
            .iter()
            .map(|item| to_text(item, context))
            .collect::<JsResult<Vec<_>>>()?
            .join(" | "),
        None => to_text(&syntax, context)?,
    };
    let inherits = declaration
        .get(js_string!("inherits"), context)?
        .to_boolean();
    let mut rule = format!("@property {name}{{syntax:\"{syntax}\";inherits:{inherits}");
    let initial_value = declaration.get(js_string!("initialValue"), context)?;
    if !initial_value.is_null_or_undefined() {
        rule.push_str(";initial-value:");
        rule.push_str(&to_text(&initial_value, context)?);
    }
    rule.push('}');
    Ok(rule)
}

/// `fallbackVar(...values)`: each `var()` falls back to the values after it
fn fallback_var(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let mut result = String::new();
    for (index, value) in args.iter().rev().enumerate() {
        let value = to_text(value, context)?;
        result = match value.strip_suffix(')') {
            Some(head) if index > 0 => format!("{head}, {result})"),
            _ => value,
        };
    }
    Ok(js_string!(result).into())
}

fn create_container(collector: &StyleCollector, args: &[JsValue]) -> JsValue {
    let name = collector
        .borrow_mut()
        .identifier(js_str(args.get_or_undefined(0)), "container");
    js_string!(name).into()
}

/// `(parent, name argument)` of `layer(options?, debugId?)` and
/// `globalLayer(options?, name)`
fn layer_args(args: &[JsValue], context: &mut Context) -> JsResult<(Option<String>, JsValue)> {
    let first = args.get_or_undefined(0);
    Ok(match first.as_object() {
        Some(options) => (
            js_str(&options.get(js_string!("parent"), context)?),
            args.get_or_undefined(1).clone(),
        ),
        None => (None, first.clone()),
    })
}

fn declare_layer(collector: &StyleCollector, parent: Option<String>, name: String) -> JsValue {
    let name = match parent {
        Some(parent) => format!("{parent}.{name}"),
        None => name,
    };
    collector.borrow_mut().styles.layers.push(name.clone());
    js_string!(name).into()
}

fn layer(collector: &StyleCollector, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let (parent, debug_id) = layer_args(args, context)?;
    let name = collector
        .borrow_mut()
        .identifier(js_str(&debug_id), "layer");
    Ok(declare_layer(collector, parent, name))
}

fn global_layer(
    collector: &StyleCollector,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let (parent, name) = layer_args(args, context)?;
    let name = to_text(&name, context)?;
    Ok(declare_layer(collector, parent, name))
}

type Leaf<'a> = dyn FnMut(&JsValue, &[String], &mut Context) -> JsResult<JsValue> + 'a;

/// vanilla-extract's `walkObject`: `tokens` with every string, number or empty
/// leaf replaced by `leaf(value, path)`; other values are skipped
fn walk_object(
    tokens: &JsValue,
    context: &mut Context,
    path: &mut Vec<String>,
    leaf: &mut Leaf<'_>,
) -> JsResult<JsValue> {
    let walked = ObjectInitializer::new(context).build();
    if let Some(object) = tokens.as_object() {
        for (key, name) in own_keys(&object, context)? {
            let value = object.get(key, context)?;
            path.push(name.clone());
            let mapped = if value.is_string() || value.is_number() || value.is_null_or_undefined() {
                Some(leaf(&value, path, context)?)
            } else if value.as_object().is_some_and(|object| !object.is_array()) {
                Some(walk_object(&value, context, path, leaf)?)
            } else {
                None
            };
            path.pop();
            if let Some(mapped) = mapped {
                walked.set(js_string!(name), mapped, false, context)?;
            }
        }
    }
    Ok(walked.into())
}

/// A contract of `tokens`' shape whose leaves read generated custom properties
fn contract_of(
    collector: &StyleCollector,
    tokens: &JsValue,
    context: &mut Context,
) -> JsResult<JsValue> {
    walk_object(
        tokens,
        context,
        &mut Vec::new(),
        &mut |_value, path, _context| {
            let name = collector
                .borrow_mut()
                .identifier(Some(path.join("-")), "var");
            Ok(js_string!(format!("var(--{name})")).into())
        },
    )
}

fn create_theme_contract(
    collector: &StyleCollector,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    contract_of(collector, args.get_or_undefined(0), context)
}

/// `createGlobalThemeContract(tokens, map?)`: leaves name their custom property,
/// or `map(value, path)` does
fn create_global_theme_contract(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let map = args.get_or_undefined(1).as_callable();
    walk_object(
        args.get_or_undefined(0),
        context,
        &mut Vec::new(),
        &mut |value, path, context| {
            let name = match &map {
                Some(map) => {
                    let path = JsArray::from_iter(
                        path.iter().map(|key| js_string!(key.as_str()).into()),
                        context,
                    );
                    map.call(
                        &JsValue::undefined(),
                        &[value.clone(), path.into()],
                        context,
                    )?
                }
                None => value.clone(),
            };
            let name = to_text(&name, context)?;
            Ok(js_string!(format!(
                "var(--{})",
                name.strip_prefix("--").unwrap_or(&name)
            ))
            .into())
        },
    )
}

/// `(custom property, value)` for every leaf of `tokens`, read from the same
/// path of `contract`
fn assign_vars(
    contract: &JsValue,
    tokens: &JsValue,
    context: &mut Context,
) -> JsResult<Vec<(String, String)>> {
    let mut vars = Vec::new();
    walk_object(
        tokens,
        context,
        &mut Vec::new(),
        &mut |value, path, context| {
            let mut target = contract.clone();
            for key in path {
                target = match target.as_object() {
                    Some(object) => object.get(js_string!(key.as_str()), context)?,
                    None => JsValue::undefined(),
                };
            }
            if let Some(reference) = js_str(&target) {
                vars.push((var_name(&reference).to_string(), to_text(value, context)?));
            }
            Ok(JsValue::undefined())
        },
    )?;
    Ok(vars)
}

fn assign_vars_api(_this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let assigned = ObjectInitializer::new(context).build();
    for (name, value) in assign_vars(args.get_or_undefined(0), args.get_or_undefined(1), context)? {
        assigned.set(
            js_string!(format!("var({name})")),
            js_string!(value),
            false,
            context,
        )?;
    }
    Ok(assigned.into())
}

/// `tokens` without its `@layer`, and the layer it names
fn split_layer(tokens: &JsValue, context: &mut Context) -> JsResult<(JsValue, Option<String>)> {
    let Some(object) = tokens.as_object() else {
        return Ok((tokens.clone(), None));
    };
    let mut layer = None;
    let rest = ObjectInitializer::new(context).build();
    for (key, name) in own_keys(&object, context)? {
        let value = object.get(key, context)?;
        if name == "@layer" {
            layer = Some(to_text(&value, context)?);
        } else {
            rest.set(js_string!(name), value, false, context)?;
        }
    }
    Ok((rest.into(), layer))
}

/// Declare on `selector` the values `tokens` gives the variables of `contract`
fn declare_theme(
    collector: &StyleCollector,
    selector: String,
    contract: &JsValue,
    tokens: &JsValue,
    layer: Option<String>,
    context: &mut Context,
) -> JsResult<()> {
    let vars = assign_vars(contract, tokens, context)?;
    if vars.is_empty() {
        return Ok(());
    }
    let mut declarations: Vec<String> = layer
        .iter()
        .map(|layer| format!("\"@layer\":{}", json_string(layer)))
        .collect();
    for (name, value) in &vars {
        declarations.push(format!("{}:{}", json_string(name), json_string(value)));
    }
    collector
        .borrow_mut()
        .styles
        .global_styles
        .push((selector, format!("{{{}}}", declarations.join(","))));
    Ok(())
}

/// `createTheme(tokens, debugId?)` -> `[class, vars]`, or
/// `createTheme(contract, tokens, debugId?)` -> class
fn create_theme(
    collector: &StyleCollector,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let contract_given = args.get_or_undefined(1).is_object();
    let (tokens, debug_id) = if contract_given {
        (args.get_or_undefined(1), args.get_or_undefined(2))
    } else {
        (args.get_or_undefined(0), args.get_or_undefined(1))
    };
    let class_name = collector.borrow_mut().identifier(js_str(debug_id), "theme");
    let (tokens, layer) = split_layer(tokens, context)?;
    let contract = if contract_given {
        args.get_or_undefined(0).clone()
    } else {
        contract_of(collector, &tokens, context)?
    };
    declare_theme(
        collector,
        format!(".{class_name}"),
        &contract,
        &tokens,
        layer,
        context,
    )?;
    let class_name = JsValue::from(js_string!(class_name));
    Ok(if contract_given {
        class_name
    } else {
        JsArray::from_iter([class_name, contract], context).into()
    })
}

/// `createGlobalTheme(selector, tokens)` -> vars, or
/// `createGlobalTheme(selector, contract, tokens)`
fn create_global_theme(
    collector: &StyleCollector,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let selector = to_text(args.get_or_undefined(0), context)?;
    let contract_given = args.get_or_undefined(2).to_boolean();
    let (tokens, layer) = split_layer(
        args.get_or_undefined(if contract_given { 2 } else { 1 }),
        context,
    )?;
    let contract = if contract_given {
        args.get_or_undefined(1).clone()
    } else {
        contract_of(collector, &tokens, context)?
    };
    declare_theme(collector, selector, &contract, &tokens, layer, context)?;
    Ok(if contract_given {
        JsValue::undefined()
    } else {
        contract
    })
}

/// Placeholders interpolated into a selector end inside a JSON key.
fn in_json_key(rest: &str) -> bool {
    rest.find('"')
        .is_some_and(|i| rest[i + 1..].starts_with(':'))
}

impl CollectedStyles {
    fn resolve(
        &self,
        source: &str,
        keyframes_names: &FxHashMap<String, String>,
        selector: impl Fn(&str) -> bool,
    ) -> String {
        replace_placeholders(source, |token, rest| {
            Some(match self.references.get(token)? {
                Reference::Style { class_name, .. } => {
                    if selector(rest) {
                        format!(".{class_name}")
                    } else {
                        class_name.clone()
                    }
                }
                Reference::Keyframes(name) => keyframes_names.get(name)?.clone(),
            })
        })
        .unwrap_or_else(|| source.to_string())
    }

    fn resolve_json(&self, json: &str, keyframes_names: &FxHashMap<String, String>) -> String {
        self.resolve(json, keyframes_names, in_json_key)
    }

    /// Every text a placeholder can be interpolated into.
    fn texts(&self) -> impl Iterator<Item = &str> {
        self.styles
            .values()
            .chain(self.keyframes.values())
            .map(|entry| entry.json.as_str())
            .chain(
                self.global_styles
                    .iter()
                    .flat_map(|(selector, json)| [selector.as_str(), json.as_str()]),
            )
    }

    fn referenced(&self) -> FxHashSet<&Reference> {
        let mut referenced = FxHashSet::default();
        for text in self.texts() {
            replace_placeholders(text, |token, _| {
                if let Some(reference) = self.references.get(token) {
                    referenced.insert(reference);
                }
                None
            });
        }
        referenced
    }

    /// `variable_name` -> unique class of every style some selector or value refers to
    fn referenced_classes(&self) -> FxHashMap<&str, &str> {
        self.referenced()
            .into_iter()
            .filter_map(|reference| match reference {
                Reference::Style { name, class_name } => Some((name.as_str(), class_name.as_str())),
                Reference::Keyframes(_) => None,
            })
            .collect()
    }
}

/// Names of the keyframes some style refers to; their generated names have to be
/// known before the styles referring to them are generated.
pub fn referenced_keyframes(collected: &CollectedStyles) -> FxHashSet<String> {
    collected
        .referenced()
        .into_iter()
        .filter_map(|reference| match reference {
            Reference::Keyframes(name) => Some(name.clone()),
            Reference::Style { .. } => None,
        })
        .collect()
}

/// Code declaring only the keyframes in `names`, extracted first to learn their
/// generated names
pub fn keyframes_to_code(
    collected: &CollectedStyles,
    package: &str,
    names: &FxHashSet<String>,
) -> String {
    let mut keyframes: Vec<_> = collected
        .keyframes
        .iter()
        .filter(|(name, _)| names.contains(*name))
        .collect();
    keyframes.sort_by_key(|(name, _)| *name);
    let mut output = format!("import {{ keyframes }} from '{package}'");
    for (name, entry) in keyframes {
        output.push_str("\nconst ");
        output.push_str(name);
        output.push_str(" = keyframes(");
        output.push_str(&collected.resolve_json(&entry.json, &FxHashMap::default()));
        output.push(')');
    }
    output
}

/// The members of a JSON object, without its braces
fn inner_json(json: &str) -> &str {
    json.trim()
        .strip_prefix('{')
        .and_then(|json| json.strip_suffix('}'))
        .unwrap_or_default()
        .trim()
}

/// Styles `entry` composes, transitively and in order, each once
fn collect_bases<'a>(
    collected: &'a CollectedStyles,
    entry: &'a StyleEntry,
    seen: &mut FxHashSet<&'a str>,
    bases: &mut Vec<(&'a str, &'a StyleEntry)>,
) {
    for base in &entry.bases {
        if seen.insert(base.as_str())
            && let Some(base_entry) = collected.styles.get(base)
        {
            collect_bases(collected, base_entry, seen, bases);
            bases.push((base.as_str(), base_entry));
        }
    }
}

/// `css(...)` of the style `name` composed onto its bases, plus the classes it
/// composes and the unique classes that let selectors target them
fn composed_css(
    collected: &CollectedStyles,
    keyframes_names: &FxHashMap<String, String>,
    referenced_classes: &FxHashMap<&str, &str>,
    name: &str,
    entry: &StyleEntry,
) -> String {
    let mut seen = FxHashSet::default();
    seen.insert(name);
    let mut bases = Vec::new();
    collect_bases(collected, entry, &mut seen, &mut bases);

    let mut rules = Vec::with_capacity(bases.len() + 1);
    let mut classes = Vec::new();
    for (base_name, base) in &bases {
        let json = collected.resolve_json(&base.json, keyframes_names);
        let inner = inner_json(&json);
        if !inner.is_empty() {
            rules.push(inner.to_string());
        }
        classes.extend(base.classes.iter().map(String::as_str));
        classes.extend(referenced_classes.get(base_name).copied());
    }
    classes.extend(entry.classes.iter().map(String::as_str));
    classes.extend(referenced_classes.get(name).copied());

    let own = collected.resolve_json(&entry.json, keyframes_names);
    let css = if bases.is_empty() {
        format!("css({own})")
    } else {
        let inner = inner_json(&own);
        if !inner.is_empty() {
            rules.push(inner.to_string());
        }
        format!("css({{{}}})", rules.join(","))
    };
    if classes.is_empty() {
        css
    } else {
        format!("{css} + \" {}\"", classes.join(" "))
    }
}

fn sorted(entries: &FxHashMap<String, StyleEntry>) -> Vec<(&String, &StyleEntry)> {
    let mut entries: Vec<_> = entries.iter().collect();
    entries.sort_by_key(|(name, _)| *name);
    entries
}

const fn export_prefix(exported: bool) -> &'static str {
    if exported { "export " } else { "" }
}

/// Convert collected styles to code, with keyframes references replaced by
/// `keyframes_names` (`variable_name` -> generated name)
pub fn collected_styles_to_code_with_keyframes(
    collected: &CollectedStyles,
    package: &str,
    keyframes_names: &FxHashMap<String, String>,
) -> String {
    let mut code = Vec::new();

    let mut imports = Vec::new();
    if !collected.styles.is_empty() {
        imports.push("css");
    }
    if !(collected.global_styles.is_empty()
        && collected.font_faces.is_empty()
        && collected.layers.is_empty()
        && collected.property_rules.is_empty())
    {
        imports.push("globalCss");
    }
    if !collected.keyframes.is_empty() {
        imports.push("keyframes");
    }
    if !imports.is_empty() {
        code.push(format!(
            "import {{ {} }} from '{package}'",
            imports.join(", ")
        ));
    }

    // Declaring every layer up front fixes their order the way the calls did.
    let mut layers: Vec<&str> = Vec::new();
    for layer in &collected.layers {
        if !layers.contains(&layer.as_str()) {
            layers.push(layer);
        }
    }
    if !layers.is_empty() {
        code.push(format!("globalCss`@layer {};`", layers.join(",")));
    }
    for rule in &collected.property_rules {
        code.push(format!("globalCss`{rule}`"));
    }

    for (name, entry) in sorted(&collected.keyframes) {
        code.push(format!(
            "{}const {name} = keyframes({})",
            export_prefix(entry.exported),
            collected.resolve_json(&entry.json, keyframes_names)
        ));
    }

    let referenced_classes = collected.referenced_classes();
    for (name, entry) in sorted(&collected.styles) {
        code.push(format!(
            "{}const {name} = {}",
            export_prefix(entry.exported),
            composed_css(collected, keyframes_names, &referenced_classes, name, entry)
        ));
    }

    for (selector, json) in &collected.global_styles {
        let selector = collected.resolve(selector, keyframes_names, |_| true);
        code.push(format!(
            "globalCss({{ {}: {} }})",
            json_string(&selector),
            collected.resolve_json(json, keyframes_names)
        ));
    }
    if !collected.font_faces.is_empty() {
        code.push(format!(
            "globalCss({{ fontFaces: [{}] }})",
            collected.font_faces.join(", ")
        ));
    }

    for (name, value) in &collected.constant_exports {
        code.push(format!("export const {name} = {value}"));
    }
    code.join("\n")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use css::file_map::reset_file_map;
    use serial_test::serial;

    const PACKAGE: &str = "@devup-ui/react";

    fn generate_with(code: &str, keyframes_names: &[(&str, &str)]) -> String {
        reset_file_map();
        let collected = execute_vanilla_extract(
            &format!("import {{ style, globalStyle, styleVariants, keyframes, fontFace, globalFontFace, createVar, fallbackVar, createContainer, layer, globalLayer, createTheme, createGlobalTheme, createThemeContract, createGlobalThemeContract, assignVars }} from '{PACKAGE}'\n{code}"),
            PACKAGE,
            "test.css.ts",
        )
        .unwrap();
        let keyframes_names = keyframes_names
            .iter()
            .map(|(name, generated)| (name.to_string(), generated.to_string()))
            .collect();
        collected_styles_to_code_with_keyframes(&collected, PACKAGE, &keyframes_names)
    }

    fn generate(code: &str) -> String {
        generate_with(code, &[])
    }

    #[test]
    #[serial]
    fn test_names_follow_the_values_variables_hold() {
        assert_eq!(
            generate_with(
                "export const sizes = { sm: style({ padding: 1 }), lg: style({ padding: 2 }) }
export const box = style({ color: 'red' })
const spin = keyframes({ to: { opacity: 1 } })
export const alias = box
export const animated = style({ animationName: spin })",
                &[("spin", "k")]
            ),
            r#"import { css, keyframes } from '@devup-ui/react'
const spin = keyframes({"to":{"opacity":1}})
const _ve0 = css({"padding":"1px"})
const _ve1 = css({"padding":"2px"})
export const animated = css({"animationName":"k"})
export const box = css({"color":"red"})
export const sizes = { "sm": _ve0, "lg": _ve1 }
export const alias = box"#
        );
        assert_eq!(
            generate("const _ve0 = 1\nexport const list = [style({ color: 'red' })]"),
            r#"import { css } from '@devup-ui/react'
const _ve0_ = css({"color":"red"})
export const list = [_ve0_]"#
        );
        assert_eq!(
            generate("export const animated = style({ animationName: keyframes({}) })"),
            r#"import { css, keyframes } from '@devup-ui/react'
const _ve0 = keyframes({})
export const animated = css({"animationName":"__style_0__"})"#
        );
    }

    #[test]
    #[serial]
    fn test_exported_values_become_code() {
        assert_eq!(
            generate(
                "const box = style({ color: 'red' })
export const text = 'plain \"quoted\"'
export const template = `${box} extra \\`tick\\` \\${x} back\\\\slash __open`
export const nothing = undefined
export const empty = null
export const flag = true
export const count = 1.5
export const list = [box, 2]
export const nested = { a: { b: [box] } }
export const helper = (x) => x
export const fns = [() => 1]
export const date = new Date(0)
export const cyclic = (() => { const o = {}; o.self = o; return o })()
export const bare = Object.create(null)
export const { destructured } = { destructured: () => 1 }
export declare const typeOnly: string
export function makeBox() { return box }
export type Box = string
export const symbol = Symbol('x')"
            ),
            r#"import { css } from '@devup-ui/react'
const box = css({"color":"red"})
export const text = "plain \"quoted\""
export const template = `${box} extra \`tick\` \${x} back\\slash __open`
export const nothing = undefined
export const empty = null
export const flag = true
export const count = 1.5
export const list = [box, 2]
export const nested = { "a": { "b": [box] } }
export const helper = (x) => x
export const fns = [() => 1]
export const date = new Date(0)
export const cyclic = (() => { const o = {}; o.self = o; return o })()
export const bare = {}
export const symbol = Symbol('x')"#
        );
    }

    #[test]
    #[serial]
    fn test_style_variants_are_styles() {
        assert_eq!(
            generate(
                "export const tone = styleVariants({ primary: { color: 'red' }, 0: { color: 'blue' }, [Symbol('s')]: { color: 'green' } })
export const space = styleVariants({ sm: 4 }, (padding, key) => ({ padding, content: key }))
export const none = styleVariants(undefined)
export const combined = style([tone.primary, 'external', [{ margin: 1 }]])
globalStyle(`${tone.primary} > span`, { fontWeight: 700 })"
            ),
            r#"import { css, globalCss } from '@devup-ui/react'
const _ve0 = css({"color":"blue"})
const _ve1 = css({"color":"red"}) + " f0__ve1"
const _ve2 = css({"padding":"4px","content":"sm"})
export const combined = css({"color":"red","margin":"1px"}) + " f0__ve1 external"
globalCss({ ".f0__ve1 > span": {"fontWeight":700} })
export const tone = { "0": _ve0, "primary": _ve1 }
export const space = { "sm": _ve2 }
export const none = {}"#
        );
    }

    #[test]
    #[serial]
    fn test_composition_is_transitive() {
        assert_eq!(
            generate(
                "const a = style({ color: 'red' })
const b = style([a, { margin: 2 }])
export const c = style([b, { padding: 3 }])
const e = style({})
export const f = style([e, a, '__style_99__'])
export const g = style(5)
const o = {}; o.self = o
export const h = style(o)
export const hover = style({ selectors: { [`${a}:hover &`]: { color: 'blue' } } })"
            ),
            r#"import { css } from '@devup-ui/react'
const a = css({"color":"red"}) + " f0_a"
const b = css({"color":"red","margin":"2px"}) + " f0_a"
export const c = css({"color":"red","margin":"2px","padding":"3px"}) + " f0_a"
const e = css({})
export const f = css({"color":"red"}) + " f0_a"
export const g = css({})
export const h = css({})
export const hover = css({"selectors":{".f0_a:hover &":{"color":"blue"}}})"#
        );
    }

    #[test]
    #[serial]
    fn test_font_faces() {
        assert_eq!(
            generate(
                "export const body = fontFace({ src: 'local(a)' }, 'Body Font')
export const icons = fontFace([{ src: 'local(b)' }, { src: 'local(c)', fontWeight: 700 }])
fontFace({})
const o = {}; o.self = o
fontFace(o)
globalFontFace('Inter', { src: 'local(Inter)' })"
            ),
            r#"import { globalCss } from '@devup-ui/react'
globalCss({ fontFaces: [{"src":"local(a)","fontFamily":"Body_Font-0-0"}, {"src":"local(b)","fontFamily":"font-0-1"}, {"src":"local(c)","fontWeight":700,"fontFamily":"font-0-1"}, {"fontFamily":"font-0-2"}, {"fontFamily":"font-0-3"}, {"src":"local(Inter)","fontFamily":"Inter"}] })
export const body = "Body_Font-0-0"
export const icons = "font-0-1""#
        );
    }

    #[test]
    #[serial]
    fn test_vars_follow_vanilla_extract() {
        assert_eq!(
            generate(
                "export const plain = createVar()
export const named = createVar('9 lives')
export const typed = createVar({ syntax: '<length>', inherits: false, initialValue: '0px' }, 'size')
export const union = createVar({ syntax: ['<length>', '<percentage>'], inherits: true })
export const empty = createVar('')
export const fallback = fallbackVar(plain, named, 'red')
export const invalid = fallbackVar('red', 'blue')
export const none = fallbackVar()
export const box = style({ vars: { [plain]: '1px', '--raw': 2 }, width: plain, padding: 0, zIndex: 2, margin: [1, 2] })"
            ),
            r#"import { css, globalCss } from '@devup-ui/react'
globalCss`@property --size-0-2{syntax:"<length>";inherits:false;initial-value:0px}`
globalCss`@property --var-0-3{syntax:"<length> | <percentage>";inherits:true}`
export const box = css({"vars":{"--var-0-0":"1px","--raw":2},"width":"var(--var-0-0)","padding":0,"zIndex":2,"margin":[1,2]})
export const plain = "var(--var-0-0)"
export const named = "var(--_9_lives-0-1)"
export const typed = "var(--size-0-2)"
export const union = "var(--var-0-3)"
export const empty = "var(--var-0-4)"
export const fallback = "var(--var-0-0, var(--_9_lives-0-1, red))"
export const invalid = "red"
export const none = """#
        );
    }

    #[test]
    #[serial]
    fn test_layers_and_containers() {
        assert_eq!(
            generate(
                "export const reset = layer()
export const base = layer('base')
export const child = layer({ parent: base }, 'child')
export const orphan = layer({})
export const named = globalLayer('utilities')
export const nested = globalLayer({ parent: named }, 'x')
export const again = globalLayer('utilities')
export const container = createContainer('sidebar')
export const anonymous = createContainer()"
            ),
            r#"import { globalCss } from '@devup-ui/react'
globalCss`@layer layer-0-0,base-0-1,base-0-1.child-0-2,layer-0-3,utilities,utilities.x;`
export const reset = "layer-0-0"
export const base = "base-0-1"
export const child = "base-0-1.child-0-2"
export const orphan = "layer-0-3"
export const named = "utilities"
export const nested = "utilities.x"
export const again = "utilities"
export const container = "sidebar-0-4"
export const anonymous = "container-0-5""#
        );
    }

    #[test]
    #[serial]
    fn test_themes_and_contracts() {
        assert_eq!(
            generate(
                "const contract = createThemeContract({ color: { brand: null, text: null }, list: [1], flag: true })
export const [light, lightVars] = createTheme({ '@layer': 'theme', color: { brand: 'blue' }, size: 4 })
export const dark = createTheme(contract, { color: { brand: 'black', text: 'white' } }, 'dark')
export const partial = createTheme(contract, { color: { brand: 'x', missing: 'y' } })
export const emptyTheme = createTheme(contract, {})
export const globalVars = createGlobalTheme(':root', { space: '1px' })
createGlobalTheme('.scope', contract, { '@layer': 'scoped', color: { text: 'grey' } })
export const exact = createGlobalThemeContract({ color: 'brand-color', raw: '--raw' })
export const mapped = createGlobalThemeContract({ color: { brand: null } }, (_value, path) => `x-${path.join('-')}`)
export const assigned = assignVars(contract, { color: { brand: 'red' } })
export const unassigned = assignVars(undefined, { a: 'b' })
export const box = style({ vars: assigned })
export const notObject = createTheme('tokens')"
            ),
            r#"import { css, globalCss } from '@devup-ui/react'
export const box = css({"vars":{"--color-brand-0-0":"red"}})
globalCss({ ".theme-0-2": {"@layer":"theme","--color-brand-0-3":"blue","--size-0-4":"4"} })
globalCss({ ".dark-0-5": {"--color-brand-0-0":"black","--color-text-0-1":"white"} })
globalCss({ ".theme-0-6": {"--color-brand-0-0":"x"} })
globalCss({ ":root": {"--space-0-8":"1px"} })
globalCss({ ".scope": {"@layer":"scoped","--color-text-0-1":"grey"} })
export const light = "theme-0-2"
export const lightVars = { "color": { "brand": "var(--color-brand-0-3)" }, "size": "var(--size-0-4)" }
export const dark = "dark-0-5"
export const partial = "theme-0-6"
export const emptyTheme = "theme-0-7"
export const globalVars = { "space": "var(--space-0-8)" }
export const exact = { "color": "var(--brand-color)", "raw": "var(--raw)" }
export const mapped = { "color": { "brand": "var(--x-color-brand)" } }
export const assigned = { "var(--color-brand-0-0)": "red" }
export const unassigned = {}
export const notObject = ["theme-0-9", {}]"#
        );
    }

    #[test]
    #[serial]
    fn test_nothing_collected() {
        assert_eq!(generate("const x = 1"), "");
        reset_file_map();
        assert!(execute_vanilla_extract("throw new Error('x')", PACKAGE, "test.css.ts").is_err());
        assert!(
            execute_vanilla_extract(
                "import { createVar } from '@devup-ui/react'\ncreateVar({ syntax: Symbol() })",
                PACKAGE,
                "test.css.ts"
            )
            .is_err()
        );
    }

    #[test]
    fn test_identifier() {
        let mut collector = Collector {
            file_num: 3,
            ..Collector::default()
        };
        assert_eq!(collector.identifier(Some("a b".into()), "x"), "a_b-3-0");
        assert_eq!(collector.identifier(Some("-y".into()), "x"), "_-y-3-1");
        assert_eq!(collector.identifier(Some(String::new()), "x"), "x-3-2");
        assert_eq!(collector.identifier(None, "_z"), "_z-3-3");
    }

    #[test]
    fn test_preprocess_typescript_binds_imports() {
        assert_eq!(
            preprocess_typescript(
                "import { style as s, globalStyle } from '@devup-ui/react'
import * as ve from '@devup-ui/react'
import '@devup-ui/react'
import { other } from 'other'
interface Props { color: string }
export const a: number = s(globalStyle, ve, other)
export function f() {}
export default a",
                PACKAGE
            ),
            "const { style: s, globalStyle } = __vanilla_extract__;
const ve = __vanilla_extract__;
import \"@devup-ui/react\";
import { other } from \"other\";
const a = s(globalStyle, ve, other);
function f() {}
export default a;"
        );
        assert_eq!(
            preprocess_typescript(
                "import { style } from '@devup-ui/react'\nconst a = 1",
                PACKAGE
            ),
            "const a = 1;\n"
        );
    }

    #[test]
    fn test_style_to_json_adds_vanilla_extract_units() {
        let mut context = Context::default();
        let rule = context
            .eval(Source::from_bytes(
                "({ fontSize: 16, top: 0, lineHeight: 1.5, '--gap': 4, 'var(--x)': 2, fallback: [1, 2], ':hover': { width: 3 }, vars: { 'var(--a)': 1, '--b': 2 } })",
            ))
            .unwrap();
        assert_eq!(
            style_to_json(&rule, &mut context),
            r#"{"fontSize":"16px","top":0,"lineHeight":1.5,"--gap":4,"var(--x)":2,"fallback":[1,2],":hover":{"width":"3px"},"vars":{"--a":1,"--b":2}}"#
        );
        assert_eq!(style_to_json(&JsValue::undefined(), &mut context), "{}");
    }

    #[test]
    #[serial]
    fn test_referenced_keyframes() {
        reset_file_map();
        let collected = execute_vanilla_extract(
            "import { style, keyframes } from '@devup-ui/react'
const fade = keyframes({ from: { opacity: 0 } })
const unused = keyframes({ to: { opacity: 1 } })
export const box = style({ animationName: fade })",
            PACKAGE,
            "test.css.ts",
        )
        .unwrap();
        let names = referenced_keyframes(&collected);
        assert_eq!(names, FxHashSet::from_iter(["fade".to_string()]));
        assert_eq!(
            keyframes_to_code(&collected, PACKAGE, &names),
            r#"import { keyframes } from '@devup-ui/react'
const fade = keyframes({"from":{"opacity":0}})"#
        );
    }

    #[test]
    fn test_inner_json() {
        assert_eq!(inner_json(r#" {"a":{"b":1}} "#), r#""a":{"b":1}"#);
        assert_eq!(inner_json("[1]"), "");
    }
}
