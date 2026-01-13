//! Vanilla-extract style file (.css.ts, .css.js) processor
//!
//! This module uses boa_engine to execute vanilla-extract style files
//! and extract style definitions for processing by the existing extract logic.

#![allow(dead_code)] // Public API fields/functions for future expansion

use boa_engine::{
    Context, JsArgs, JsValue, NativeFunction, Source, js_string, object::ObjectInitializer,
    property::Attribute,
};
use css::file_map::get_file_num_by_filename;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};
use oxc_transformer::{TransformOptions, Transformer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

/// A single variant in styleVariants
#[derive(Debug, Clone)]
pub struct StyleVariant {
    /// Base class reference (variable name), if composing
    pub base: Option<String>,
    /// The style object JSON for this variant
    pub styles_json: String,
}

/// Style with export info
#[derive(Debug, Clone)]
pub struct StyleEntry {
    /// The style object JSON
    pub json: String,
    /// Whether this style is exported
    pub exported: bool,
    /// Base class references for composition (placeholder IDs like "__style_0__")
    pub bases: Vec<String>,
}

/// Entry for createGlobalTheme() - CSS variables scoped to a selector
#[derive(Debug, Clone, Default)]
pub struct GlobalThemeEntry {
    /// CSS selector (e.g., ":root")
    pub selector: String,
    /// CSS variables: Vec<(var_name, value)> e.g. [("--color-brand-0-0", "blue")]
    pub css_vars: Vec<(String, String)>,
    /// Serialized JS object with var() references
    pub vars_object_json: String,
    /// Whether this is exported
    pub exported: bool,
}

/// Entry for createTheme() - CSS variables scoped to a generated class
#[derive(Debug, Clone, Default)]
pub struct ThemeEntry {
    /// CSS variables: Vec<(var_name, value)> e.g. [("--color-brand", "blue")]
    pub css_vars: Vec<(String, String)>,
    /// Whether this is exported
    pub exported: bool,
    /// For single-arg createTheme: the vars object JSON with var() references
    /// Used to generate the second element of the returned array
    pub vars_object_json: Option<String>,
    /// For single-arg createTheme: the name of the vars variable from [themeClass, vars]
    pub vars_name: Option<String>,
    /// The unique generated class name (file_prefix + variable_name)
    pub class_name: String,
}

/// Collected style definitions from vanilla-extract API calls
#[derive(Debug, Default)]
pub struct CollectedStyles {
    /// style() calls: variable_name -> (json, exported)
    pub styles: HashMap<String, StyleEntry>,
    /// globalStyle() calls: selector -> style object JSON
    pub global_styles: Vec<(String, String)>,
    /// keyframes() calls: variable_name -> (json, exported)
    pub keyframes: HashMap<String, StyleEntry>,
    /// createVar() calls: variable_name -> (CSS variable string, exported)
    pub vars: HashMap<String, (String, bool)>,
    /// fontFace() calls: placeholder_id -> (font_face JSON, font-family name, exported)
    pub font_faces: HashMap<String, (String, String, bool)>,
    /// styleVariants() calls: variable_name -> (variants, exported)
    pub style_variants: HashMap<String, (HashMap<String, StyleVariant>, bool)>,
    /// createContainer() calls: variable_name -> (container name string, exported)
    pub containers: HashMap<String, (String, bool)>,
    /// layer() calls: variable_name -> (layer name string, exported)
    pub layers: HashMap<String, (String, bool)>,
    /// createGlobalTheme() calls: variable_name -> GlobalThemeEntry
    pub global_themes: HashMap<String, GlobalThemeEntry>,
    /// createTheme() calls: variable_name -> ThemeEntry
    pub themes: HashMap<String, ThemeEntry>,
    /// Theme vars from array destructuring: vars_name -> (vars_object_json, exported)
    pub theme_vars: HashMap<String, (String, bool)>,
    /// Non-style constant exports: variable_name -> value (as code string)
    pub constant_exports: HashMap<String, String>,
}

/// Check if a filename is a vanilla-extract style file
pub fn is_vanilla_extract_file(filename: &str) -> bool {
    filename.ends_with(".css.ts") || filename.ends_with(".css.js")
}

/// Internal state for collecting styles during JS execution
#[derive(Default)]
struct StyleCollectorInner {
    styles: CollectedStyles,
    style_counter: usize,
    font_counter: usize,
    global_theme_counter: usize,
}

type StyleCollector = Rc<RefCell<StyleCollectorInner>>;

fn next_style_id(collector: &StyleCollector) -> String {
    let mut inner = collector.borrow_mut();
    let id = format!("__style_{}__", inner.style_counter);
    inner.style_counter += 1;
    id
}

fn next_font_id(collector: &StyleCollector) -> String {
    let mut inner = collector.borrow_mut();
    let id = format!("__font_{}__", inner.font_counter);
    inner.font_counter += 1;
    id
}

fn next_global_theme_id(collector: &StyleCollector) -> String {
    let mut inner = collector.borrow_mut();
    let id = format!("__global_theme_{}__", inner.global_theme_counter);
    inner.global_theme_counter += 1;
    id
}

/// Parse font-face JSON and return list of (key, value) pairs
/// Input: {"src":"local(...)","fontWeight":400}
/// Output: vec![("src", "\"local(...)\""), ("fontWeight", "400")]
fn parse_font_face_json(json: &str) -> Vec<(String, String)> {
    // Use serde_json to parse properly
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };

    let Some(obj) = value.as_object() else {
        return Vec::new();
    };

    obj.iter()
        .map(|(k, v)| {
            let val = match v {
                serde_json::Value::String(s) => {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                }
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => v.to_string(),
            };
            (k.clone(), val)
        })
        .collect()
}

/// Recursively transform theme contract object to CSS var() references
/// Returns a new JS object with null leaves replaced by var(--path)
fn transform_contract_to_vars(value: &JsValue, ctx: &mut Context, path: &[String]) -> JsValue {
    if let Some(obj) = value.as_object() {
        // Check if it's an array
        if obj.is_array() {
            return value.clone();
        }

        // It's an object - recursively transform each property
        let new_obj = boa_engine::object::ObjectInitializer::new(ctx).build();

        if let Ok(keys) = obj.own_property_keys(ctx) {
            for key in keys {
                let key_string = match &key {
                    boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
                    boa_engine::property::PropertyKey::Symbol(_) => continue,
                    boa_engine::property::PropertyKey::Index(i) => i.get().to_string(),
                };

                if let Ok(prop_value) = obj.get(js_string!(key_string.as_str()), ctx) {
                    let mut new_path = path.to_vec();
                    new_path.push(key_string.clone());

                    let transformed = transform_contract_to_vars(&prop_value, ctx, &new_path);

                    let _ = new_obj.set(js_string!(key_string.as_str()), transformed, false, ctx);
                }
            }
        }

        JsValue::from(new_obj)
    } else {
        // Leaf value (should be null) - create CSS variable reference
        let var_name = format!("--{}", path.join("-"));
        JsValue::from(js_string!(format!("var({})", var_name)))
    }
}

/// Extract CSS variable assignments by matching contract vars with values
fn extract_theme_vars(
    contract: &JsValue,
    values: &JsValue,
    ctx: &mut Context,
    css_vars: &mut Vec<(String, String)>,
    path: &[String],
) {
    if let (Some(contract_obj), Some(values_obj)) = (contract.as_object(), values.as_object()) {
        // Both are objects - recurse into properties
        if let Ok(keys) = contract_obj.own_property_keys(ctx) {
            for key in keys {
                let key_string = match &key {
                    boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
                    boa_engine::property::PropertyKey::Symbol(_) => continue,
                    boa_engine::property::PropertyKey::Index(i) => i.get().to_string(),
                };

                if let (Ok(contract_prop), Ok(value_prop)) = (
                    contract_obj.get(js_string!(key_string.as_str()), ctx),
                    values_obj.get(js_string!(key_string.as_str()), ctx),
                ) {
                    let mut new_path = path.to_vec();
                    new_path.push(key_string);

                    extract_theme_vars(&contract_prop, &value_prop, ctx, css_vars, &new_path);
                }
            }
        }
    } else if let Some(contract_str) = contract.as_string() {
        // Contract leaf is a var(--name) string
        let contract_str = contract_str.to_std_string_escaped();
        if contract_str.starts_with("var(") && contract_str.ends_with(')') {
            // Extract var name from var(--name)
            let var_name = &contract_str[4..contract_str.len() - 1];

            // Get the value
            let value_str = if let Some(s) = values.as_string() {
                s.to_std_string_escaped()
            } else if let Ok(s) = values.to_string(ctx) {
                s.to_std_string_escaped()
            } else {
                String::new()
            };

            css_vars.push((var_name.to_string(), value_str));
        }
    }
}

/// Recursively transform theme object to CSS var() references
/// Returns a new JS object with the same structure but leaf values replaced with var(--path)
fn transform_theme_to_vars(
    value: &JsValue,
    ctx: &mut Context,
    placeholder_id: &str,
    css_vars: &mut Vec<(String, String)>,
    var_counter: &mut usize,
    path: &[String],
) -> JsValue {
    if let Some(obj) = value.as_object() {
        // Check if it's an array (shouldn't happen in theme objects, but handle it)
        if obj.is_array() {
            return value.clone();
        }

        // It's an object - recursively transform each property
        let new_obj = boa_engine::object::ObjectInitializer::new(ctx).build();

        // Get own property keys
        if let Ok(keys) = obj.own_property_keys(ctx) {
            for key in keys {
                // Convert PropertyKey to string
                let key_string = match &key {
                    boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
                    boa_engine::property::PropertyKey::Symbol(_) => continue,
                    boa_engine::property::PropertyKey::Index(i) => i.get().to_string(),
                };

                if let Ok(prop_value) = obj.get(js_string!(key_string.as_str()), ctx) {
                    let mut new_path = path.to_vec();
                    new_path.push(key_string.clone());

                    let transformed = transform_theme_to_vars(
                        &prop_value,
                        ctx,
                        placeholder_id,
                        css_vars,
                        var_counter,
                        &new_path,
                    );

                    let _ = new_obj.set(js_string!(key_string.as_str()), transformed, false, ctx);
                }
            }
        }

        JsValue::from(new_obj)
    } else {
        // Leaf value - create CSS variable
        let var_name = format!(
            "--{}-{}-{}",
            path.join("-"),
            placeholder_id.trim_matches('_').replace("__", "-"),
            var_counter
        );
        *var_counter += 1;

        // Get the actual value as string
        let value_str = if let Some(s) = value.as_string() {
            s.to_std_string_escaped()
        } else if let Ok(s) = value.to_string(ctx) {
            s.to_std_string_escaped()
        } else {
            String::new()
        };

        css_vars.push((var_name.clone(), value_str));

        // Return var(--name)
        JsValue::from(js_string!(format!("var({})", var_name)))
    }
}

/// Convert JsValue to JSON string using JSON.stringify
fn js_value_to_json(value: &JsValue, context: &mut Context) -> String {
    // Use JSON.stringify to convert the value
    let json_obj = context.intrinsics().objects().json();
    let stringify_result = json_obj.get(js_string!("stringify"), context);

    if let Ok(stringify) = stringify_result
        && let Some(callable) = stringify.as_callable()
        && let Ok(result) =
            callable.call(&JsValue::undefined(), std::slice::from_ref(value), context)
        && let Some(s) = result.as_string()
    {
        return s.to_std_string_escaped();
    }

    // Fallback: simple conversion using boa_engine 0.21 API
    match value.get_type() {
        boa_engine::value::Type::String => {
            format!(
                "\"{}\"",
                value
                    .to_string(context)
                    .unwrap_or_default()
                    .to_std_string_escaped()
            )
        }
        boa_engine::value::Type::Boolean => value.to_boolean().to_string(),
        boa_engine::value::Type::Null => "null".to_string(),
        boa_engine::value::Type::Undefined => "undefined".to_string(),
        boa_engine::value::Type::Number => {
            if let Ok(n) = value.to_number(context) {
                n.to_string()
            } else {
                "NaN".to_string()
            }
        }
        _ => "{}".to_string(),
    }
}

/// Execute vanilla-extract style file and collect styles
pub fn execute_vanilla_extract(
    code: &str,
    package: &str,
    filename: &str,
) -> Result<CollectedStyles, String> {
    let collector: StyleCollector = Rc::new(RefCell::new(StyleCollectorInner::default()));
    let file_num = get_file_num_by_filename(filename);

    let mut context = Context::default();

    // Create the mock module object
    register_vanilla_extract_apis(&mut context, collector.clone(), package, file_num)?;

    // Preprocess code: convert TypeScript to JavaScript using Oxc Transformer
    let js_code = preprocess_typescript(code, package);

    // Extract variable names from the original code before execution
    let var_names = extract_var_names(code, package);

    // Execute the code
    context
        .eval(Source::from_bytes(js_code.as_bytes()))
        .map_err(|e| format!("JS execution error: {}", e))?;

    // Map placeholder IDs back to original variable names
    let mut result = std::mem::take(&mut collector.borrow_mut().styles);
    remap_style_names(&mut result, &var_names, &mut context, file_num);

    Ok(result)
}

/// Info about a variable declaration
#[derive(Debug, Clone)]
enum VarInfo {
    /// A style API call (style, keyframes, createContainer, etc.)
    StyleApi { exported: bool },
    /// A regular constant export with its original code
    Constant(String),
    /// The vars object from createTheme array destructuring [themeClass, vars]
    ThemeVars,
}

/// Extract all variable names and their info from the original code
/// Returns (style_api_vars, exported_constants)
fn extract_var_names(code: &str, _package: &str) -> Vec<(String, VarInfo)> {
    let allocator = Allocator::default();
    let source_type = SourceType::ts();
    let ret = Parser::new(&allocator, code, source_type).parse();

    let mut vars = Vec::new();

    for stmt in &ret.program.body {
        match stmt {
            // Exported variable declarations
            oxc_ast::ast::Statement::ExportNamedDeclaration(export) => {
                if let Some(oxc_ast::ast::Declaration::VariableDeclaration(var_decl)) =
                    &export.declaration
                {
                    for decl in &var_decl.declarations {
                        if let Some(init) = &decl.init {
                            // Check for array destructuring: const [themeClass, vars] = createTheme(...)
                            if let oxc_ast::ast::BindingPattern::ArrayPattern(array_pat) = &decl.id
                            {
                                if is_style_api_call(init) {
                                    // First element is the theme class
                                    if let Some(Some(first)) = array_pat.elements.first()
                                        && let oxc_ast::ast::BindingPattern::BindingIdentifier(id) =
                                            first
                                    {
                                        vars.push((
                                            id.name.to_string(),
                                            VarInfo::StyleApi { exported: true },
                                        ));
                                    }
                                    // Second element is the vars object - mark as ThemeVars
                                    if let Some(Some(second)) = array_pat.elements.get(1)
                                        && let oxc_ast::ast::BindingPattern::BindingIdentifier(id) =
                                            second
                                    {
                                        vars.push((id.name.to_string(), VarInfo::ThemeVars));
                                    }
                                }
                            } else if let Some(name) = decl.id.get_identifier_name() {
                                if is_style_api_call(init) {
                                    vars.push((
                                        name.to_string(),
                                        VarInfo::StyleApi { exported: true },
                                    ));
                                } else {
                                    // Extract the original init expression using span
                                    let span = init.span();
                                    let init_code = &code[span.start as usize..span.end as usize];
                                    vars.push((
                                        name.to_string(),
                                        VarInfo::Constant(init_code.to_string()),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            // Non-exported variable declarations
            oxc_ast::ast::Statement::VariableDeclaration(var_decl) => {
                for decl in &var_decl.declarations {
                    if let Some(init) = &decl.init {
                        // Check for array destructuring
                        if let oxc_ast::ast::BindingPattern::ArrayPattern(array_pat) = &decl.id {
                            if is_style_api_call(init) {
                                if let Some(Some(first)) = array_pat.elements.first()
                                    && let oxc_ast::ast::BindingPattern::BindingIdentifier(id) =
                                        first
                                {
                                    vars.push((
                                        id.name.to_string(),
                                        VarInfo::StyleApi { exported: false },
                                    ));
                                }
                                if let Some(Some(second)) = array_pat.elements.get(1)
                                    && let oxc_ast::ast::BindingPattern::BindingIdentifier(id) =
                                        second
                                {
                                    vars.push((id.name.to_string(), VarInfo::ThemeVars));
                                }
                            }
                        } else if let Some(name) = decl.id.get_identifier_name()
                            && is_style_api_call(init)
                        {
                            vars.push((name.to_string(), VarInfo::StyleApi { exported: false }));
                        }
                    }
                    // We don't need to track non-exported non-style constants
                }
            }
            _ => {}
        }
    }

    vars
}

/// Check if an expression is a call to a style API (style, keyframes, styleVariants, etc.)
fn is_style_api_call(expr: &oxc_ast::ast::Expression) -> bool {
    if let oxc_ast::ast::Expression::CallExpression(call) = expr
        && let oxc_ast::ast::Expression::Identifier(id) = &call.callee
    {
        let name = id.name.as_str();
        return matches!(
            name,
            "style"
                | "keyframes"
                | "styleVariants"
                | "fontFace"
                | "createVar"
                | "createContainer"
                | "layer"
                | "createGlobalTheme"
                | "createTheme"
        );
    }
    false
}

/// Remap style placeholder IDs to original variable names
fn remap_style_names(
    collected: &mut CollectedStyles,
    vars: &[(String, VarInfo)],
    _context: &mut Context,
    file_num: usize,
) {
    // Generate a file-based prefix for unique class names
    // e.g., file_num 0 -> "f0"
    let file_prefix = format!("f{}", file_num);
    // Build mapping from placeholder ID to original name
    // The order of style() calls matches the order of variable declarations
    let mut placeholder_to_name: HashMap<String, String> = HashMap::new();
    let mut font_placeholder_to_name: HashMap<String, String> = HashMap::new();
    let mut new_styles = HashMap::new();
    let mut new_keyframes = HashMap::new();
    let mut new_style_variants = HashMap::new();
    let mut new_vars = HashMap::new();
    let mut new_containers = HashMap::new();
    let mut new_layers = HashMap::new();
    let mut new_font_faces = HashMap::new();
    let mut new_global_themes = HashMap::new();
    let mut new_themes = HashMap::new();
    let mut style_idx = 0;
    let mut font_idx = 0;
    let mut global_theme_idx = 0;
    // Track the last processed theme's vars_object_json for ThemeVars handling
    let mut last_theme_vars_json: Option<String> = None;

    // First pass: collect old entries preserving all fields
    let old_styles: HashMap<String, StyleEntry> = collected.styles.drain().collect();
    let old_keyframes: HashMap<String, StyleEntry> = collected.keyframes.drain().collect();
    let old_style_variants: HashMap<String, HashMap<String, StyleVariant>> = collected
        .style_variants
        .drain()
        .map(|(k, v)| (k, v.0))
        .collect();
    let old_vars: HashMap<String, String> = collected.vars.drain().map(|(k, v)| (k, v.0)).collect();
    let old_containers: HashMap<String, String> = collected
        .containers
        .drain()
        .map(|(k, v)| (k, v.0))
        .collect();
    let old_layers: HashMap<String, String> =
        collected.layers.drain().map(|(k, v)| (k, v.0)).collect();
    // font_faces: placeholder_id -> (json, font_family, exported)
    let old_font_faces: HashMap<String, (String, String)> = collected
        .font_faces
        .drain()
        .map(|(k, v)| (k, (v.0, v.1)))
        .collect();
    // global_themes: placeholder_id -> GlobalThemeEntry (without exported flag for remapping)
    let old_global_themes: HashMap<String, GlobalThemeEntry> =
        collected.global_themes.drain().collect();
    // themes: placeholder_id -> ThemeEntry (without exported flag for remapping)
    let old_themes: HashMap<String, ThemeEntry> = collected.themes.drain().collect();

    for (name, info) in vars {
        match info {
            VarInfo::StyleApi { exported } => {
                // First check if this is a fontFace (uses __font_N__ placeholder)
                let font_placeholder = format!("__font_{}__", font_idx);
                if let Some((json, font_family)) = old_font_faces.get(&font_placeholder) {
                    font_placeholder_to_name.insert(font_placeholder.clone(), name.clone());
                    new_font_faces
                        .insert(name.clone(), (json.clone(), font_family.clone(), *exported));
                    font_idx += 1;
                    continue;
                }

                // Check if this is a createGlobalTheme (uses __global_theme_N__ placeholder)
                let global_theme_placeholder = format!("__global_theme_{}__", global_theme_idx);
                if let Some(entry) = old_global_themes.get(&global_theme_placeholder) {
                    new_global_themes.insert(
                        name.clone(),
                        GlobalThemeEntry {
                            selector: entry.selector.clone(),
                            css_vars: entry.css_vars.clone(),
                            vars_object_json: entry.vars_object_json.clone(),
                            exported: *exported,
                        },
                    );
                    global_theme_idx += 1;
                    continue;
                }

                let placeholder = format!("__style_{}__", style_idx);
                placeholder_to_name.insert(placeholder.clone(), name.clone());

                if let Some(entry) = old_styles.get(&placeholder) {
                    new_styles.insert(
                        name.clone(),
                        StyleEntry {
                            json: entry.json.clone(),
                            exported: *exported,
                            bases: entry.bases.clone(),
                        },
                    );
                    style_idx += 1;
                } else if let Some(entry) = old_keyframes.get(&placeholder) {
                    new_keyframes.insert(
                        name.clone(),
                        StyleEntry {
                            json: entry.json.clone(),
                            exported: *exported,
                            bases: entry.bases.clone(),
                        },
                    );
                    style_idx += 1;
                } else if let Some(variants) = old_style_variants.get(&placeholder) {
                    new_style_variants.insert(name.clone(), (variants.clone(), *exported));
                    style_idx += 1;
                } else if let Some(value) = old_vars.get(&placeholder) {
                    new_vars.insert(name.clone(), (value.clone(), *exported));
                    style_idx += 1;
                } else if let Some(value) = old_containers.get(&placeholder) {
                    new_containers.insert(name.clone(), (value.clone(), *exported));
                    style_idx += 1;
                } else if let Some(value) = old_layers.get(&placeholder) {
                    new_layers.insert(name.clone(), (value.clone(), *exported));
                    style_idx += 1;
                } else if let Some(entry) = old_themes.get(&placeholder) {
                    // Track this theme name for the next ThemeVars entry
                    if entry.vars_object_json.is_some() {
                        last_theme_vars_json = Some(name.clone());
                    }

                    // Generate unique class name: file_prefix + variable_name
                    let class_name = format!("{}_{}", file_prefix, name);

                    // Add CSS variables to global_styles with class selector
                    if !entry.css_vars.is_empty() {
                        let vars_json = format!(
                            "{{ {} }}",
                            entry
                                .css_vars
                                .iter()
                                .map(|(var_name, value)| format!("\"{}\": \"{}\"", var_name, value))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        collected
                            .global_styles
                            .push((format!(".{}", class_name), vars_json));
                    }

                    new_themes.insert(
                        name.clone(),
                        ThemeEntry {
                            css_vars: entry.css_vars.clone(),
                            exported: *exported,
                            vars_object_json: entry.vars_object_json.clone(),
                            vars_name: None, // Will be set by ThemeVars if present
                            class_name,
                        },
                    );
                    style_idx += 1;
                }
            }
            VarInfo::ThemeVars => {
                // This is the vars object from [themeClass, vars] = createTheme(...)
                // Set vars_name on the theme we just processed
                if let Some(theme_name) = last_theme_vars_json.take()
                    && let Some(theme_entry) = new_themes.get_mut(&theme_name)
                {
                    theme_entry.vars_name = Some(name.clone());
                }
            }
            VarInfo::Constant(code) => {
                collected
                    .constant_exports
                    .insert(name.clone(), code.clone());
            }
        }
    }

    // Remap base references in styleVariants
    for (variants, _) in new_style_variants.values_mut() {
        for variant in variants.values_mut() {
            if let Some(base) = &variant.base
                && let Some(name) = placeholder_to_name.get(base)
            {
                variant.base = Some(name.clone());
            }
        }
    }

    // Remap base references in styles (for composition)
    for entry in new_styles.values_mut() {
        entry.bases = entry
            .bases
            .iter()
            .map(|b| {
                placeholder_to_name
                    .get(b)
                    .cloned()
                    .unwrap_or_else(|| b.clone())
            })
            .collect();
    }

    // Replace font placeholders in style JSONs with actual font-family names
    // Build a mapping from placeholder to font-family name
    let font_family_map: HashMap<String, String> = new_font_faces
        .iter()
        .map(|(name, (_, font_family, _))| {
            // Find the placeholder that maps to this name
            let placeholder = font_placeholder_to_name
                .iter()
                .find(|(_, n)| *n == name)
                .map(|(p, _)| p.clone())
                .unwrap_or_default();
            (placeholder, font_family.clone())
        })
        .collect();

    // Replace __font_N__ placeholders in style JSON with font-family names
    for entry in new_styles.values_mut() {
        for (placeholder, font_family) in &font_family_map {
            if entry.json.contains(placeholder) {
                entry.json = entry.json.replace(placeholder, font_family);
            }
        }
    }

    // Replace __style_N__ placeholders in style JSON with variable names
    // This is needed for selectors that reference other styles like `${parent}:hover &`
    for entry in new_styles.values_mut() {
        for (placeholder, var_name) in &placeholder_to_name {
            if entry.json.contains(placeholder) {
                entry.json = entry.json.replace(placeholder, var_name);
            }
        }
    }

    collected.styles = new_styles;
    collected.keyframes = new_keyframes;
    collected.style_variants = new_style_variants;
    collected.vars = new_vars;
    collected.containers = new_containers;
    collected.layers = new_layers;
    collected.font_faces = new_font_faces;
    collected.global_themes = new_global_themes;
    collected.themes = new_themes;
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
    let mut js_code = Codegen::new().build(&program).code;

    // Replace import from package with our mock object destructuring
    // e.g., import { style } from '@devup-ui/react' -> const { style } = __vanilla_extract__;
    let import_patterns = [
        format!("from \"{}\"", package),
        format!("from '{}'", package),
    ];

    for pattern in &import_patterns {
        if js_code.contains(pattern) {
            // Find and replace the import line
            let lines: Vec<&str> = js_code.lines().collect();
            let mut new_lines = Vec::new();

            for line in lines {
                if line.contains(pattern) {
                    // Extract imported names from: import { a, b, c } from 'package'
                    if let Some(start) = line.find('{')
                        && let Some(end) = line.find('}')
                    {
                        let imports = &line[start + 1..end];
                        new_lines.push(format!("const {{{}}} = __vanilla_extract__;", imports));
                        continue;
                    }
                }
                new_lines.push(line.to_string());
            }
            js_code = new_lines.join("\n");
            break;
        }
    }

    // Remove 'export' keyword (boa doesn't support ES modules)
    js_code = js_code.replace("export const ", "const ");
    js_code = js_code.replace("export let ", "let ");
    js_code = js_code.replace("export var ", "var ");
    js_code = js_code.replace("export function ", "function ");

    js_code
}

/// Register vanilla-extract mock APIs in the JS context
fn register_vanilla_extract_apis(
    context: &mut Context,
    collector: StyleCollector,
    _package: &str,
    file_num: usize,
) -> Result<(), String> {
    // style() function
    let collector_style = collector.clone();
    // SAFETY: The closure only captures Rc<RefCell<_>> which is safe to use in single-threaded JS context
    let style_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let style_obj = args.get_or_undefined(0);
            let id = next_style_id(&collector_style);

            // Check if argument is an array (composition syntax)
            let (json, bases) = if let Some(obj) = style_obj.as_object() {
                if let Ok(length_val) = obj.get(js_string!("length"), ctx) {
                    if let Some(len) = length_val.as_number() {
                        // It's an array - handle composition
                        let len = len as u32;
                        let mut base_classes = Vec::new();
                        let mut merged_styles = String::from("{");
                        let mut first_style = true;

                        for i in 0..len {
                            if let Ok(elem) = obj.get(i, ctx) {
                                if let Some(base_str) = elem.as_string() {
                                    // It's a base class reference (string)
                                    base_classes.push(base_str.to_std_string_escaped());
                                } else if elem.is_object() {
                                    // It's a style object - merge it
                                    let elem_json = js_value_to_json(&elem, ctx);
                                    // Strip outer braces and merge
                                    let inner = elem_json
                                        .trim()
                                        .trim_start_matches('{')
                                        .trim_end_matches('}')
                                        .trim();
                                    if !inner.is_empty() {
                                        if !first_style {
                                            merged_styles.push(',');
                                        }
                                        merged_styles.push_str(inner);
                                        first_style = false;
                                    }
                                }
                            }
                        }
                        merged_styles.push('}');
                        (merged_styles, base_classes)
                    } else {
                        // Not an array, just a style object
                        (js_value_to_json(style_obj, ctx), Vec::new())
                    }
                } else {
                    // No length property, just a style object
                    (js_value_to_json(style_obj, ctx), Vec::new())
                }
            } else {
                // Not an object at all
                (js_value_to_json(style_obj, ctx), Vec::new())
            };

            collector_style.borrow_mut().styles.styles.insert(
                id.clone(),
                StyleEntry {
                    json,
                    exported: false, // Will be updated in remap_style_names
                    bases,
                },
            );

            Ok(JsValue::from(js_string!(id)))
        })
    };

    // globalStyle() function
    let collector_global = collector.clone();
    let global_style_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let selector = args
                .get_or_undefined(0)
                .to_string(ctx)?
                .to_std_string_escaped();
            let style_obj = args.get_or_undefined(1);
            let json = js_value_to_json(style_obj, ctx);

            collector_global
                .borrow_mut()
                .styles
                .global_styles
                .push((selector, json));

            Ok(JsValue::undefined())
        })
    };

    // keyframes() function
    let collector_keyframes = collector.clone();
    let keyframes_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let keyframes_obj = args.get_or_undefined(0);
            let json = js_value_to_json(keyframes_obj, ctx);
            let id = next_style_id(&collector_keyframes);

            collector_keyframes.borrow_mut().styles.keyframes.insert(
                id.clone(),
                StyleEntry {
                    json,
                    exported: false,
                    bases: Vec::new(),
                },
            );

            Ok(JsValue::from(js_string!(id)))
        })
    };

    // createVar() function
    // Returns CSS custom property name like "--var-0", not wrapped in var()
    // When used as a key in vars: {[colorVar]: 'blue'}, it becomes the property name
    // When used as a value, the extraction logic will wrap it in var()
    let collector_var = collector.clone();
    let create_var_fn = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            let id = next_style_id(&collector_var);
            let var_name = format!("--var-{}", collector_var.borrow().styles.vars.len());
            collector_var
                .borrow_mut()
                .styles
                .vars
                .insert(id.clone(), (var_name.clone(), false));
            // Return just the CSS custom property name, without var() wrapper
            Ok(JsValue::from(js_string!(var_name)))
        })
    };

    // fontFace() function
    // Returns a placeholder ID that will be replaced with generated font-family name
    let collector_font = collector.clone();
    let font_face_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let font_obj = args.get_or_undefined(0);
            let json = js_value_to_json(font_obj, ctx);
            let id = next_font_id(&collector_font);

            // Generate a unique font-family name for this font
            // Include file_num to ensure uniqueness across different files
            let font_family = format!(
                "__devup_font_{}_{}",
                file_num,
                collector_font.borrow().font_counter - 1
            );

            collector_font
                .borrow_mut()
                .styles
                .font_faces
                .insert(id.clone(), (json, font_family.clone(), false));

            // Return the placeholder ID - will be replaced in code generation
            Ok(JsValue::from(js_string!(id)))
        })
    };

    // styleVariants() function
    let collector_variants = collector.clone();
    let style_variants_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let variants_obj = args.get_or_undefined(0);
            let variants = parse_style_variants(variants_obj, ctx);
            let id = next_style_id(&collector_variants);

            collector_variants
                .borrow_mut()
                .styles
                .style_variants
                .insert(id.clone(), (variants, false));

            // Return an object placeholder - the actual object will be built in code generation
            Ok(JsValue::from(js_string!(id)))
        })
    };

    // fallbackVar() function - returns var(--x, fallback) format
    let fallback_var_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let var_value = args
                .get_or_undefined(0)
                .to_string(ctx)?
                .to_std_string_escaped();
            let fallback = args
                .get_or_undefined(1)
                .to_string(ctx)?
                .to_std_string_escaped();

            // var_value is now just "--var-0" (CSS custom property name)
            // Return var(--var-0, fallback)
            let result = format!("var({}, {})", var_value, fallback);
            Ok(JsValue::from(js_string!(result)))
        })
    };

    // createThemeContract() function - transforms null leaves to var(--path) references
    let create_theme_contract_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let contract_obj = args.get_or_undefined(0);
            let transformed = transform_contract_to_vars(contract_obj, ctx, &[]);
            Ok(transformed)
        })
    };

    // createTheme() function - creates a class with CSS variable assignments
    // Single arg: createTheme({ color: { brand: 'blue' } }) -> returns [themeClass, vars]
    // Two args: createTheme(contract, values) -> returns themeClass
    let collector_theme = collector.clone();
    let create_theme_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let first_arg = args.get_or_undefined(0);
            let second_arg = args.get_or_undefined(1);

            // Check if it's single-arg (values only) or two-arg (contract, values)
            let is_single_arg = second_arg.is_undefined();

            let id = next_style_id(&collector_theme);

            if is_single_arg {
                // Single arg: createTheme({ color: { brand: 'blue' } })
                // Returns [themeClass, vars] where vars has var() references
                let mut css_vars = Vec::new();
                let mut var_counter = 0usize;

                // Transform values to var() references and collect CSS variables
                let vars_obj = transform_theme_to_vars(
                    first_arg,
                    ctx,
                    &id,
                    &mut css_vars,
                    &mut var_counter,
                    &[],
                );

                let vars_object_json = js_value_to_json(&vars_obj, ctx);

                // Store the theme entry with vars object
                collector_theme.borrow_mut().styles.themes.insert(
                    id.clone(),
                    ThemeEntry {
                        css_vars,
                        exported: false,
                        vars_object_json: Some(vars_object_json),
                        vars_name: None,           // Will be set during remapping
                        class_name: String::new(), // Will be set during remapping
                    },
                );

                // Return [themeId, varsObject] as an array
                let result_array = boa_engine::object::builtins::JsArray::new(ctx);
                let _ = result_array.push(JsValue::from(js_string!(id.clone())), ctx);
                let _ = result_array.push(vars_obj, ctx);

                Ok(JsValue::from(result_array))
            } else {
                // Two args: createTheme(contract, values)
                // Returns just the themeClass
                let mut css_vars = Vec::new();
                extract_theme_vars(first_arg, second_arg, ctx, &mut css_vars, &[]);

                // Store the theme entry
                collector_theme.borrow_mut().styles.themes.insert(
                    id.clone(),
                    ThemeEntry {
                        css_vars,
                        exported: false,
                        vars_object_json: None,
                        vars_name: None,
                        class_name: String::new(), // Will be set during remapping
                    },
                );

                Ok(JsValue::from(js_string!(id)))
            }
        })
    };

    // layer() function
    let collector_layer = collector.clone();
    let layer_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let id = next_style_id(&collector_layer);
            let name = args
                .get_or_undefined(0)
                .to_string(ctx)?
                .to_std_string_escaped();
            collector_layer
                .borrow_mut()
                .styles
                .layers
                .insert(id.clone(), (name.clone(), false));
            Ok(JsValue::from(js_string!(name)))
        })
    };

    // createContainer() function
    let collector_container = collector.clone();
    let create_container_fn = unsafe {
        NativeFunction::from_closure(move |_this, _args, _ctx| {
            let id = next_style_id(&collector_container);
            let container_name = format!(
                "__container_{}__",
                collector_container.borrow().styles.containers.len()
            );
            collector_container
                .borrow_mut()
                .styles
                .containers
                .insert(id.clone(), (container_name.clone(), false));
            Ok(JsValue::from(js_string!(container_name)))
        })
    };

    // createGlobalTheme() function
    let collector_global_theme = collector.clone();
    let create_global_theme_fn = unsafe {
        NativeFunction::from_closure(move |_this, args, ctx| {
            let placeholder_id = next_global_theme_id(&collector_global_theme);
            let selector = args
                .get_or_undefined(0)
                .to_string(ctx)
                .map(|s| s.to_std_string_escaped())
                .unwrap_or_else(|_| ":root".to_string());
            let theme_obj = args.get_or_undefined(1);

            // Collect CSS variables and build new object with var() references
            let mut css_vars = Vec::new();
            let mut var_counter = 0usize;
            let result_obj = transform_theme_to_vars(
                theme_obj,
                ctx,
                &placeholder_id,
                &mut css_vars,
                &mut var_counter,
                &[],
            );

            // Serialize the result object to JSON for code generation
            let vars_object_json = js_value_to_json(&result_obj, ctx);

            // Store the collected CSS variables and vars object
            collector_global_theme
                .borrow_mut()
                .styles
                .global_themes
                .insert(
                    placeholder_id,
                    GlobalThemeEntry {
                        selector,
                        css_vars,
                        vars_object_json,
                        exported: false,
                    },
                );

            Ok(result_obj)
        })
    };

    // Build the mock object
    let mut ve_builder = ObjectInitializer::new(context);
    ve_builder.function(style_fn, js_string!("style"), 1);
    ve_builder.function(global_style_fn, js_string!("globalStyle"), 2);
    ve_builder.function(keyframes_fn, js_string!("keyframes"), 1);
    ve_builder.function(create_var_fn, js_string!("createVar"), 0);
    ve_builder.function(font_face_fn, js_string!("fontFace"), 1);
    ve_builder.function(style_variants_fn, js_string!("styleVariants"), 1);
    ve_builder.function(fallback_var_fn, js_string!("fallbackVar"), 2);
    ve_builder.function(create_theme_fn, js_string!("createTheme"), 1);
    ve_builder.function(
        create_theme_contract_fn,
        js_string!("createThemeContract"),
        1,
    );
    ve_builder.function(layer_fn, js_string!("layer"), 1);
    ve_builder.function(create_container_fn, js_string!("createContainer"), 0);
    ve_builder.function(create_global_theme_fn, js_string!("createGlobalTheme"), 2);

    let ve_obj = ve_builder.build();

    // Register as global __vanilla_extract__
    context
        .register_global_property(js_string!("__vanilla_extract__"), ve_obj, Attribute::all())
        .map_err(|e| format!("Failed to register __vanilla_extract__: {}", e))?;

    Ok(())
}

/// Find all style names that are referenced in selectors of other styles
/// Returns a set of style names that need to be extracted first
pub fn find_selector_references(collected: &CollectedStyles) -> std::collections::HashSet<String> {
    let mut referenced = std::collections::HashSet::new();
    let style_names: std::collections::HashSet<&str> =
        collected.styles.keys().map(|s| s.as_str()).collect();

    for entry in collected.styles.values() {
        // Check if this style's JSON contains references to other style names
        for style_name in &style_names {
            // Look for patterns like "stylename:" or "stylename " in selectors
            // The JSON has selectors like {"selectors":{"parent:hover &":{...}}}
            if entry.json.contains(&format!("\"{}:", style_name))
                || entry.json.contains(&format!("\"{} ", style_name))
            {
                referenced.insert(style_name.to_string());
            }
        }
    }

    referenced
}

/// Generate code only for specific styles (used for first-pass extraction)
pub fn collected_styles_to_code_partial(
    collected: &CollectedStyles,
    package: &str,
    style_names: &std::collections::HashSet<String>,
) -> String {
    let mut code_parts = Vec::new();

    if !style_names.is_empty() {
        code_parts.push(format!("import {{ css }} from '{}'", package));
    }

    // Generate only the specified styles
    let mut styles: Vec<_> = collected
        .styles
        .iter()
        .filter(|(name, _)| style_names.contains(*name))
        .collect();
    styles.sort_by_key(|(name, _)| *name);

    for (name, entry) in styles {
        // Generate as non-exported for first pass
        code_parts.push(format!("const {} = css({})", name, entry.json));
    }

    code_parts.join("\n")
}

/// Convert collected styles to code with selector references replaced by class names
pub fn collected_styles_to_code_with_classes(
    collected: &CollectedStyles,
    package: &str,
    class_map: &HashMap<String, String>,
) -> String {
    let mut code_parts = Vec::new();

    // Generate import statement
    let mut imports = Vec::new();
    if !collected.styles.is_empty()
        || !collected.style_variants.is_empty()
        || !collected.themes.is_empty()
    {
        imports.push("css");
    }
    if !collected.global_styles.is_empty()
        || !collected.font_faces.is_empty()
        || !collected.global_themes.is_empty()
    {
        imports.push("globalCss");
    }
    if !collected.keyframes.is_empty() {
        imports.push("keyframes");
    }

    if !imports.is_empty() {
        code_parts.push(format!(
            "import {{ {} }} from '{}'",
            imports.join(", "),
            package
        ));
    }

    // Generate style declarations with selector references replaced
    let style_json_map: HashMap<&str, &str> = collected
        .styles
        .iter()
        .map(|(name, entry)| (name.as_str(), entry.json.as_str()))
        .collect();

    let mut styles: Vec<_> = collected.styles.iter().collect();
    styles.sort_by_key(|(name, _)| *name);

    for (name, entry) in styles {
        let prefix = if entry.exported { "export " } else { "" };

        // Replace style name references with class names in JSON
        let mut json = entry.json.clone();
        for (style_name, class_name) in class_map {
            // Replace patterns like "stylename:" with "classname:"
            json = json.replace(&format!("\"{}:", style_name), &format!("\"{}:", class_name));
            json = json.replace(&format!("\"{} ", style_name), &format!("\"{} ", class_name));
        }

        if entry.bases.is_empty() {
            code_parts.push(format!("{}const {} = css({})", prefix, name, json));
        } else {
            // Composition: merge all base styles
            let mut merged_parts = Vec::new();
            for base_name in &entry.bases {
                if let Some(base_json) = style_json_map.get(base_name.as_str()) {
                    let inner = base_json
                        .trim()
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .trim();
                    if !inner.is_empty() {
                        merged_parts.push(inner.to_string());
                    }
                }
            }
            let own_inner = json
                .trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .trim();
            if !own_inner.is_empty() {
                merged_parts.push(own_inner.to_string());
            }
            let merged_json = format!("{{{}}}", merged_parts.join(","));
            code_parts.push(format!("{}const {} = css({})", prefix, name, merged_json));
        }
    }

    // Generate createTheme exports (class name and optionally vars object)
    // Note: CSS variables are added to global_styles during remapping
    let mut themes: Vec<_> = collected.themes.iter().collect();
    themes.sort_by_key(|(name, _)| *name);
    for (name, entry) in themes {
        let prefix = if entry.exported { "export " } else { "" };
        if let Some(vars_name) = &entry.vars_name {
            if let Some(vars_json) = &entry.vars_object_json {
                code_parts.push(format!(
                    "{}const [{}, {}] = [\"{}\", {}]",
                    prefix, name, vars_name, entry.class_name, vars_json
                ));
            } else {
                code_parts.push(format!(
                    "{}const {} = \"{}\"",
                    prefix, name, entry.class_name
                ));
            }
        } else {
            code_parts.push(format!(
                "{}const {} = \"{}\"",
                prefix, name, entry.class_name
            ));
        }
    }

    // Add remaining code generation (globalCss, keyframes, etc.) - call original function's logic
    append_non_style_code(collected, package, &mut code_parts);

    code_parts.join("\n")
}

/// Append non-style code parts (globalCss, keyframes, fontFaces, etc.)
fn append_non_style_code(
    collected: &CollectedStyles,
    _package: &str,
    code_parts: &mut Vec<String>,
) {
    // Generate globalCss calls
    for (selector, json) in &collected.global_styles {
        code_parts.push(format!("globalCss({{ \"{}\": {} }})", selector, json));
    }

    // Generate @font-face rules
    let mut font_faces_sorted: Vec<_> = collected.font_faces.iter().collect();
    font_faces_sorted.sort_by_key(|(name, _)| *name);
    for (_name, (json, font_family, _exported)) in font_faces_sorted {
        let props = parse_font_face_json(json);
        let props_str = props
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        let code = if props_str.is_empty() {
            format!(
                "globalCss({{ fontFaces: [{{ fontFamily: \"{}\" }}] }})",
                font_family
            )
        } else {
            format!(
                "globalCss({{ fontFaces: [{{ fontFamily: \"{}\", {} }}] }})",
                font_family, props_str
            )
        };
        code_parts.push(code);
    }

    // Generate createGlobalTheme CSS variables
    let mut global_themes_sorted: Vec<_> = collected.global_themes.iter().collect();
    global_themes_sorted.sort_by_key(|(name, _)| *name);
    for (_name, entry) in &global_themes_sorted {
        if !entry.css_vars.is_empty() {
            let vars_str = entry
                .css_vars
                .iter()
                .map(|(var_name, value)| format!("\"{}\": \"{}\"", var_name, value))
                .collect::<Vec<_>>()
                .join(", ");
            code_parts.push(format!(
                "globalCss({{ \"{}\": {{ {} }} }})",
                entry.selector, vars_str
            ));
        }
    }

    // Generate keyframes
    let mut keyframes: Vec<_> = collected.keyframes.iter().collect();
    keyframes.sort_by_key(|(name, _)| *name);
    for (name, entry) in keyframes {
        let prefix = if entry.exported { "export " } else { "" };
        code_parts.push(format!(
            "{}const {} = keyframes({})",
            prefix, name, entry.json
        ));
    }

    // Generate styleVariants
    let style_json_map: HashMap<&str, &str> = collected
        .styles
        .iter()
        .map(|(name, entry)| (name.as_str(), entry.json.as_str()))
        .collect();
    let mut variants: Vec<_> = collected.style_variants.iter().collect();
    variants.sort_by_key(|(name, _)| *name);
    for (name, (variant_map, exported)) in variants {
        let mut variant_entries: Vec<_> = variant_map.iter().collect();
        variant_entries.sort_by_key(|(k, _)| *k);
        let mut object_parts = Vec::new();
        for (variant_key, variant) in variant_entries {
            let value = if let Some(base_name) = &variant.base {
                let mut merged_parts = Vec::new();
                if let Some(base_json) = style_json_map.get(base_name.as_str()) {
                    let inner = base_json
                        .trim()
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .trim();
                    if !inner.is_empty() {
                        merged_parts.push(inner.to_string());
                    }
                }
                let own_inner = variant
                    .styles_json
                    .trim()
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .trim();
                if !own_inner.is_empty() {
                    merged_parts.push(own_inner.to_string());
                }
                format!("css({{{}}})", merged_parts.join(","))
            } else {
                format!("css({})", variant.styles_json)
            };
            object_parts.push(format!("  {}: {}", variant_key, value));
        }
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!(
            "{}const {} = {{\n{}\n}}",
            prefix,
            name,
            object_parts.join(",\n")
        ));
    }

    // Generate createVar declarations
    let mut vars: Vec<_> = collected.vars.iter().collect();
    vars.sort_by_key(|(name, _)| *name);
    for (name, (value, exported)) in vars {
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!("{}const {} = \"{}\"", prefix, name, value));
    }

    // Generate fontFace declarations
    let mut font_faces: Vec<_> = collected.font_faces.iter().collect();
    font_faces.sort_by_key(|(name, _)| *name);
    for (name, (_, font_family, exported)) in font_faces {
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!("{}const {} = \"{}\"", prefix, name, font_family));
    }

    // Generate createContainer declarations
    let mut containers: Vec<_> = collected.containers.iter().collect();
    containers.sort_by_key(|(name, _)| *name);
    for (name, (value, exported)) in containers {
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!("{}const {} = \"{}\"", prefix, name, value));
    }

    // Generate layer declarations
    let mut layers: Vec<_> = collected.layers.iter().collect();
    layers.sort_by_key(|(name, _)| *name);
    for (name, (value, exported)) in layers {
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!("{}const {} = \"{}\"", prefix, name, value));
    }

    // Generate createGlobalTheme vars object declarations
    for (name, entry) in &global_themes_sorted {
        let prefix = if entry.exported { "export " } else { "" };
        code_parts.push(format!(
            "{}const {} = {}",
            prefix, name, entry.vars_object_json
        ));
    }

    // Generate constant exports
    let mut constants: Vec<_> = collected.constant_exports.iter().collect();
    constants.sort_by_key(|(name, _)| *name);
    for (name, value) in constants {
        code_parts.push(format!("export const {} = {}", name, value));
    }
}

/// Convert collected styles to code that can be processed by existing extract logic
pub fn collected_styles_to_code(collected: &CollectedStyles, package: &str) -> String {
    let mut code_parts = Vec::new();

    // Generate import statement
    let mut imports = Vec::new();
    if !collected.styles.is_empty()
        || !collected.style_variants.is_empty()
        || !collected.themes.is_empty()
    {
        imports.push("css");
    }
    if !collected.global_styles.is_empty()
        || !collected.font_faces.is_empty()
        || !collected.global_themes.is_empty()
    {
        imports.push("globalCss");
    }
    if !collected.keyframes.is_empty() {
        imports.push("keyframes");
    }

    if !imports.is_empty() {
        code_parts.push(format!(
            "import {{ {} }} from '{}'",
            imports.join(", "),
            package
        ));
    }

    // Generate style declarations (sorted for deterministic output)
    // First, build a map of name -> json for looking up base styles
    let style_json_map: HashMap<&str, &str> = collected
        .styles
        .iter()
        .map(|(name, entry)| (name.as_str(), entry.json.as_str()))
        .collect();

    let mut styles: Vec<_> = collected.styles.iter().collect();
    styles.sort_by_key(|(name, _)| *name);
    for (name, entry) in styles {
        let prefix = if entry.exported { "export " } else { "" };
        if entry.bases.is_empty() {
            // Simple style, no composition
            code_parts.push(format!("{}const {} = css({})", prefix, name, entry.json));
        } else {
            // Composition: merge all base styles + own styles into a single css() call
            let mut merged_parts = Vec::new();

            // Add styles from each base
            for base_name in &entry.bases {
                if let Some(base_json) = style_json_map.get(base_name.as_str()) {
                    // Strip outer braces and add to merged parts
                    let inner = base_json
                        .trim()
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .trim();
                    if !inner.is_empty() {
                        merged_parts.push(inner.to_string());
                    }
                }
            }

            // Add own styles
            let own_inner = entry
                .json
                .trim()
                .trim_start_matches('{')
                .trim_end_matches('}')
                .trim();
            if !own_inner.is_empty() {
                merged_parts.push(own_inner.to_string());
            }

            let merged_json = format!("{{{}}}", merged_parts.join(","));
            code_parts.push(format!("{}const {} = css({})", prefix, name, merged_json));
        }
    }

    // Generate createTheme exports (class name and optionally vars object)
    // Note: CSS variables are added to global_styles during remapping
    let mut themes: Vec<_> = collected.themes.iter().collect();
    themes.sort_by_key(|(name, _)| *name);
    for (name, entry) in themes {
        let prefix = if entry.exported { "export " } else { "" };
        // If this theme has a vars_name, output as array destructuring
        if let Some(vars_name) = &entry.vars_name {
            if let Some(vars_json) = &entry.vars_object_json {
                code_parts.push(format!(
                    "{}const [{}, {}] = [\"{}\", {}]",
                    prefix, name, vars_name, entry.class_name, vars_json
                ));
            } else {
                code_parts.push(format!(
                    "{}const {} = \"{}\"",
                    prefix, name, entry.class_name
                ));
            }
        } else {
            code_parts.push(format!(
                "{}const {} = \"{}\"",
                prefix, name, entry.class_name
            ));
        }
    }

    // Generate globalCss calls
    for (selector, json) in &collected.global_styles {
        code_parts.push(format!("globalCss({{ \"{}\": {} }})", selector, json));
    }

    // Generate @font-face rules via globalCss fontFaces (sorted for deterministic output)
    // NOTE: fontFaces are generated in globalCss format here.
    // The extractor will then parse and extract them as FontFace styles.
    let mut font_faces_sorted: Vec<_> = collected.font_faces.iter().collect();
    font_faces_sorted.sort_by_key(|(name, _)| *name);
    for (_name, (json, font_family, _exported)) in font_faces_sorted {
        // Parse JSON and build JS object literal - clean single-line format
        let props = parse_font_face_json(json);
        let props_str = props
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join(", ");

        // Generate clean single-line globalCss call
        let code = if props_str.is_empty() {
            format!(
                "globalCss({{ fontFaces: [{{ fontFamily: \"{}\" }}] }})",
                font_family
            )
        } else {
            format!(
                "globalCss({{ fontFaces: [{{ fontFamily: \"{}\", {} }}] }})",
                font_family, props_str
            )
        };
        code_parts.push(code);
    }

    // Generate createGlobalTheme CSS variables via globalCss (sorted for deterministic output)
    let mut global_themes_sorted: Vec<_> = collected.global_themes.iter().collect();
    global_themes_sorted.sort_by_key(|(name, _)| *name);
    for (_name, entry) in &global_themes_sorted {
        if !entry.css_vars.is_empty() {
            // Build CSS variables object for the selector
            let vars_str = entry
                .css_vars
                .iter()
                .map(|(var_name, value)| format!("\"{}\": \"{}\"", var_name, value))
                .collect::<Vec<_>>()
                .join(", ");
            code_parts.push(format!(
                "globalCss({{ \"{}\": {{ {} }} }})",
                entry.selector, vars_str
            ));
        }
    }

    // Generate keyframes declarations (sorted for deterministic output)
    let mut keyframes: Vec<_> = collected.keyframes.iter().collect();
    keyframes.sort_by_key(|(name, _)| *name);
    for (name, entry) in keyframes {
        let prefix = if entry.exported { "export " } else { "" };
        code_parts.push(format!(
            "{}const {} = keyframes({})",
            prefix, name, entry.json
        ));
    }

    // Generate styleVariants - produce an object with variant keys
    let mut variants: Vec<_> = collected.style_variants.iter().collect();
    variants.sort_by_key(|(name, _)| *name);
    for (name, (variant_map, exported)) in variants {
        // Sort variant keys for deterministic output
        let mut variant_entries: Vec<_> = variant_map.iter().collect();
        variant_entries.sort_by_key(|(k, _)| *k);

        let mut object_parts = Vec::new();
        for (variant_key, variant) in variant_entries {
            let value = if let Some(base_name) = &variant.base {
                // Composition: merge base styles + variant styles into single css() call
                let mut merged_parts = Vec::new();

                // Add base styles
                if let Some(base_json) = style_json_map.get(base_name.as_str()) {
                    let inner = base_json
                        .trim()
                        .trim_start_matches('{')
                        .trim_end_matches('}')
                        .trim();
                    if !inner.is_empty() {
                        merged_parts.push(inner.to_string());
                    }
                }

                // Add variant's own styles
                let own_inner = variant
                    .styles_json
                    .trim()
                    .trim_start_matches('{')
                    .trim_end_matches('}')
                    .trim();
                if !own_inner.is_empty() {
                    merged_parts.push(own_inner.to_string());
                }

                format!("css({{{}}})", merged_parts.join(","))
            } else {
                // No composition, just the styles
                format!("css({})", variant.styles_json)
            };
            object_parts.push(format!("  {}: {}", variant_key, value));
        }

        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!(
            "{}const {} = {{\n{}\n}}",
            prefix,
            name,
            object_parts.join(",\n")
        ));
    }

    // Generate createVar declarations (sorted for deterministic output)
    let mut vars: Vec<_> = collected.vars.iter().collect();
    vars.sort_by_key(|(name, _)| *name);
    for (name, (value, exported)) in vars {
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!("{}const {} = \"{}\"", prefix, name, value));
    }

    // Generate fontFace declarations (sorted for deterministic output)
    // fontFace returns the font-family name that can be used in style({ fontFamily: ... })
    let mut font_faces: Vec<_> = collected.font_faces.iter().collect();
    font_faces.sort_by_key(|(name, _)| *name);
    for (name, (_, font_family, exported)) in font_faces {
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!("{}const {} = \"{}\"", prefix, name, font_family));
    }

    // Generate createContainer declarations (sorted for deterministic output)
    let mut containers: Vec<_> = collected.containers.iter().collect();
    containers.sort_by_key(|(name, _)| *name);
    for (name, (value, exported)) in containers {
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!("{}const {} = \"{}\"", prefix, name, value));
    }

    // Generate layer declarations (sorted for deterministic output)
    let mut layers: Vec<_> = collected.layers.iter().collect();
    layers.sort_by_key(|(name, _)| *name);
    for (name, (value, exported)) in layers {
        let prefix = if *exported { "export " } else { "" };
        code_parts.push(format!("{}const {} = \"{}\"", prefix, name, value));
    }

    // Generate createGlobalTheme vars object declarations (sorted for deterministic output)
    for (name, entry) in &global_themes_sorted {
        let prefix = if entry.exported { "export " } else { "" };
        code_parts.push(format!(
            "{}const {} = {}",
            prefix, name, entry.vars_object_json
        ));
    }

    // Generate constant exports (sorted for deterministic output)
    let mut constants: Vec<_> = collected.constant_exports.iter().collect();
    constants.sort_by_key(|(name, _)| *name);
    for (name, value) in constants {
        code_parts.push(format!("export const {} = {}", name, value));
    }

    code_parts.join("\n")
}

impl Clone for CollectedStyles {
    fn clone(&self) -> Self {
        Self {
            styles: self.styles.clone(),
            global_styles: self.global_styles.clone(),
            keyframes: self.keyframes.clone(),
            vars: self.vars.clone(),
            font_faces: self.font_faces.clone(),
            style_variants: self.style_variants.clone(),
            containers: self.containers.clone(),
            layers: self.layers.clone(),
            global_themes: self.global_themes.clone(),
            themes: self.themes.clone(),
            theme_vars: self.theme_vars.clone(),
            constant_exports: self.constant_exports.clone(),
        }
    }
}

/// Parse a styleVariants object and extract variant info
fn parse_style_variants(
    variants_obj: &JsValue,
    context: &mut Context,
) -> HashMap<String, StyleVariant> {
    let mut result = HashMap::new();

    if let Some(obj) = variants_obj.as_object() {
        // Get the object's own property keys
        if let Ok(keys) = obj.own_property_keys(context) {
            for key in keys {
                // Convert PropertyKey to string
                let key_name = match &key {
                    boa_engine::property::PropertyKey::String(s) => s.to_std_string_escaped(),
                    boa_engine::property::PropertyKey::Symbol(_) => continue,
                    boa_engine::property::PropertyKey::Index(i) => i.get().to_string(),
                };

                if let Ok(value) = obj.get(key.clone(), context) {
                    let variant = parse_single_variant(&value, context);
                    result.insert(key_name, variant);
                }
            }
        }
    }

    result
}

/// Parse a single variant value (either style object or [base, styleObj] array)
fn parse_single_variant(value: &JsValue, context: &mut Context) -> StyleVariant {
    // Check if value is an array by checking if it has a numeric "length" property
    if let Some(obj) = value.as_object()
        && let Ok(length_val) = obj.get(js_string!("length"), context)
        && let Some(len) = length_val.as_number()
    {
        let len = len as u32;
        if len >= 2 {
            // It's an array with at least 2 elements
            if let Ok(first) = obj.get(0, context)
                && let Some(base_str) = first.as_string()
            {
                let base_class = base_str.to_std_string_escaped();
                // Check if it looks like a placeholder (__style_N__) or class name
                if base_class.starts_with("__style_") || !base_class.contains('{') {
                    // Get the style object (second element)
                    if let Ok(style_obj) = obj.get(1, context) {
                        let json = js_value_to_json(&style_obj, context);
                        return StyleVariant {
                            base: Some(base_class),
                            styles_json: json,
                        };
                    }
                }
            }
        }
    }

    // Not an array or not composition - treat as plain style object
    StyleVariant {
        base: None,
        styles_json: js_value_to_json(value, context),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vanilla_extract_file() {
        assert!(is_vanilla_extract_file("styles.css.ts"));
        assert!(is_vanilla_extract_file("theme.css.js"));
        assert!(is_vanilla_extract_file("path/to/styles.css.ts"));
        assert!(!is_vanilla_extract_file("styles.ts"));
        assert!(!is_vanilla_extract_file("styles.css"));
        assert!(!is_vanilla_extract_file("component.tsx"));
    }

    #[test]
    fn test_preprocess_typescript() {
        let code = r#"import { style } from '@devup-ui/react'
export const container = style({ background: "red" })"#;
        let result = preprocess_typescript(code, "@devup-ui/react");
        // The result should have destructuring from __vanilla_extract__ and no export keyword
        assert!(
            result.contains("__vanilla_extract__"),
            "Expected __vanilla_extract__ but got: {}",
            result
        );
        assert!(
            !result.contains("export const"),
            "Should not contain 'export const': {}",
            result
        );
    }

    #[test]
    fn test_preprocess_typescript_strips_types() {
        let code = r#"import { style } from '@devup-ui/react'
interface Props {
    color: string;
}
export const container = style({ background: "red" })"#;
        let result = preprocess_typescript(code, "@devup-ui/react");
        // TypeScript interface should be stripped
        assert!(
            !result.contains("interface"),
            "Should not contain interface: {}",
            result
        );
    }

    #[test]
    fn test_execute_vanilla_extract_style() {
        let code = r#"import { style } from '@devup-ui/react'
export const container = style({ background: "red", padding: 16 })"#;
        let result = execute_vanilla_extract(code, "@devup-ui/react", "test.css.ts").unwrap();
        assert!(!result.styles.is_empty());
    }

    #[test]
    fn test_execute_vanilla_extract_global_style() {
        let code = r#"import { globalStyle } from '@devup-ui/react'
globalStyle("body", { margin: 0, padding: 0 })"#;
        let result = execute_vanilla_extract(code, "@devup-ui/react", "test.css.ts").unwrap();
        assert_eq!(result.global_styles.len(), 1);
        assert_eq!(result.global_styles[0].0, "body");
    }

    #[test]
    fn test_collected_styles_to_code() {
        let mut collected = CollectedStyles::default();
        collected.styles.insert(
            "container".to_string(),
            StyleEntry {
                json: r#"{"background":"red"}"#.to_string(),
                exported: true,
                bases: Vec::new(),
            },
        );

        let code = collected_styles_to_code(&collected, "@devup-ui/react");
        assert!(code.contains("import { css } from '@devup-ui/react'"));
        assert!(code.contains(r#"export const container = css({"background":"red"})"#));
    }

    #[test]
    fn test_execute_vanilla_extract_with_variable() {
        // Test that variables are evaluated at execution time
        let code = r#"import { style } from '@devup-ui/react'
const primaryColor = "blue";
const spacing = 16;
export const button = style({ background: primaryColor, padding: spacing })"#;
        let result = execute_vanilla_extract(code, "@devup-ui/react", "test.css.ts").unwrap();
        assert!(result.styles.contains_key("button"));
        let entry = &result.styles["button"];
        // The variable values should be resolved
        assert!(
            entry.json.contains("blue"),
            "Expected 'blue' in JSON: {}",
            entry.json
        );
        assert!(
            entry.json.contains("16"),
            "Expected '16' in JSON: {}",
            entry.json
        );
    }

    #[test]
    fn test_execute_vanilla_extract_with_computed_value() {
        // Test computed values
        let code = r#"import { style } from '@devup-ui/react'
const base = 8;
export const box = style({ padding: base * 2, margin: base / 2 })"#;
        let result = execute_vanilla_extract(code, "@devup-ui/react", "test.css.ts").unwrap();
        assert!(result.styles.contains_key("box"));
        let entry = &result.styles["box"];
        assert!(
            entry.json.contains("16"),
            "Expected padding 16 in JSON: {}",
            entry.json
        );
        assert!(
            entry.json.contains("4"),
            "Expected margin 4 in JSON: {}",
            entry.json
        );
    }

    #[test]
    fn test_execute_vanilla_extract_with_conditional() {
        // Test conditional expressions
        let code = r#"import { style } from '@devup-ui/react'
const isDark = true;
export const theme = style({ background: isDark ? "black" : "white" })"#;
        let result = execute_vanilla_extract(code, "@devup-ui/react", "test.css.ts").unwrap();
        assert!(result.styles.contains_key("theme"));
        let entry = &result.styles["theme"];
        assert!(
            entry.json.contains("black"),
            "Expected 'black' in JSON: {}",
            entry.json
        );
    }

    #[test]
    fn test_execute_vanilla_extract_with_spread() {
        // Test spread operator
        let code = r#"import { style } from '@devup-ui/react'
const baseStyle = { padding: 8, margin: 4 };
export const extended = style({ ...baseStyle, background: "red" })"#;
        let result = execute_vanilla_extract(code, "@devup-ui/react", "test.css.ts").unwrap();
        assert!(result.styles.contains_key("extended"));
        let entry = &result.styles["extended"];
        assert!(
            entry.json.contains("8"),
            "Expected padding 8 in JSON: {}",
            entry.json
        );
        assert!(
            entry.json.contains("4"),
            "Expected margin 4 in JSON: {}",
            entry.json
        );
        assert!(
            entry.json.contains("red"),
            "Expected 'red' in JSON: {}",
            entry.json
        );
    }

    #[test]
    fn test_execute_vanilla_extract_create_theme_contract() {
        // Test createThemeContract + createTheme
        let code = r#"import { createThemeContract, createTheme } from '@devup-ui/react'
const vars = createThemeContract({
  color: {
    brand: null,
    text: null
  }
})
export const lightTheme = createTheme(vars, {
  color: {
    brand: 'blue',
    text: 'black'
  }
})"#;
        let result = execute_vanilla_extract(code, "@devup-ui/react", "test.css.ts").unwrap();

        // Check that themes were collected
        assert!(
            !result.themes.is_empty(),
            "Expected themes to be collected, got empty. Themes: {:?}",
            result.themes
        );
        assert!(
            result.themes.contains_key("lightTheme"),
            "Expected lightTheme in themes: {:?}",
            result.themes.keys().collect::<Vec<_>>()
        );

        // Check CSS vars
        let theme_entry = &result.themes["lightTheme"];
        assert!(
            !theme_entry.css_vars.is_empty(),
            "Expected CSS vars in theme: {:?}",
            theme_entry
        );

        // Check that CSS vars are correct
        let css_vars: Vec<_> = theme_entry.css_vars.iter().collect();
        assert!(
            css_vars
                .iter()
                .any(|(name, val)| name == "--color-brand" && val == "blue"),
            "Expected --color-brand: blue in {:?}",
            css_vars
        );
    }
}
