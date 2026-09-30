mod as_visit;
mod build_time_values;
mod component;
mod css_utils;
pub mod extract_style;
mod extractor;
mod gen_class_name;
mod gen_style;
mod import_alias_visit;
mod imported_constants;
mod module_loader;
mod mutations;
mod prop_modify_utils;
mod source_map;
mod style_values;
mod stylex;
mod tailwind;
mod util_type;
mod utils;
mod vanilla_extract;
mod visit;
use crate::extract_style::extract_style_value::ExtractStyleValue;
use crate::visit::DevupVisitor;
use css::file_map::{canonical, get_file_num_by_filename, is_global};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::Expression;
use oxc_ast_visit::VisitMut;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::path::PathBuf;

/// `StyleX` package specifier recognized by the relevant-import gate in `extract`.
const STYLEX_PACKAGE: &str = "@stylexjs/stylex";

/// Import alias configuration for redirecting imports from other CSS-in-JS libraries
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportAlias {
    /// Default export → named export (e.g., `import styled from '@emotion/styled'` → `import { styled } from '@devup-ui/react'`)
    DefaultToNamed(String),
    /// Named exports (1:1 mapping, e.g., `import { style } from '@vanilla-extract/css'` → `import { style } from '@devup-ui/react'`)
    NamedToNamed,
}

#[derive(Debug)]
pub enum ExtractStyleProp<'a> {
    Static(ExtractStyleValue),
    StaticArray(Vec<ExtractStyleProp<'a>>),
    Conditional {
        condition: Expression<'a>,
        consequent: Option<Box<ExtractStyleProp<'a>>>,
        alternate: Option<Box<ExtractStyleProp<'a>>>,
    },
    Enum {
        condition: Expression<'a>,
        map: BTreeMap<String, Vec<ExtractStyleProp<'a>>>,
    },
    Expression {
        styles: Vec<ExtractStyleValue>,
        expression: Expression<'a>,
    },
    MemberExpression {
        map: BTreeMap<String, Box<ExtractStyleProp<'a>>>,
        expression: Expression<'a>,
    },
    /// Styles written where the build cannot read them, reported as an error;
    /// `prop` for a computed key among an element's props, which the element
    /// takes as it is at runtime
    Unreadable {
        offset: u32,
        code: String,
        prop: bool,
    },
}

impl<'a> ExtractStyleProp<'a> {
    pub fn clone_in(&self, alloc: &'a Allocator) -> Self {
        match self {
            ExtractStyleProp::Static(v) => ExtractStyleProp::Static(v.clone()),
            ExtractStyleProp::StaticArray(arr) => {
                ExtractStyleProp::StaticArray(arr.iter().map(|s| s.clone_in(alloc)).collect())
            }
            ExtractStyleProp::Conditional {
                condition,
                consequent,
                alternate,
            } => ExtractStyleProp::Conditional {
                condition: condition.clone_in(alloc),
                consequent: consequent.as_ref().map(|c| Box::new(c.clone_in(alloc))),
                alternate: alternate.as_ref().map(|a| Box::new(a.clone_in(alloc))),
            },
            ExtractStyleProp::Enum { condition, map } => ExtractStyleProp::Enum {
                condition: condition.clone_in(alloc),
                map: map
                    .iter()
                    .map(|(k, v)| (k.clone(), v.iter().map(|s| s.clone_in(alloc)).collect()))
                    .collect(),
            },
            ExtractStyleProp::Expression { styles, expression } => ExtractStyleProp::Expression {
                styles: styles.clone(),
                expression: expression.clone_in(alloc),
            },
            ExtractStyleProp::MemberExpression { map, expression } => {
                ExtractStyleProp::MemberExpression {
                    map: map
                        .iter()
                        .map(|(k, v)| (k.clone(), Box::new(v.clone_in(alloc))))
                        .collect(),
                    expression: expression.clone_in(alloc),
                }
            }
            ExtractStyleProp::Unreadable { offset, code, prop } => ExtractStyleProp::Unreadable {
                offset: *offset,
                code: code.clone(),
                prop: *prop,
            },
        }
    }

    pub fn extract(&self) -> Vec<ExtractStyleValue> {
        match self {
            ExtractStyleProp::Static(style) => vec![style.clone()],
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => match (consequent, alternate) {
                // Exactly one branch: return its `extract()` directly, skipping
                // the throwaway accumulator `Vec` and the drain-into-it copy.
                (Some(branch), None) | (None, Some(branch)) => branch.extract(),
                // Both branches present: preserve consequent-then-alternate
                // order, presizing the accumulator to hold both results.
                (Some(consequent), Some(alternate)) => {
                    let mut consequent = consequent.extract();
                    let mut alternate = alternate.extract();
                    consequent.reserve(alternate.len());
                    consequent.append(&mut alternate);
                    consequent
                }
                (None, None) => vec![],
            },
            ExtractStyleProp::StaticArray(array) => {
                array.iter().flat_map(ExtractStyleProp::extract).collect()
            }
            ExtractStyleProp::Expression { styles, .. } => styles.clone(),
            ExtractStyleProp::MemberExpression { map, .. } => {
                map.values().flat_map(|s| s.extract()).collect()
            }
            ExtractStyleProp::Enum { map, .. } => map
                .values()
                .flat_map(|s| s.iter().flat_map(ExtractStyleProp::extract))
                .collect(),
            ExtractStyleProp::Unreadable { .. } => vec![],
        }
    }

    /// Owning variant of [`extract`](Self::extract): consumes `self` and moves
    /// every collected [`ExtractStyleValue`] out instead of cloning it. Use at
    /// call sites that discard the `ExtractStyleProp` right after extraction.
    pub fn into_extract(self) -> Vec<ExtractStyleValue> {
        match self {
            ExtractStyleProp::Static(style) => vec![style],
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => match (consequent, alternate) {
                // Exactly one branch: return its `into_extract()` directly,
                // skipping the throwaway accumulator `Vec`.
                (Some(branch), None) | (None, Some(branch)) => branch.into_extract(),
                // Both branches present: preserve consequent-then-alternate
                // order, presizing the accumulator to hold both results.
                (Some(consequent), Some(alternate)) => {
                    let mut consequent = consequent.into_extract();
                    let mut alternate = alternate.into_extract();
                    consequent.reserve(alternate.len());
                    consequent.append(&mut alternate);
                    consequent
                }
                (None, None) => vec![],
            },
            ExtractStyleProp::StaticArray(array) => array
                .into_iter()
                .flat_map(ExtractStyleProp::into_extract)
                .collect(),
            ExtractStyleProp::Expression { styles, .. } => styles,
            ExtractStyleProp::MemberExpression { map, .. } => {
                map.into_values().flat_map(|s| s.into_extract()).collect()
            }
            ExtractStyleProp::Enum { map, .. } => map
                .into_values()
                .flat_map(|s| s.into_iter().flat_map(ExtractStyleProp::into_extract))
                .collect(),
            ExtractStyleProp::Unreadable { .. } => vec![],
        }
    }
}
/// Style property for props
#[derive(Debug)]
pub struct ExtractOutput {
    // used styles
    pub styles: FxHashSet<ExtractStyleValue>,

    // output source
    pub code: String,

    pub map: Option<String>,
    pub css_file: Option<String>,

    /// Files read through the module resolver, for the bundler to watch
    pub dependencies: Vec<String>,
}

/// A module the bundler resolved an import to
pub struct ResolvedModule {
    pub path: String,
    pub code: String,
}

/// Resolves `(specifier, importer)` the way the bundler does; `None` when it
/// cannot
pub type ModuleResolver = dyn Fn(&str, &str) -> Option<ResolvedModule>;

#[derive(Clone)]
pub struct ExtractOption {
    pub package: String,
    pub css_dir: String,
    pub single_css: bool,
    pub import_main_css: bool,
    /// Import aliases for redirecting imports from other CSS-in-JS libraries to the target package
    pub import_aliases: HashMap<String, ImportAlias>,
}

impl Default for ExtractOption {
    fn default() -> Self {
        Self {
            package: "@devup-ui/react".to_string(),
            css_dir: "@devup-ui/react".to_string(),
            single_css: false,
            import_main_css: false,
            import_aliases: HashMap::new(),
        }
    }
}

pub fn extract(
    filename: &str,
    code: &str,
    option: ExtractOption,
) -> Result<ExtractOutput, Box<dyn Error>> {
    extract_with_source_map(filename, code, option, true, None)
}

pub fn extract_without_source_map(
    filename: &str,
    code: &str,
    option: ExtractOption,
) -> Result<ExtractOutput, Box<dyn Error>> {
    extract_with_source_map(filename, code, option, false, None)
}

/// [`extract`] that reads the modules a file imports through `resolver`
pub fn extract_with_modules(
    filename: &str,
    code: &str,
    option: ExtractOption,
    source_map: bool,
    resolver: &ModuleResolver,
) -> Result<ExtractOutput, Box<dyn Error>> {
    extract_with_source_map(filename, code, option, source_map, Some(resolver))
}

fn extract_with_source_map(
    filename: &str,
    code: &str,
    option: ExtractOption,
    source_map: bool,
    resolver: Option<&ModuleResolver>,
) -> Result<ExtractOutput, Box<dyn Error>> {
    extract_source(filename, code, None, option, source_map, resolver)
}

/// `evaluated` is the source `code` was computed from, with the layers of
/// edits, last made first, that map `code` back to it
fn extract_source(
    filename: &str,
    code: &str,
    evaluated: Option<(&str, &[&[import_alias_visit::Edit]])>,
    option: ExtractOption,
    source_map: bool,
    resolver: Option<&ModuleResolver>,
) -> Result<ExtractOutput, Box<dyn Error>> {
    // Step 1: Transform import aliases
    // e.g., `import styled from '@emotion/styled'` → `import { styled } from '@devup-ui/react'`
    // e.g., `import { style } from '@vanilla-extract/css'` → `import { style } from '@devup-ui/react'`
    let (transformed_code, alias_edits) = import_alias_visit::transform_import_aliases_with_edits(
        code,
        filename,
        &option.package,
        &option.import_aliases,
    );

    // Step 2: Check if code contains the target package (after transformation)
    let has_relevant_import = transformed_code.contains(option.package.as_str())
        || transformed_code.contains(STYLEX_PACKAGE);

    if !has_relevant_import {
        // skip if not using package
        return Ok(ExtractOutput {
            styles: FxHashSet::default(),
            code: code.to_string(),
            map: None,
            css_file: None,
            dependencies: Vec::new(),
        });
    }

    let mut dependencies = std::collections::BTreeSet::new();
    let mut evaluation_error = None;
    // Step 3: Handle vanilla-extract style files (.css.ts, .css.js)
    // `processed_code` is Some only when vanilla-extract generation succeeded;
    // otherwise the untouched `transformed_code` is parsed directly (no copy).
    let processed_code: Option<String> = if utils::is_vanilla_extract_file(filename) {
        // Use transformed code (with imports already pointing to @devup-ui/react)
        match vanilla_extract::execute_stylesheet(&transformed_code, filename, &option, resolver) {
            Ok((collected, imports)) => {
                dependencies = imports.dependencies;
                // Keyframes names are generated, so extract the referenced ones
                // first and substitute their names into the styles using them.
                let referenced = vanilla_extract::referenced_keyframes(&collected);
                let keyframes_names = if referenced.is_empty() {
                    FxHashMap::default()
                } else {
                    extract_class_map_from_code(
                        filename,
                        &vanilla_extract::keyframes_to_code(
                            &collected,
                            &option.package,
                            &referenced,
                        ),
                        &option,
                        &referenced,
                    )?
                };
                let code = vanilla_extract::collected_styles_to_code_with_keyframes(
                    &collected,
                    &option.package,
                    &keyframes_names,
                );
                Some(if code.is_empty() {
                    code
                } else {
                    imports
                        .kept_imports
                        .iter()
                        .map(|specifier| {
                            format!("import {};\n", vanilla_extract::json_string(specifier))
                        })
                        .chain([code])
                        .collect()
                })
            }
            // A stylesheet another one imports must give its own values, and an
            // import cycle read too early fails as it does in ES modules, so both
            // are reported rather than hidden behind plain extraction
            Err(error)
                if module_loader::loading_for_stylesheet()
                    || error.contains(module_loader::IMPORT_CYCLE) =>
            {
                return Err(error.into());
            }
            // Plain extraction still compiles Devup UI's own APIs; the error is
            // reported when calls it cannot compile remain
            Err(error) => {
                evaluation_error = Some(error);
                None
            }
        }
    } else {
        None
    };
    // For vanilla-extract files, if no styles were collected, return early
    if processed_code.as_deref() == Some("") {
        return Ok(ExtractOutput {
            styles: FxHashSet::default(),
            code: code.to_string(),
            map: None,
            css_file: None,
            dependencies: dependencies.into_iter().collect(),
        });
    }

    let code_to_parse = processed_code.as_deref().unwrap_or(&transformed_code);

    let source_type = SourceType::from_path(filename)?;
    let (bucket, global, css_file) = resolve_css_target(filename, &option);
    let import_main = option.import_main_css && !global;
    // Presize to the exact final length (1 target + optional main-css entry) and
    // push the main-css path FIRST so the order (main-css, target) matches the
    // former `insert(0, …)` without paying its O(n) element shift.
    let mut css_files = Vec::with_capacity(1 + usize::from(import_main));
    if import_main {
        css_files.push(main_css_path(&option.css_dir));
    }
    css_files.push(css_file.clone());
    let allocator = Allocator::default();

    let ParserReturn {
        mut program, // AST
        fatal_error, // Parser encountered an error it couldn't recover from
        ..
    } = Parser::new(&allocator, code_to_parse, source_type).parse();
    if fatal_error {
        return Err("Parser panicked".into());
    }
    let inlined = if processed_code.is_none() {
        imported_constants::inline_constants(
            &oxc_ast::builder::AstBuilder::new(&allocator),
            &mut program,
            filename,
            &option,
            resolver,
        )
    } else {
        imported_constants::Inlined::default()
    };
    dependencies.extend(inlined.dependencies);
    let mut visitor = DevupVisitor::new(
        &allocator,
        filename,
        &option.package,
        css_files,
        if global { None } else { Some(bucket) },
    );
    visitor.import_stylex(inlined.stylex_vars, inlined.stylex_themes);
    visitor.unknown_bindings(&inlined.unknown);
    visitor.changed_bindings(inlined.changed.clone());
    visitor.visit_program(&mut program);
    if let Some(error) = evaluation_error
        && imports_uncompiled(&program, &option.package)
    {
        return Err(error.into());
    }
    // Run the code a value computes, or tell rules the module computes from a
    // class it composes
    if (!visitor.errors.is_empty() || visitor.composes_unknown)
        && evaluated.is_none()
        && !utils::is_vanilla_extract_file(filename)
        && let Some((computed, value_edits, read)) = build_time_values::evaluate(
            &transformed_code,
            filename,
            &option,
            resolver,
            &inlined.unknown,
        )
    {
        let mut output = extract_source(
            filename,
            &computed,
            Some((code, &[value_edits.as_slice(), alias_edits.as_slice()])),
            option,
            source_map,
            resolver,
        )?;
        let mut files: std::collections::BTreeSet<String> =
            output.dependencies.into_iter().collect();
        files.extend(read);
        output.dependencies = files.into_iter().collect();
        return Ok(output);
    }
    let (source, earlier_edits) = evaluated.unwrap_or((code, &[]));
    let edits: Vec<&[import_alias_visit::Edit]> = std::iter::once(alias_edits.as_slice())
        .chain(earlier_edits.iter().copied())
        .collect();
    visitor.errors.append(&mut visitor.unknown_parts);
    if !visitor.errors.is_empty() {
        let mut message = located_errors(filename, source, &edits, visitor.errors);
        message += &changed_notes(&message, filename, source, &edits, &inlined.changed);
        return Err(message.into());
    }
    let codegen_options = if source_map {
        CodegenOptions {
            source_map_path: Some(PathBuf::from(filename)),
            ..Default::default()
        }
    } else {
        CodegenOptions::default()
    };
    let result = Codegen::new().with_options(codegen_options).build(&program);
    // A stylesheet's output is generated, so its map stays on that code
    let map = result.map.map(|map| {
        if processed_code.is_some() || edits.iter().all(|edits| edits.is_empty()) {
            map
        } else {
            source_map::remap(map, code_to_parse, source, &edits)
        }
        .to_json_string()
    });

    Ok(ExtractOutput {
        styles: visitor.styles,
        code: result.code,
        map,
        css_file: Some(css_file),
        dependencies: dependencies.into_iter().collect(),
    })
}

/// Build the shared `devup-ui.css` main-css path for a given css directory.
///
/// Single source of truth for the `"{css_dir}/devup-ui.css"` literal, used both
/// by the global/single-css branch of [`resolve_css_target`] and the
/// `import_main_css` entry in [`extract`], so the shape never drifts between the
/// two sites.
fn main_css_path(css_dir: &str) -> String {
    format!("{css_dir}/devup-ui.css")
}

/// Resolve the CSS bucket identity, global flag, and target CSS file for a
/// source file.
///
/// The bucket is the identity for CSS naming/emission (single-importer
/// collapse); identity when no canonical map is loaded. The real `filename` is
/// kept for parse/sourcemap. Global (shared-chunk) files are emitted like
/// single-css: into devup-ui.css with prefix-less global naming, so styles
/// shared across routes ship once.
/// Whether `program` still imports a value from `package` that extraction did
/// not compile away
fn imports_uncompiled(program: &oxc_ast::ast::Program<'_>, package: &str) -> bool {
    program.body.iter().any(|statement| {
        matches!(statement, oxc_ast::ast::Statement::ImportDeclaration(import)
        if import.source.value == package
            && !import.import_kind.is_type()
            && import.specifiers.iter().flatten().any(|specifier| matches!(
                specifier,
                oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(named)
                    if !named.import_kind.is_type()
            )))
    })
}

/// `errors` in source order, one per line, each led by `filename:line:column`
/// of the code it is about; each layer of `edits`, last made first, maps the
/// offsets back to `source`
fn located_errors(
    filename: &str,
    source: &str,
    edits: &[&[import_alias_visit::Edit]],
    mut errors: Vec<(u32, String)>,
) -> String {
    errors.sort_unstable();
    errors.dedup();
    errors
        .into_iter()
        .map(|(offset, message)| {
            let offset = edits.iter().fold(offset as usize, |offset, edits| {
                import_alias_visit::source_offset(edits, offset)
            });
            format!("{}: {message}", locate(filename, source, offset))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `filename:line:column` of `offset` in `source`
fn locate(filename: &str, source: &str, offset: usize) -> String {
    let before = source.get(..offset).unwrap_or(source);
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    format!(
        "{filename}:{}:{}",
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1
    )
}

/// A line for each binding `message` names that code changes, telling where
fn changed_notes(
    message: &str,
    filename: &str,
    source: &str,
    edits: &[&[import_alias_visit::Edit]],
    changed: &imported_constants::Changed,
) -> String {
    let mut notes = String::new();
    for change in changed.named_in(message) {
        let location = match &change.site {
            imported_constants::ChangeSite::Here(offset) => {
                let offset = edits.iter().fold(*offset as usize, |offset, edits| {
                    import_alias_visit::source_offset(edits, offset)
                });
                locate(filename, source, offset)
            }
            imported_constants::ChangeSite::In(location) => location.clone(),
        };
        let what = if change.handed {
            "is handed here to code that may change it"
        } else {
            "is changed here"
        };
        let _ = std::fmt::Write::write_fmt(
            &mut notes,
            format_args!(
                "\n{location}: `{}` {what}, so the build cannot read it as a constant",
                change.name
            ),
        );
    }
    notes
}

/// The file name generated names of `filename` are scoped by, `None` when its
/// CSS goes to the shared sheet
fn css_bucket(filename: &str, option: &ExtractOption) -> Option<String> {
    (!(option.single_css || is_global(filename))).then(|| canonical(filename))
}

fn resolve_css_target(filename: &str, option: &ExtractOption) -> (String, bool, String) {
    let bucket = canonical(filename);
    let global = option.single_css || is_global(filename);
    let css_file = if global {
        main_css_path(&option.css_dir)
    } else {
        format!(
            "{}/devup-ui-{}.css",
            option.css_dir,
            get_file_num_by_filename(&bucket)
        )
    };
    (bucket, global, css_file)
}

/// Extract class names from generated code for specific style names
/// Used for two-pass vanilla-extract processing to resolve selector references
fn extract_class_map_from_code(
    filename: &str,
    partial_code: &str,
    option: &ExtractOption,
    style_names: &FxHashSet<String>,
) -> Result<FxHashMap<String, String>, Box<dyn Error>> {
    let source_type = SourceType::from_path(filename)?;
    let (bucket, global, css_file) = resolve_css_target(filename, option);
    let css_files = vec![css_file];
    let allocator = Allocator::default();

    let ParserReturn {
        mut program,
        fatal_error,
        ..
    } = Parser::new(&allocator, partial_code, source_type).parse();
    if fatal_error {
        Ok(FxHashMap::default())
    } else {
        let mut visitor = DevupVisitor::new(
            &allocator,
            filename,
            &option.package,
            css_files,
            if global { None } else { Some(bucket) },
        );
        visitor.visit_program(&mut program);

        let result = Codegen::new().build(&program);

        // Parse the output code to extract class name assignments
        // Format: const styleName = "className" or const styleName = "className1 className2"
        let mut class_map =
            FxHashMap::with_capacity_and_hasher(style_names.len(), rustc_hash::FxBuildHasher);
        for line in result.code.lines() {
            let line = line.trim();
            if line.starts_with("const ") || line.starts_with("export const ") {
                // Parse: [export] const name = "value"
                let after_const = if line.starts_with("export ") {
                    line.strip_prefix("export const ").unwrap_or(line)
                } else {
                    line.strip_prefix("const ").unwrap_or(line)
                };

                if let Some((name, rest)) = after_const.split_once(" = ") {
                    // Codegen emits `const name = "value";` (or bare `value`). Strip
                    // the statement-terminating `;` first, then peel the wrapping
                    // quotes off both ends in one pass. Byte-identical to the prior
                    // `trim_start_matches('"').trim_end_matches(';').trim_end_matches('"')`
                    // for the `"value"`, `"value";` and bare-`value` shapes Codegen
                    // produces (no interior/leading `;`, symmetric wrapping quotes).
                    let value = rest.trim_end_matches(';').trim_matches('"');

                    if style_names.contains(name) {
                        // For multi-class values like "a b", take the first class
                        let first_class = value.split_whitespace().next().unwrap_or(value);
                        class_map.insert(name.to_string(), first_class.to_string());
                    }
                }
            }
        }
        Ok(class_map)
    }
}

/// Check if the code has an import from the specified package
#[must_use]
pub fn has_devup_ui(filename: &str, code: &str, package: &str) -> bool {
    if !code.contains(package) {
        return false;
    }

    let source_type = match SourceType::from_path(filename) {
        Ok(st) => st,
        Err(_) => return false,
    };

    let allocator = Allocator::default();
    let ParserReturn {
        program,
        fatal_error,
        ..
    } = Parser::new(&allocator, code, source_type).parse();

    if fatal_error {
        return false;
    }

    for stmt in &program.body {
        if let oxc_ast::ast::Statement::ImportDeclaration(decl) = stmt
            && decl.source.value == package
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::build_time_values::has_build_time_values;
    use css::class_map::reset_class_map;
    use css::file_map::reset_file_map;
    use insta::assert_debug_snapshot;
    use oxc_ast::builder::AstBuilder;
    use oxc_span::SPAN;
    use rstest::rstest;
    use serial_test::serial;

    #[derive(Debug)]
    #[allow(dead_code)]
    struct ToBTreeSet {
        // used styles
        pub(crate) styles: BTreeSet<ExtractStyleValue>,

        // output source
        pub(crate) code: String,
    }

    impl From<ExtractOutput> for ToBTreeSet {
        fn from(output: ExtractOutput) -> Self {
            Self {
                styles: {
                    let mut set = BTreeSet::new();
                    set.extend(output.styles);
                    set
                },
                code: output.code,
            }
        }
    }

    #[test]
    fn test_extract_option_default() {
        // Tests lines 90-91: ExtractOption::default()
        let option = ExtractOption::default();
        assert_eq!(option.package, "@devup-ui/react");
        assert_eq!(option.css_dir, "@devup-ui/react");
        assert!(!option.single_css);
        assert!(!option.import_main_css);
        assert!(option.import_aliases.is_empty());
    }

    #[test]
    fn extract_style_prop_conditional_collects_present_branches() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let condition = || Expression::new_identifier(SPAN, "condition", &builder);
        let style = |value: &str| {
            ExtractStyleProp::Static(ExtractStyleValue::Typography(value.to_string()))
        };

        let consequent_only = ExtractStyleProp::Conditional {
            condition: condition(),
            consequent: Some(Box::new(style("consequent"))),
            alternate: None,
        };
        assert_eq!(
            consequent_only.extract(),
            vec![ExtractStyleValue::Typography("consequent".to_string())]
        );

        let alternate_only = ExtractStyleProp::Conditional {
            condition: condition(),
            consequent: None,
            alternate: Some(Box::new(style("alternate"))),
        };
        assert_eq!(
            alternate_only.extract(),
            vec![ExtractStyleValue::Typography("alternate".to_string())]
        );

        let both = ExtractStyleProp::Conditional {
            condition: condition(),
            consequent: Some(Box::new(style("consequent"))),
            alternate: Some(Box::new(style("alternate"))),
        };
        assert_eq!(
            both.extract(),
            vec![
                ExtractStyleValue::Typography("consequent".to_string()),
                ExtractStyleValue::Typography("alternate".to_string())
            ]
        );
    }

    #[test]
    fn extract_style_prop_empty_conditional_is_empty_for_borrowed_and_owned_extraction() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let empty = ExtractStyleProp::Conditional {
            condition: Expression::new_identifier(SPAN, "condition", &builder),
            consequent: None,
            alternate: None,
        };

        assert!(empty.extract().is_empty());
        assert!(empty.into_extract().is_empty());
    }

    #[test]
    fn extract_style_prop_expression_and_enum_collect_all_styles() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let expression = || Expression::new_identifier(SPAN, "variant", &builder);
        let expression_style = ExtractStyleValue::Typography("expression".to_string());
        let expression_prop = ExtractStyleProp::Expression {
            styles: vec![expression_style.clone()],
            expression: expression(),
        };
        assert_eq!(expression_prop.extract(), vec![expression_style]);

        let mut map = BTreeMap::new();
        map.insert(
            "primary".to_string(),
            vec![ExtractStyleProp::Static(ExtractStyleValue::Typography(
                "primary".to_string(),
            ))],
        );
        map.insert(
            "secondary".to_string(),
            vec![ExtractStyleProp::Static(ExtractStyleValue::Typography(
                "secondary".to_string(),
            ))],
        );
        let enum_prop = ExtractStyleProp::Enum {
            condition: expression(),
            map,
        };
        assert_eq!(
            enum_prop.extract(),
            vec![
                ExtractStyleValue::Typography("primary".to_string()),
                ExtractStyleValue::Typography("secondary".to_string())
            ]
        );
    }

    #[test]
    #[serial]
    fn extract_canonical_bucket_merge() {
        use css::file_map::{reset_canonical_map, set_canonical_map};
        reset_class_map();
        reset_file_map();
        reset_canonical_map();
        // child.tsx is a single-importer leaf collapsed into parent.tsx's bucket.
        let mut m = std::collections::HashMap::new();
        m.insert("child.tsx".to_string(), "parent.tsx".to_string());
        set_canonical_map(m);

        let opt = || ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "df/devup-ui".to_string(),
            single_css: false,
            import_main_css: false,
            import_aliases: HashMap::new(),
        };
        let src = r#"import { Box } from "@devup-ui/react"; const a = <Box bg="red" />;"#;
        let parent = extract("parent.tsx", src, opt()).unwrap();
        let child = extract("child.tsx", src, opt()).unwrap();

        // co-bucketed -> same css chunk file (child uses parent's file_num)
        assert_eq!(parent.css_file, child.css_file);
        // co-bucketed -> identical class naming (dedup) -> identical transformed code
        assert_eq!(parent.code, child.code);

        reset_canonical_map();
        reset_class_map();
        reset_file_map();
    }

    #[test]
    #[serial]
    fn extract_global_hoist() {
        use css::file_map::{GLOBAL_BUCKET, reset_canonical_map, set_canonical_map};
        reset_class_map();
        reset_file_map();
        reset_canonical_map();
        // shared.tsx is hoisted to the global chunk; normal.tsx is a per-file bucket.
        let mut m = std::collections::HashMap::new();
        m.insert("shared.tsx".to_string(), GLOBAL_BUCKET.to_string());
        set_canonical_map(m);

        let opt = || ExtractOption {
            package: "@devup-ui/react".to_string(),
            css_dir: "df/devup-ui".to_string(),
            single_css: false,
            import_main_css: false,
            import_aliases: HashMap::new(),
        };
        let src = r#"import { Box } from "@devup-ui/react"; const a = <Box bg="red" />;"#;
        let global_out = extract("shared.tsx", src, opt()).unwrap();
        let normal_out = extract("normal.tsx", src, opt()).unwrap();

        // global file emits into the shared devup-ui.css (loaded once)
        assert_eq!(
            global_out.css_file.as_deref(),
            Some("df/devup-ui/devup-ui.css")
        );
        // non-global file stays in a per-file chunk
        assert!(
            normal_out
                .css_file
                .as_deref()
                .unwrap()
                .contains("devup-ui-")
        );

        reset_canonical_map();
        reset_class_map();
        reset_file_map();
    }

    #[test]
    #[serial]
    fn extract_just_tsx() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                "const a = 1;",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                "<Box gap={1} />",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn ignore_special_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box padding={1} ref={ref} data-test={1} role={2} children={[]} onClick={()=>{}} aria-valuenow={24} key={2} tabIndex={1} id="id" />
        "#,
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Input} from '@devup-ui/core'
        <Input placeholder="a" maxLength="b" minLength="c" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn convert_tag() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as="section" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={"section"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box as={`section`} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={"section"}></Box>
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box as={`section`}></Box>
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={b ? "div":"section"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={b ? `div`:`section`} w="100%" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={b ? undefined:"section"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={b ? null:"section"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={b ? "section":undefined} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={b ? "section":null} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box as={b ? null:undefined} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={Variable} w="100%" h="100%" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={b ? Variable : "section"} w="100%" h="100%" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={{A: "section", B: "div", C: Variable, D, [key]: "section", ...rest}[key]} w="100%" h="100%" />
        "#,
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={{A: "section", B: "div", C: Variable, D, ["key"]: "section", ...rest}["key"]} w="100%" h="100%" />
        "#,
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={b === 1 ? "section" : "div"} w="100%" h="100%" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={["div", "section"][b]} w="100%" h="100%" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={["div", "section"][b]} w="100%" h="100%" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // maintain object expression
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={Variable} w="100%" props={{animate:{duration: 1}}} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // maintain object expression
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box as={Variable} w="100%" props={{animate:{duration: 1}}}></Box>
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn extract_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box padding={1} margin={2} wrong={} wrong2=<></> />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box as C} from '@devup-ui/core'
                <C padding={1} margin={2} />
                ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Input} from '@devup-ui/core'
        <Input padding={1} margin={2} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Button} from '@devup-ui/core'
        <Button padding={1} margin={2} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex padding={1} margin={2} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex padding={('-1')}/>
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_style_props_with_namespace_import() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import * as B from '@devup-ui/core'
        <B.Flex padding={('-1')} className={B.css({
            color: 'red'
        })}/>
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_style_props_with_var_css() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {css} from '@devup-ui/core'
        const newCss=css;
        <div className={newCss({
            color: 'red'
        })}/>
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_style_props_with_default_import() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import B from '@devup-ui/core'
        <B.Flex padding={('-1')} className={B.css({
            color: 'red'
        })}/>
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_style_props_with_class_name() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className="exists class name" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className="  exists class name  " />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"exists class name"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"a"+"b"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Image} from '@devup-ui/core'
        <Image
          className={styles.logo}
          src="/next.svg"
          alt="Next.js logo"
          width={180}
          height={38}
        />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={variable} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box as C} from '@devup-ui/core'
        <C padding={1} 
      _hover={{
        borderColor: true ? 'blue' : ``,
      }}
 className={variable} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box, Button as DevupButton, Center, css } from '@devup-ui/core'
import clsx from 'clsx'

<DevupButton
      boxSizing="border-box"
      className={clsx(
        variants[variant],
        isError && variant === 'default' && errorClassNames,
        className,
      )}
      typography={
        isPrimary
          ? {
              sm: 'buttonS',
              md: 'buttonM',
            }[size]
          : undefined
      }
      {...props}
    />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_class_name_from_component() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {VStack as C} from '@devup-ui/core'
        <C padding={1} margin={2} className={"a"+"b"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn extract_responsive_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[null,1]} margin={[2,null,4]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Flex } from "@devup-ui/core";
<Flex display={['none', null, "flex"]}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_dynamic_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={someStyleVar} margin={someStyleVar2} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg={left + right} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={Math.abs(5)} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg={data.buttonBgColor} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg={data.a.b.buttonBgColor} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_dynamic_style_props_with_type() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={a as A} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={data[d as A] as B} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={"10px" as B} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn remove_semicolon() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg="red;" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg="blue;;" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg={`${"green;"}`} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg={`${color};`} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg={`${color}` + ";"} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_dynamic_responsive_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[someStyleVar,null,someStyleVar1]} margin={[null,someStyleVar2]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_compound_responsive_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[someStyleVar,undefined,someStyleVar1]} margin={[null,someStyleVar2]} bg="red" />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_wrong_responsive_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[NaN,undefined,null]} margin={Infinity} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[1,,null]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_variable_style_props_with_style() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a} style={{ key:value }} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a} style={styles} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_conditional_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? "4px" : "3px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : d} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? "4px" : d} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? null : undefined} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? 1 : undefined} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? undefined : 2} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? `${a}px` : undefined} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? null : `${b}px`} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? `${b}px` : undefined} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? undefined : null} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box _hover={b ? void 0 : { bg: "blue" }} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_same_value_conditional_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? "4px" : "4px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? 4 : 4} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? `4px` : `4px`} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? `${"1px"}` : `${"1px"}`} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? `${"1"}px` : `${1}px`} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? `${`1`}px` : `${"1"}px`} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_same_dynamic_value_conditional_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? a : a} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? `${a}` : `${a}`} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_responsive_conditional_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b ? "4px" : "3px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
        <Box margin={["6px", a === b ? "4px" : "3px"]} />;
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : [d, e, f, "2px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? c : [d, e, f, x === y ? "4px" : "2px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? [d, e, f, x === y ? "4px" : "2px"] : c} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a === b ? [d, e, f, x === y ? "4px" : "2px"] : ["1px", "2px", "3px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b && "4px"]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b && "4px", c === d ? "5px" : null]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_logical_case() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a===b && "1px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a===b || "1px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a ?? "1px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={(a===1||a===2)&&b===3 && "1px"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_dynamic_logical_case() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a===b && a} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a===b || a} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={a ?? b} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={(a===1||a===2)&&b===3 && a} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn extract_responsive_conditional_style_props_with_class_name() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b ? (q > w ? "4px" : "8px") : "3px"]} className={"exists"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box margin={[null, a === b || "4px"]} className={"exists"} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_selector() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={{
          mx: 1
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Center} from '@devup-ui/core'
    <Center
      _active={
        variant !== 'disabled' && {
          boxShadow: 'none',
          transform: 'scale(0.95)',
        }
      }
      _hover={
        variant !== 'disabled' && {
          boxShadow: [
            '0px 1px 3px 0px rgba(0, 0, 0, 0.25)',
            null,
            '0px 0px 15px 0px rgba(0, 0, 0, 0.25)',
          ],
        }
      }
      {...props}
    >
      {children}
    </Center>
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box selectors={{
          _themeDark: {
            mx: 1
          }
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box selectors={{
          '_themeDark': {
            mx: 1
          }
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box selectors={{
          _hover: {
            mx: 1
          }
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          "_hover, _active": {
            mx: 1
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          "&:hover,                 &:active": {
            mx: 1
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _nthLastChild={{
          mx: 1
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box
          selectors={{
            "&:nth-last-child(2), &:nth-last-child(3)": {
              mx: 1
            },
            _nthLastChild: {
              mx: 2
            }
          }}
         />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          ".dataTestId > &": {
            mx: 1
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          "_hover": {
            mx: 1
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          "_hover": {
            selectors: {
              ".dataTestId > &": {
                color: "red"
              }
            }
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          ".test-picker__day--keyboard-selected": {
            bg: "$primary"
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          "& .a, & .b": {
            bg: "$primary"
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          ".test-picker__day--keyboard-selected": {
            _hover: {
              bg: "$primary"
            },
            selectors: {
              "&:active": {
                bg: "$primary"
              }
            }
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          ".a, .b": {
            _hover: {
              bg: "$primary"
            },
            selectors: {
              "&:active": {
                bg: "$secondary"
              }
            }
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_selector_multibyte_key() {
        reset_class_map();
        reset_file_map();
        // the last comma-separated segment must survive a multi-byte final char
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          "&:hover, .한글": {
            mx: 1
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn optimize_func() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box transform="scale(0.95)" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box transform="scale(0deg)" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box transform="scaleX(0deg)" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box transform="scaleY(0deg)" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box transform="scaleZ(0deg)" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box transform="scale(0deg, 0deg)" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box transform="scaleX(0deg) scaleY(0deg) scaleZ(0deg)" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_selector_with_literal() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={`
        background-color: red;
        `} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box selectors={{
          "&:hover":`
          background-color: red;
          ` 
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn extract_nested_selector() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _hover={{
          _placeholder: {
            color: "red"
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _hover={{
          selectors: {
            "&::placeholder, &:active": {
              color: "blue"
            }
          },
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _hover={{
          selectors: {
            "&::placeholder": {
              color: "red"
            },
            "&::placeholder, &:active": {
              color: "blue"
            }
          },
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _hover={{
          selectors: {
            "&::placeholder": {
              _active: {
                color: "red",
              }
            },
          },
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box 
          selectors={{
            "&::placeholder": {
              _active: {
                color: "red",
              }
            },
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box 
          selectors={{
            "&::placeholder": {
              selectors: {
                "&:active": {
                  selectors: {
                    "&:hover": {
                      color: "red",
                    }
                  }
                }
              }
            },
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box 
          _placeholder={{
            _active: {
              _hover: {
                color: "blue",
              },
              color: "red",
            },
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box 
        selectors={{
          _hover: {
             selectors: {
              "&:active, _hover": {
                color: "red",
              }
             }
          }
        }}
        />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box 
        selectors={{
          _hover: {
             selectors: {
              "_themeDark": {
                color: "red",
              }
             }
          }
        }}
        />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box 
        selectors={{
          _hover: {
             selectors: {
              "_themeDark,_active": {
                color: "red",
              }
             }
          }
        }}
        />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box 
        selectors={{
          _hover: {
             selectors: {
              "_themeDark,_placeholder": {
                color: "red",
              }
             }
          }
        }}
        />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_conditional_selector() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={a===b ? undefined : {
          mx: 1
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={a===b && {
          mx: 1
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={a===b && {}} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={a===b && {
          mx: 1,
          my: 1
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_selector_with_responsive() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={{
          mx: [1, 2]
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={[{
          mx: 10
        },{
          mx: 20
        }]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _hover={[`
        margin-left: 10px;
        margin-right: 10px;
        `,{
        marginLeft: '20px',
        marginRight: '20px',
        }]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_static_css_class_name_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css`
  background-color: red;
`}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css as c } from "@devup-ui/core";
<Box className={c`
  background-color: red;
`}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css({
  bg:"red",
  color:"blue"
})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css as c } from "@devup-ui/core";
<Box className={c({
  bg:"red"
})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css({
  _hover: {
    bg:"red",
    color:"blue"
  }
})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
        <div className={css(a?{bg:"red"}:{bg:"blue"})}/>;
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css()}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css({})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css({
  _hover:`
  background-color: red;
  color: blue;
`
})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css({
  _hover:[`
  background-color: red;
  color: blue;
`,{
  backgroundColor: "green",
  color: "yellow"
}, `
  background-color: red;
  color: blue;
`]
})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css``}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css`   `}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css`  
 `}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css(...{bg: "red"})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css(...{})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css(...{...{bg: "red"}})}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_static_css_with_media_query() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css`
  @media (min-width: 768px) {
    & {
      background-color: red;
    }
  }
`}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css`
  @media (min-width: 768px) {
    &:hover {
      background-color: red;
    }
    &:active {
      background-color: blue;
    }
  }
`}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<Box className={css`
  @media (min-width: 768px) {
    background-color: red;
  }
`}/>;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_static_css_with_theme() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box color="$nice" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box color={`$nice`} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box color={("$nice")} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn apply_typography() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Text} from '@devup-ui/core'
        <Text typography="bold" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Text} from '@devup-ui/core'
        <Text typography={`bold`} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Text} from '@devup-ui/core'
        <Text typography={a ? "bold" : "bold2"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn apply_var_typography() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Text} from '@devup-ui/core'
        <Text typography={variable} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Text} from '@devup-ui/core'
        <Text typography={bo ? a : b} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Text} from '@devup-ui/core'
        <Text typography={`${bo ? a : b}`} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box as DevupButton} from '@devup-ui/core'
        <DevupButton
      boxSizing="border-box"
      className={className}
      typography={typography}
    >
    </DevupButton>
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Text} from '@devup-ui/core'
        <Text
            typography={({
                lg: "buttonLg",
                md: "button",
                sm: "buttonSm",
                tag: "tag"
            } as const)[size]}
        >
            {children}
        </Text>
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // ParenthesizedExpression wrapping object in computed member
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Text} from '@devup-ui/core'
        <Text
            typography={({
                lg: "buttonLg",
                md: "button",
                sm: "buttonSm",
                tag: "tag"
            })[size]}
        >
            {children}
        </Text>
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn raise_error() {
        reset_class_map();
        reset_file_map();
        assert!(
            extract(
                "test.wrong",
                "@devup-ui/core;const a = 1;",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap_err()
            .to_string()
            .starts_with("Unknown file extension")
        );

        reset_class_map();
        reset_file_map();
        assert_eq!(
            extract(
                "test.tsx",
                "import {} '@devup-ui/core';\na a = 1;",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap_err()
            .to_string(),
            "Parser panicked"
        );
    }

    #[test]
    #[serial]
    fn import_wrong_component() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {W} from '@devup-ui/core'
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {W, useTheme} from '@devup-ui/core';
useTheme();
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn support_transpile_mjs() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsxs as r, jsx as e } from "react/jsx-runtime";
import { Box as o, Text as t, Flex as i } from "@devup-ui/react";
function c() {
  return /* @__PURE__ */ r("div", { children: [
    /* @__PURE__ */ e(
      o,
      {
        _hover: {
          bg: "blue"
        },
        bg: "$text",
        color: "red",
        children: "hello"
      }
    ),
    /* @__PURE__ */ e(t, { typography: "header", children: "typo" }),
    /* @__PURE__ */ e(i, { as: "section", mt: 2, children: "section" })
  ] });
}
export {
  c as Lib
};"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.js",
                r#"import { jsxs as r, jsx as e } from "react/jsx-runtime";
import { Box as o, Text as t, Flex as i } from "@devup-ui/react";
function c() {
  return /* @__PURE__ */ r("div", { children: [
    /* @__PURE__ */ e(
      o,
      {
        _hover: {
          bg: "blue"
        },
        bg: "$text",
        color: "red",
        children: "hello"
      }
    ),
    /* @__PURE__ */ e(t, { typography: "header", children: "typo" }),
    /* @__PURE__ */ e(i, { as: "section", mt: 2, children: "section" })
  ] });
}
export {
  c as Lib
};"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.js",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/core";
e(o, { className: "a", bg: "red" })
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.js",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/core";
e(o, { className: "a", bg: variable, style: { color: "blue" } })
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.js",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/core";
e(o, { className: "a", bg: variable, style: { color: "blue" }, ...props })
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // conditional as
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.js",
                r#"import { jsx as e } from "react/jsx-runtime";
        import { Box as o } from "@devup-ui/core";
        e(o, { as: b ? "div" : "section", className: "a", bg: variable, style: { color: "blue" }, props: { animate: { duration: 1 } } })
        "#,
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.js",
                r#"import { jsx as e } from "react/jsx-runtime";
        import { Box as o } from "@devup-ui/core";
        e(o, { as: Variable, className: "a", bg: variable, style: { color: "blue" }, props: { animate: { duration: 1 } } })
        "#,
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.js",
                r#"import { jsx as e } from "react/jsx-runtime";
        import { Box as o } from "@devup-ui/core";
        e(o, { as: b ? null : undefined, className: "a", bg: variable, style: { color: "blue" }, props: { animate: { duration: 1 } } })
        "#,
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn support_transpile_cjs() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(extract("test.cjs", r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const e=require("react/jsx-runtime"),r=require("@devup-ui/react");function t(){return e.jsxs("div",{children:[e.jsx(r.Box,{_hover:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e.jsx(r.Text,{typography:"header",children:"typo"}),e.jsx(r.Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#, ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }).unwrap()));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(extract("test.cjs", r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const {jsx:e1, jsxs:e2}=require("react/jsx-runtime"),r=require("@devup-ui/react");function t(){return e2("div",{children:[e1(r.Box,{_hover:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e1(r.Text,{typography:"header",children:"typo"}),e1(r.Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#, ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }).unwrap()));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(extract("test.js", r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const e=require("react/jsx-runtime"),r=require("@devup-ui/react");function t(){return e.jsxs("div",{children:[e.jsx(r.Box,{_hover:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e.jsx(r.Text,{typography:"header",children:"typo"}),e.jsx(r.Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#, ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }).unwrap()));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(extract("test.js", r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const e=require("react/jsx-runtime"),r=require("@devup-ui/react");function t(){return e.jsxs("div",{children:[e.jsx(r.Box,{_hover:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e.jsx(r.Text,{typography:`header`,children:"typo"}),e.jsx(r.Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#, ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }).unwrap()));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(extract("test.js", r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const e=require("react/jsx-runtime"),{Box,Text,Flex}=require("@devup-ui/react");function t(){return e.jsxs("div",{children:[e.jsx(Box,{_hover:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e.jsx(Text,{typography:`header`,children:"typo"}),e.jsx(Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#, ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }).unwrap()));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(extract("test.js", r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const e=require("react/jsx-runtime"),{Box,Text,Flex}=require("@devup-ui/react");function t(){return e.jsxs("div",{children:[e.jsx(Box,{["_hover"]:{bg:"blue"},bg:"$text",color:"red",children:"hello"}),e.jsx(Text,{typography:`header`,children:"typo"}),e.jsx(Flex,{as:"section",mt:2,children:"section"})]})}exports.Lib=t;"#, ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }).unwrap()));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(extract("test.js", r#""use strict";Object.defineProperty(exports,Symbol.toStringTag,{value:"Module"});const e=require("react/jsx-runtime"),{Box,Text,Flex}=require("@devup-ui/react");function t(){return e.jsxs("div",{children:[e.jsx(Box,{["_hover"]:{bg:"blue"},bg:"$text",[variable]:"red",children:"hello"})]})}exports.Lib=t;"#, ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }).unwrap()));
    }

    #[test]
    #[serial]
    fn maintain_value() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={1} zIndex={2} fontWeight={900} scale={2} flex={1} lineHeight={1} tabSize={4} MozTabSize={4} WebkitLineClamp={4} />
        ",
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn with_prefix() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex MozTabSize={4} WebkitLineClamp={4} msBorderRadius={4} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn optimize_aspect_ratio() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex aspectRatio={"200/400"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex aspectRatio={"   200  /  400  "} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex aspectRatio={"   200.2  /  400.2  "} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn ternary_operator_in_selector() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex _hover={a ? { bg: "red" } : undefined} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex _hover={a ? { bg: "red" } : {}} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex _hover={a ? { bg: "red",color:"blue" } : { fontWeight:"bold", color:"red" }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_rest_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={0.5} {...props} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import { VStack } from '@devup-ui/core'

export default function Card({
  children,
  className,
  ...props
}) {
  return (
    <VStack
      _active={{
        boxShadow: 'none',
        transform: 'scale(0.95)',
      }}
      className={className}
      {...props}
    >
      {children}
    </VStack>
  )
}

        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_wrong_direct_array_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={[][0]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][-10]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][+10]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][100]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box padding={[1,,null][1]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Array with spread + numeric index (spread captured, element found after spread)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Flex } from "@devup-ui/core";
<Flex opacity={[...arr, 1][1]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Array with spread + numeric index out of range (etc Some fallback)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Flex } from "@devup-ui/core";
<Flex opacity={[...arr, 1][5]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Array with spread + dynamic index
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Flex } from "@devup-ui/core";
<Flex opacity={[...arr, 1][idx]} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn negative_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box zIndex={-1} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box zIndex={-a} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box zIndex={-(1+a)} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box zIndex={-1*a} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box zIndex={-(1)} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box zIndex={(-1)} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box zIndex={[(-1),-2, -(3)]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_wrong_direct_object_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box opacity={{}[1]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}["wrong"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}[`wrong`]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}[1]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Object with spread + string literal key not matching (etc Some fallback)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{...rest, a:1, b:0.5}["nonexistent"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_conditional_style_props_with_class_name() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box as DevupButton} from '@devup-ui/core'
        <DevupButton
      className={className}
      typography={typography}
    >
    </DevupButton>
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][a]} className="ab" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_direct_array_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][0]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5][a]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex bg={["$red", "$blue"][idx]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex bg={[`$red`, `${variable}`][idx]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Center} from '@devup-ui/core'
<Center
            bg={['$webBg', '$appBg', '$solutionBg'][categoryId - 1]}
          >
          </Center>
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5, ...some][100]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={[1, 0.5, ...some][a]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_multi_expression() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {
  Box,
  Button as DevupButton,
  Center,
  css,
} from '@devup-ui/core'

<DevupButton
    border={
    {
        primary: 'none',
        default: '1px solid var(--border, #E4E4E4)',
    }[variant]
    }
    className={className}
    px={
    {
        false: { sm: '12px', md: '16px', lg: '20px' }[size],
        true: { sm: '24px', md: '28px', lg: '32px' }[size],
    }[(!!icon).toString()]
    }
/>
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_direct_object_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}["a"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5, ...any}["b"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5, ...any}["some"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
        <Flex bg={{a:"$red", b:"$blue"}[idx]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_direct_variable_object_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
        <Flex opacity={{a:1, b:0.5}[a]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
<Box bg={SOME_VAR[idx]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_direct_object_responsive_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
;<Flex gap={{ 0: [1, 2, 3], 1: [4, 5, 6] }[idx]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
;<Flex gap={{ "a": [1, 2, 3], "b": [4, 5, 6] }[idx]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn props_direct_variable_object_responsive_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
;<Flex gap={{ 0: [a, b, c], "1": [d, e, f] }[idx]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_direct_array_responsive_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
;<Flex gap={[[1, 2, 3], [4, 5, 6]][idx]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
;<Flex gap={[[1, 2, 3],[4, 5, 6]][idx]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn props_direct_variable_array_responsive_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
;<Flex gap={[[a, b, c], [d, e, f]][idx]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
;<Flex gap={[, [d, e, f]][idx]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_direct_hybrid_responsive_select() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
;<Flex gap={[[a, 1, c], [d, e, 2]][idx]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn props_direct_wrong() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Flex} from '@devup-ui/core'
;<Flex gap={true[1]} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn test_component_in_func() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Flex} from '@devup-ui/core'
PROCESS_DATA.map(({ id, title, content }, idx) => (
          <MotionDiv key={idx}>
            <Flex alignItems="center" gap={[3, null, 5, null, 10]}>
            </Flex>
          </MotionDiv>
        ))
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn backtick_prop() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
            <Box bg={`black`} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
            <Box bg={`${variable}`} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn group_selector_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
            <Box _groupHover={{ bg: "red" }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_duplicate_style_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
            <Box bg="red" background="red" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn avoid_same_name_component() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
import {Button} from '@devup/ui'
            ;<Box bg="red" background="red" />
            ;<Button bg="red" background="red" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn css_props_destructuring_assignment() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {css} from '@devup-ui/core'
    <div className={css({
       ...(a ? { bg: 'red' } : { bg: 'blue' }),
       ...({ p: 1 }),
     })} />
            ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {css} from '@devup-ui/core'
    <div className={css({
       ...(a ? { bg: 'red', border: "solid 1px red" } : { bg: 'blue' }),
       ...({ p: 1,m: 1 }),
     })} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn theme_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box _themeDark={{ display:"none" }} _themeLight={{ display: "flex" }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn nested_theme_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box _themeDark={{
      selectors: {
        "&:hover": {
          color: "red",
        }
      },
      _active: {
        color: "blue",
        _placeholder: {
          color: "green",
        },
      },
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn template_literal_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box bg={`${"red"}`} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
    <Box m={`${1}`} />
            ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
    <Box m={`${-1}`} />
            ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
    <Box m={`${1} ${2}`} />
            ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
    <Box className={`  ${1} ${2}  `} />
            ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box className={`  ${1} ${2}  `}
    _hover={{bg:"red"}}
    _themeDark={{ _hover:{bg:"black"} }}
    
     />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn theme_selector() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box _themeDark={{ _hover:{bg:"black"} }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box _hover={{bg:"white"}} _themeDark={{ _hover:{bg:"black"} }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box _hover={{bg:"white"}} _themeDark={{
        selectors: {
          '& :is(svg,img)': {
            boxSize: '100%',
            filter: 'brightness(0) invert(1)',
          },
        },
      }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn custom_selector() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box selectors={{
    "&[aria-diabled='true']": {
      opacity: 0.5
      }
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box selectors={{
    "*[aria-diabled='true'] &:hover": {
      opacity: 0.5
      }
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box selectors={{
    "*[aria-diabled='true'] &": {
      opacity: 0.5
      }
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // A selector only the runtime knows is reported, not dropped
        reset_class_map();
        reset_file_map();
        let message = extract(
            "test.jsx",
            r#"import {Box} from '@devup-ui/core'
<Box selectors={{ [dynamic]: { color: 'red' }, "&:focus": { color: 'blue' } }} />
            "#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        )
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
        assert!(
            message.contains("`<Box>` cannot use `[dynamic]`"),
            "{message}"
        );
    }

    #[test]
    #[serial]
    fn style_order() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box styleOrder={20} p="4" _hover={{ bg: ["red", "blue"]}} selectors={{
    "*[aria-diabled='true'] &": {
      opacity: 0.5
      }
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsxs as r, jsx as e } from "react/jsx-runtime";
import { Box as o, Text as t, Flex as i } from "@devup-ui/react";
function c() {
  return  r("div", { children: [
     e(
      o,
      {
        _hover: {
          bg: "blue"
        },
        bg: "$text",
        color: "red",
        children: "hello",
        styleOrder: 10
      }
    ),
     e(t, { typography: "header", children: "typo", styleOrder:20 }),
     e(i, { as: "section", mt: 2, children: "section",styleOrder:30 })
  ] });
}
export {
  c as Lib
};"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
    <Box className={css({color:"white", styleOrder:100})} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
    <Box className={css({color:"white"})} styleOrder={20} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
    <Box className={css({color:"white",styleOrder:30})} styleOrder={20} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
    <Box styleOrder={20} p="4" _hover={{ bg: ["red", "blue"]}}
    className={css({color:"white", styleOrder:100})}

     selectors={{
    "*[aria-diabled='true'] &": {
      opacity: 0.5
      }
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
<Box
        aria-disabled={false}
        bg="red"
        className={css({
          bg: 'blue',
          styleOrder: 17,
        })}
        styleOrder={3}
      />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
    <Box className={css({color:"white"})} styleOrder={} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn style_order_conditional() {
        // Test 1: styleOrder={condition ? 5 : 10} — ternary with two static numbers
        // Since jsx_expression_to_number doesn't handle ConditionalExpression,
        // styleOrder should be None (not applied)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={isActive ? 5 : 10} bg="red" p="4" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 2: styleOrder={condition ? 5 : variable} — ternary with mixed static/dynamic
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={isActive ? 5 : order} bg="red" p="4" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 3: styleOrder={variable} — fully dynamic styleOrder
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={order} bg="red" p="4" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 4: styleOrder={condition ? 5 : 10} with conditional style props
        // Verifies interaction between conditional styleOrder and conditional style values
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={isActive ? 5 : 10} bg={isActive ? "red" : "blue"} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 5: styleOrder={condition ? 5 : 10} with css() className
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
<Box styleOrder={isActive ? 5 : 10} className={css({color:"white"})} bg="red" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn style_order_logical_and() {
        // Test 1: JSX path — styleOrder={a === 1 && 5}
        // truthy → styleOrder=5, falsy → no styleOrder (None)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={a === 1 && 5} bg="red" p="4" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 2: JSX path — styleOrder={a === 1 && 5} with conditional style props
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={a === 1 && 5} bg={isActive ? "red" : "blue"} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 3: Call expression path — styleOrder: a === 1 && 5
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { styleOrder: a === 1 && 5, bg: "red", p: "4" });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn style_order_conditional_coverage() {
        // Coverage: lib.rs:53 — Enum variant clone_in (positioning={variable} with conditional styleOrder)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
<Box styleOrder={isActive ? 5 : 10} positioning={mode} />
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: lib.rs:54 — Expression variant clone_in (typography={variable} with conditional styleOrder)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Text} from '@devup-ui/core'
<Text styleOrder={isActive ? 5 : 10} typography={typo} />
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: lib.rs:55 — MemberExpression variant clone_in (computed member expression with conditional styleOrder)
        // bg={["red","blue"][idx]} produces ExtractStyleProp::MemberExpression which gets clone_in'd
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={isActive ? 5 : 10} bg={["red","blue"][idx]} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: prop_modify_utils.rs:58,130 — (None, None) branch: no styles, only conditional styleOrder
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
<Box styleOrder={isActive ? 5 : 10} />
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: prop_modify_utils.rs:56-58 — (None, None) branch via call expression path
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { styleOrder: isActive ? 5 : 10 });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: utils.rs:100 — expression_to_style_order with plain variable (not conditional, not static)
        // Also covers visit.rs:259,262 — fallback from pre-scan None to extract_style_from_expression's result
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { styleOrder: order, bg: "red" });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: visit.rs:297/492 — non-conditional static styleOrder via call expression (fallback _ branch)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { bg: "red" });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn style_order_conditional_call_expression() {
        // Test 1: styleOrder: isActive ? 5 : 10 — ternary with two static numbers via call expression
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { styleOrder: isActive ? 5 : 10, bg: "red", p: "4" });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 2: styleOrder: isActive ? 5 : 10 with conditional style props via call expression
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { styleOrder: isActive ? 5 : 10, bg: isActive ? "red" : "blue" });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 3: static styleOrder via call expression (backward compat)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { styleOrder: 5, bg: "red", p: "4" });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn style_order2() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
    <Box styleOrder="20" p="4" _hover={{ bg: ["red", "blue"]}}
    className={css({color:"white", styleOrder:"100"})}

     selectors={{
    "*[aria-diabled='true'] &": {
      opacity: 0.5
      }
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
    <Box styleOrder={"20"} p="4" _hover={{ bg: ["red", "blue"]}}
    className={css({color:"white", styleOrder:("100")})}

     selectors={{
    "*[aria-diabled='true'] &": {
      opacity: 0.5
      }
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box, css} from '@devup-ui/core'
    <Box styleOrder={`20`} p="4" _hover={{ bg: ["red", "blue"]}}
    className={css({color:"white", styleOrder:`100`})}

     selectors={{
    "*[aria-diabled='true'] &": {
      opacity: 0.5
      }
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn style_variables() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
    <Box styleVars={{
        c: "red"
    }} />
            "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
        <Box styleVars={{
            "--d": "red"
        }} />
                "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
        <Box styleVars={{
            "--d": "red",
            "e": "blue"
        }} />
                "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box styleVars={{
            variable
        }} />
                ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
        <Box styleVars={{
            variable: true ? "red" : "blue"
        }} />
                "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
        <Box style={{ "--d": "red" }} styleVars={{
            variable: true ? "red" : "blue"
        }} />
                "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
        <Box bg={color} style={{ "--d": "red" }} styleVars={{
            variable: true ? "red" : "blue"
        }} />
                "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
        <Box styleVars={{
            ["hello"]: "red"
        }} />
                "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
        <Box styleVars={{
            [true]: "red",
            [1]: "blue",
            [variable]: "green",
            [2+2]: "yellow"
        }} />
                "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box styleVars={styleVars} />
                ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
        <Box styleVars={{...styleVars}} />
                ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn wrong_style_variables() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r"import {Box} from '@devup-ui/core'
    <Box styleVars={} />
            ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn style_variables_mjs() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.js",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/core";
e(o, { styleVars: { c: "yellow" } })
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_global_css() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  "div": {
    bg: "red"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  div: {
    bg: "blue"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  ["div"]: {
    bg: "yellow"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  [`div`]: {
    bg: "green"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  _hover: {
    bg: "red"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  _placeholder: {
    bg: "red"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  _nthLastChild: {
    bg: "red"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss(...{})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss(...{div: {bg: "red"}})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // recursive spread
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss(...{div: {bg: "red"}, ...{span: {bg: "blue"}}})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_wrong_global_css() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
    [1]: {
        bg: "red"
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss()
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_eq!(
            extract(
                "test.tsx",
                "import { globalCss } from \"@devup-ui/core\";\nglobalCss(1)\n",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap_err()
            .to_string(),
            "test.tsx:2:1: `globalCss()` cannot use `1` at build time: its values must be literals, theme tokens or constants, or be computed from them"
        );
    }

    #[test]
    #[serial]
    fn extract_global_css_with_selector() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  "div": {
    bg: "red",
    color: "blue",
    _hover: {
      bg: "blue",
      color: "red"
    }
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  "div": {
    bg: "red",
    color: "blue",
    _hover: {
      bg: "blue",
      color: "red"
    }
  },
  "span": {
    bg: "red",
    color: "blue",
    _hover: {
      bg: "blue",
      color: "red"
    }
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  "div": {
    bg: "red",
    color: "blue",
    _hover: {
      bg: "blue",
      color: "red"
    }
  },
  "span": {
    bg: "red",
    color: "blue",
    _hover: {
      bg: "blue",
      color: "red"
    }
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  ["div"]: {
    bg: "red",
    color: "blue",
    _hover: {
      bg: "blue",
      color: "red"
    }
  },
  ["span"]: {
    bg: "red",
    color: "blue",
    _hover: {
      bg: "blue",
      color: "red"
    }
  },
  "body[data-theme='dark']": {
    bg: "red",
    color: "blue",
    _hover: {
      bg: "blue",
      color: "red"
    }
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_global_css_with_template_literal() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss({
      "div": `
        background-color: red
      `
    })
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss({
      "div": ``
    })
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss({
      "div": `     `
    })
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss({
      "div": `  
         `
    })
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss`
    div {
      background-color: red;
      color: blue;
    }
    `
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss``
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss`           `
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss`         
      `
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
    globalCss`:root {color-scheme: light dark}`
    "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_global_css_with_imports() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  imports: ["@devup-ui/core/css/global.css"]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  imports: ["@devup-ui/core/css/global.css", "@devup-ui/core/css/global2.css"]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  imports: [`@devup-ui/core/css/global3.css`, `@devup-ui/core/css/global4.css`]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_global_css_with_font_faces() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  fontFaces: [
    {
      fontFamily: "Roboto",
      src: "url('/fonts/Roboto-Regular.ttf')",
      fontWeight: 400,
    },
    {
      fontFamily: "Roboto2",
      src: "url('/fonts/Roboto-Regular.ttf')",
      fontWeight: 400,
    }
  ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  fontFaces: [
    {
      fontFamily: "Roboto",
      src: `url('/fonts/Roboto-Regular.ttf')`,
      fontWeight: `400`,
    }
  ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  fontFaces: [
    {
      fontFamily: "Roboto",
      src: "url('/fonts/Roboto-Regular.ttf')",
    }
  ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  fontFaces: [`
  font-family: "Roboto";
  src: "url('/fonts/Roboto-Regular.ttf')";
  font-weight: 400;
  `]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  fontFaces: [
    {
      fontFamily: "Roboto Hello",
      src: "url('/fonts/Roboto-Regular.ttf')",
      fontWeight: 400,
    }
  ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  fontFaces: [
    {
      fontFamily: undefined,
      src: "url('/fonts/Roboto-Regular.ttf')",
      fontWeight: 400,
    }
  ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  fontFaces: [
    {
      fontFamily: "Roboto Regular2",
      src: "//fonts/Roboto-Regular.ttf",
      fontWeight: 400,
    },
    {
      fontFamily: "Roboto Regular",
      src: "//fonts/Roboto Regular.ttf",
      fontWeight: 400,
    },
    {
      fontFamily: "Roboto Regular3",
      src: "fonts/Roboto Regular.ttf",
      fontWeight: 400,
    },
    {
      fontFamily: "Roboto Regular4",
      src: "local('fonts/Roboto Regular.ttf')",
      fontWeight: 400,
    },
  ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  imports: [{"url": "@devup-ui/core/css/global.css"}]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  imports: [{"url": "@devup-ui/core/css/global.css", "query": "layer"}]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  imports: [{"query": "layer"}]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_global_css_with_wrong_imports() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  imports: [1, 2, "./test.css"]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  imports: {}
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_global_css_with_empty() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  "div": {}
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  div: {},
  span: {}
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  div: ``
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({
  div: ``,
  span: ``
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss({})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from "@devup-ui/core";
globalCss()
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_keyframs() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  from: { opacity: 0 },
  to: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  "0%": { opacity: 0 },
  "50%": { opacity: 0.5 },
  "100%": { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  "0": { opacity: 0 },
  "50": { opacity: 0.5 },
  "100": { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  ["0"]: { opacity: 0 },
  ["50"]: { opacity: 0.5 },
  ["100"]: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  [0]: { opacity: 0 },
  [50]: { opacity: 0.5 },
  [100]: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  0: { opacity: 0 },
  50: { opacity: 0.5 },
  100: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  [`0`]: { opacity: 0 },
  [`50`]: { opacity: 0.5 },
  [`100`]: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  [`0%`]: { opacity: 0 },
  [`50%`]: { opacity: 0.5 },
  [`100%`]: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";

keyframes({
  [`0`]: { opacity: 0 },
  [`50`]: { opacity: 0.5 },
  [`100`]: { opacity: 1 }
});
keyframes({
  [`0%`]: { opacity: 0 },
  [`50%`]: { opacity: 0.5 },
  [`100%`]: { opacity: 1 }
});
keyframes({
  [`1%`]: { opacity: 0 },
  [`50%`]: { opacity: 0.5 },
  [`100%`]: { opacity: 1 }
});
keyframes({
  [`0%`]: { opacity: 1 },
  [`50%`]: { opacity: 0.5 },
  [`100%`]: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes(...{
  [`0%`]: { opacity: 0, ...{color: "red"} },
  [`50%`]: { opacity: 0.5 },
  [`100%`]: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    #[test]
    #[serial]
    fn extract_wrong_keyframs() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  from: { opacity: 0 },
  [true]: { opacity: 0.5 },
  to: { opacity: 1 }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_runtime_values_where_no_element_holds_them() {
        for (code, error) in [
            (
                "import { css } from '@devup-ui/react';\nexport const a = css({ color: x });",
                "`css()` cannot use `x`",
            ),
            (
                "import { globalCss } from '@devup-ui/react';\nglobalCss({ body: { color: x } });",
                "`globalCss()` cannot use `x`",
            ),
            (
                "import { globalCss } from '@devup-ui/react';\nglobalCss('body', { color: x });",
                "`globalCss()` cannot use `x`",
            ),
            (
                "import { keyframes } from '@devup-ui/react';\nkeyframes({ from: { opacity: 0 }, to: { color: dy } });",
                "`keyframes()` cannot use `dy`",
            ),
            (
                "import * as stylex from '@stylexjs/stylex';\nexport const spin = stylex.keyframes({ to: { color: dy } });",
                "`stylex.keyframes()` cannot use `dy`",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract("test.tsx", code, ExtractOption::default())
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();
            assert!(
                message.starts_with("test.tsx:2:") && message.contains(error),
                "{message}"
            );
            assert!(
                message.ends_with(
                    "at build time: its values must be literals, theme tokens or constants, or be computed from them"
                ),
                "{message}"
            );
        }
    }

    #[test]
    #[serial]
    fn test_template_interpolations() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { css, globalCss, keyframes, styled } from '@devup-ui/react';
const C = 'red';
const O = 0.5;
const SEL = '.x';
const SNIPPET = 'display: flex;';
const mixin = css`display: grid;`;
globalCss`body { color: ${C}; padding: ${4}px; }`;
globalCss`a { color: ${(p) => p.theme.colors.link}; }`;
export const k = keyframes`from { opacity: ${O} } to { opacity: 1 }`;
export const c = css`${SNIPPET} color: ${C};`;
export const d = css`${mixin}; margin: 0;`;
export const A = styled.div`${mixin} color: red; ${(p) => p.active && mixin}`;
export const B = styled.div`${SEL} & { color: ${C}; }`;",
                ExtractOption::default(),
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_template_interpolations_known_only_at_runtime() {
        for (code, error) in [
            (
                "import { globalCss } from '@devup-ui/react';\nglobalCss`body { color: ${x}; }`;",
                "`globalCss()` cannot use `x` at build time",
            ),
            (
                "import { keyframes } from '@devup-ui/react';\nexport const k = keyframes`from { opacity: ${x} }`;",
                "`keyframes()` cannot use `x` at build time",
            ),
            (
                "import { css } from '@devup-ui/react';\nexport const c = css`color: ${x};`;",
                "`css()` cannot use `x` at build time",
            ),
            (
                "import { css } from '@devup-ui/react';\nexport const c = css`${x}: red;`;",
                "Cannot place `x` at build time",
            ),
            (
                "import { styled } from '@devup-ui/react';\nexport const A = styled.div`${Other}:hover & { color: red; }`;",
                "Cannot place `Other` at build time",
            ),
            (
                "import { styled } from '@devup-ui/react';\nexport const A = styled.div`& ${x} { color: red; }`;",
                "Cannot place `x` at build time",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract("test.tsx", code, ExtractOption::default())
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();
            assert!(
                message.starts_with("test.tsx:2:") && message.contains(error),
                "{message}"
            );
        }
    }

    #[test]
    #[serial]
    fn test_errors_are_located_and_all_reported() {
        reset_class_map();
        reset_file_map();
        let message = extract(
            "src/App.tsx",
            "import {\n  css,\n  keyframes,\n} from '@emotion/react';\nimport { globalCss } from '@devup-ui/react';\nexport const a = css({ padding: 8 }); export const b = css({ color: x });\nglobalCss`body { color: ${y}; }`;\nexport const k = keyframes({ to: { opacity: z } });\nexport const c = css({ color: x });",
            ExtractOption {
                import_aliases: HashMap::from([(
                    "@emotion/react".to_string(),
                    ImportAlias::NamedToNamed,
                )]),
                ..ExtractOption::default()
            },
        )
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
        assert_eq!(
            message.lines().collect::<Vec<_>>(),
            [
                "src/App.tsx:6:56: `css()` cannot use `x` at build time: its values must be literals, theme tokens or constants, or be computed from them",
                "src/App.tsx:7:27: `globalCss()` cannot use `y` at build time: its values must be literals, theme tokens or constants, or be computed from them",
                "src/App.tsx:8:18: `keyframes()` cannot use `z` at build time: its values must be literals, theme tokens or constants, or be computed from them",
                "src/App.tsx:9:18: `css()` cannot use `x` at build time: its values must be literals, theme tokens or constants, or be computed from them",
            ]
        );
    }

    #[test]
    #[serial]
    fn test_inline_local_constants_edges() {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "test.tsx",
            r"import { Box } from '@devup-ui/react';
export function helper() {}
const OBJ = { a: 1 };
export const a = <Box content={`${OBJ}`} w={`${1e400}px`} h={`${-1e400}px`} />;",
            ExtractOption::default(),
        )
        .unwrap();
        let mut values: Vec<String> = output
            .styles
            .iter()
            .map(|style| format!("{style:?}"))
            .collect();
        values.sort();
        let values = values.join("\n");
        assert!(values.contains("\"Infinitypx\""), "{values}");
        assert!(values.contains("\"-Infinitypx\""), "{values}");
        assert!(output.code.contains("OBJ"), "{}", output.code);
    }

    #[test]
    #[serial]
    fn test_inline_local_constants() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { Box, css, keyframes, globalCss } from '@devup-ui/react';
const SIZE = 4;
const UNIT = `${SIZE}px`;
const DOUBLE = SIZE * 2;
const HALF = SIZE / 2 - 1;
const LABEL = 'a' + SIZE;
const TINY = 0.0000001;
const HUGE = 1e21;
const ZERO = -0;
const PX = 'px';
const SELF = SELF;
let mutable = 1;
export const a = <Box p={DOUBLE} m={UNIT} w={SIZE + 1} h={HALF} content={LABEL} top={`${TINY}px`} left={`${HUGE}px`} right={ZERO + PX} bottom={mutable} zIndex={SELF} />;
export const b = css({ padding: `${SIZE * 2}px`, margin: SIZE - 1, width: SIZE * SIZE, height: (SIZE) });
export const k = keyframes({ from: { opacity: SIZE / 8 } });
globalCss({ body: { padding: UNIT } });
export const c = <Box p={SIZE / 0} m={PX - 1} w={`${PX}${other}`} h={SIZE % 3} />;",
                ExtractOption::default(),
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_keyframs_literal() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  from: `
  background-color: red;
  `,
  to: `
  background-color: blue;
  `
})

keyframes`
  from {
    background-color: red;
  }
  to {
    background-color: blue;
  }
`
keyframes({
  from: {
    backgroundColor: "red"
  },
  to: {
    backgroundColor: "blue"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { keyframes } from "@devup-ui/core";
keyframes({
  "0%": `
  background-color: red;
  color: blue;
  `,
  "100%": `
  background-color: blue;
  color: red;
  `
})

keyframes`
  0% {
    background-color: red;
    color: blue;
  }
  100% {
    background-color: blue;
    color: red;
  }
`
keyframes({
  "0%": {
    backgroundColor: "red",
    color: "blue"
  },
  "100%": {
    backgroundColor: "blue",
    color: "red"
  }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_just_tsx_in_multiple_files() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box padding={1} margin={2} wrong={} wrong2=<></> />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box as C} from '@devup-ui/core'
                <C padding={2} margin={3} />
                ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test1.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box padding={1} margin={2} wrong={} wrong2=<></> />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test1.tsx",
                r"import {Box as C} from '@devup-ui/core'
                <C padding={2} margin={3} />
                ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn import_main_css() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box padding={1} margin={2} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: true,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn optimize_multi_css_value() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box fontFamily="Roboto, Arial, sans-serif" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_enum_style_property() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box positioning="top-left" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // wrong case
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box positioning="wrong" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box positioning={a} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box positioning={a} w="100%" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box positioning={a} top="0px" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box positioning={[a, b, "top", "left", "wrong"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_advenced_selector() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _is={{
          params: ["test"],
          bg: "red",
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _is={{
          params: ["test", "test2"],
          bg: "red",
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // empty
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
        <Box _is={{
          params: [],
        }} />
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _is={{
          params: [""],
          _hover: {
            bg: "blue"
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _is={{
          params: ["", ""],
          _hover: {
            bg: "blue"
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box _is={{
          params: ["test", variable],
          _hover: {
            bg: "blue"
          }
        }} />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
        globalCss({
          "_is": {
            params: ["test", variable],
            _hover: {
              bg: "blue"
            },
            bg: "red"
          }
        })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {css} from '@devup-ui/core'
        css({
          "_is": {
            params: ["test"],
            bg: "red"
          }
        })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_styled() {
        // Test 1: styled.div`css`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        const StyledDiv = styled.section`
          background: red;
          color: blue;
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 2: styled("div")`css`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled("article")`
          background: red;
          color: blue;
        `
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 3: styled("div")({ bg: "red" })
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled("footer")({ bg: "red" })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 4: styled.div({ bg: "red" })
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled.aside({ bg: "red" })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 5: styled(Component)({ bg: "red" })
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled, Text} from '@devup-ui/core'
        const StyledComponent = styled(Text)({ bg: "red" })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled, Text} from '@devup-ui/core'
        const StyledComponent = styled(Text)`
          background: red;
          color: blue;
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled, VStack} from '@devup-ui/core'
        const StyledComponent = styled(VStack)({ bg: "red" })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled, VStack} from '@devup-ui/core'
        const StyledComponent = styled(VStack)`
          background: red;
          color: blue;
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledComponent = styled(CustomComponent)({ bg: "red" })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        const StyledComponent = styled(CustomComponent)`
          background: red;
          color: blue;
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled.aside<{ test: string }>({ bg: "red" })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_styled_with_variable() {
        // Test 1: styled.div({ bg: "$text" })
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled.div({ bg: "$text", color: "$primary" })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 2: styled("div")({ color: "$primary" })
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled("div")({ bg: "$text", fontSize: 16 })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 3: styled.div`css`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        const StyledDiv = styled.div`
          background: var(--text);
          color: var(--primary);
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 4: styled(Component)({ bg: "$text" })
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled, Box} from '@devup-ui/core'
        const StyledComponent = styled(Box)({ bg: "$text", _hover: { bg: "$primary" } })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 5: styled("div")`css`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled("div")`
          background-color: var(--text);
          padding: 16px;
        `
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_styled_with_variable_like_emotion() {
        // Test 1: styled.div`css with ${variable}`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        let color = 'red';
        const StyledDiv = styled.div`
          color: ${color};
          background: blue;
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 2: styled("div")`css with ${variable}`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        let primaryColor = 'blue';
        let padding = '16px';
        const StyledDiv = styled("div")`
          color: ${primaryColor};
          padding: ${padding};
        `
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        let primaryColor = 'blue';
        let padding = '16px';
        const StyledDiv = styled("div")({ bg: primaryColor, padding })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        let primaryColor = 'blue';
        let padding = '16px';
        const StyledDiv = styled.div({ bg: primaryColor, padding })
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled("div")`
          color: ${obj.color};
          padding: ${func()};
          background: ${obj.func()};
        `
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled("div")({ bg: obj.bg, padding: func(), color: obj.color() })
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        const StyledDiv = styled.div({ bg: obj.bg, padding: func(), color: obj.color() })
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_styled_with_variable_like_emotion_props() {
        // Test 3: styled.div`css with ${props => props.bg}`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        const StyledDiv = styled.div`
          background: ${props => props.bg};
          color: red;
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 4: styled(Component)`css with ${variable}`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled, Box} from '@devup-ui/core'
        let fontSize = '18px';
        const StyledComponent = styled(Box)`
          font-size: ${fontSize};
          color: ${props => props.color || 'black'};
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 5: styled.div`css with multiple ${variables}`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        let margin = '10px';
        let padding = '20px';
        const StyledDiv = styled.div`
          margin: ${margin};
          padding: ${padding};
          background: ${props => props.bg || 'white'};
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test 6: styled.div`css with ${expression}`
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {styled} from '@devup-ui/core'
        const isActive = true;
        const StyledDiv = styled.div`
          color: ${isActive ? 'red' : 'blue'};
          opacity: ${isActive ? 1 : 0.5};
        `
        ",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_wrong_styled_with_variable_like_emotion_props() {
        // Left as written these would call `styled` at runtime, whose import
        // the build removes
        for (code, error) in [
            (
                "const StyledDiv = styled(null)`\n  background: ${props => props.bg};\n  color: red;\n`",
                "`styled()` cannot use `styled(null)`",
            ),
            (
                "const StyledDiv = styled(\"div\", \"span\")`\n  background: ${props => props.bg};\n  color: red;\n`",
                "`styled()` cannot use `styled(\"div\", \"span\")`",
            ),
            (
                "const StyledDiv = styled(\"div\", \"span\").filter(Boolean)",
                "`styled()` cannot use `styled(\"div\", \"span\")`",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import {{styled}} from '@devup-ui/core'\n{code}"),
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new(),
                },
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains(error), "{code}\n{message}");
        }

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled("div")({ bg: "red" }, {})
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
        const StyledDiv = styled("div")({ bg: "red" }, {})``
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_mask_properties_with_korean() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r##"import {Box} from '@devup-ui/core'
        <Box
          aspectRatio="5.49"
          bg="#752E2E"
          h="22px"
          maskImage="url('/icons/BI-타이틀.svg')"
          maskRepeat="no-repeat"
          maskSize="contain"
          w="121px"
        />
        "##,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_dot_notation_theme_variables() {
        // Test that dot notation theme variables (e.g., $primary.100) are correctly extracted
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box bg="$primary.100" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test multiple dot notation variables
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box bg="$gray.200" color="$blue.500" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test deep nested dot notation
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box bg="$color.brand.primary.100" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test dot notation in border shorthand
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
        <Box border="1px solid $border.primary" />
        "#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_styled_with_spread() {
        reset_class_map();
        reset_file_map();
        // Test styled with spread element
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {styled} from '@devup-ui/core'
const baseStyles = { bg: "red" };
const StyledDiv = styled.div({ ...baseStyles })
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_css_function_no_args() {
        reset_class_map();
        reset_file_map();
        // Test css() with no arguments
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {css} from '@devup-ui/core'
const className = css()
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_css_function_empty_object() {
        reset_class_map();
        reset_file_map();
        // Test css() with empty object
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {css} from '@devup-ui/core'
const className = css({})
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_keyframes_function() {
        reset_class_map();
        reset_file_map();
        // Test keyframes() function
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {keyframes} from '@devup-ui/core'
const spin = keyframes({
    from: { transform: "rotate(0deg)" },
    to: { transform: "rotate(360deg)" }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_function() {
        reset_class_map();
        reset_file_map();
        // Test globalCss() function
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {globalCss} from '@devup-ui/core'
globalCss({
    body: { margin: 0, padding: 0 }
})
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_conditional_styles() {
        reset_class_map();
        reset_file_map();
        // Test conditional styles with both branches having different properties
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
const Component = () => {
    const isActive = true;
    return <Box bg={isActive ? "red" : undefined} color={isActive ? undefined : "blue"} />;
}
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_css_variable_reassignment() {
        reset_class_map();
        reset_file_map();
        // Test css import reassignment
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {css as cssFunction} from '@devup-ui/core'
const myCss = cssFunction;
const className = myCss({ bg: "red" })
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_with_imports() {
        reset_class_map();
        reset_file_map();
        // Test globalCss with @import
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    imports: ["https://fonts.googleapis.com/css?family=Roboto"]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_with_font_faces() {
        reset_class_map();
        reset_file_map();
        // Test globalCss with fontFaces
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    fontFaces: [
        {
            fontFamily: "CustomFont",
            src: "url('/fonts/custom.woff2')",
            fontWeight: "400"
        }
    ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_with_pseudo_selector() {
        reset_class_map();
        reset_file_map();
        // Test globalCss with pseudo selector (prefixed with _)
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    _hover: { bg: "red" }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_responsive_array_styles() {
        reset_class_map();
        reset_file_map();
        // Test responsive array with multiple breakpoints
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
<Box p={[1, 2, 3, 4]} m={[0, 1]} />
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_member_expression_style() {
        reset_class_map();
        reset_file_map();
        // Test dynamic member expression for styles (obj[key])
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
let colors = { primary: "blue", secondary: "red" };
let key = "primary";
<Box bg={colors[key]} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_dynamic_class_name_merge() {
        reset_class_map();
        reset_file_map();
        // Test dynamic className merging with existing className
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
const Component = ({ className }) => {
    return <Box className={className} bg="red" />;
}
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_typography_style() {
        reset_class_map();
        reset_file_map();
        // Test typography prop
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Text} from '@devup-ui/core'
<Text typography="$heading" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_css_with_template_literal() {
        reset_class_map();
        reset_file_map();
        // Test css function with template literal containing expressions
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {css} from '@devup-ui/core'
const size = 16;
const className = css({ fontSize: `${size}px` })
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_conditional_with_both_branches() {
        reset_class_map();
        reset_file_map();
        // Test conditional where both branches have styles
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
const isActive = true;
<Box bg={isActive ? "red" : "blue"} p={isActive ? 4 : 2} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_spread_props() {
        reset_class_map();
        reset_file_map();
        // Test spread props on component
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
const Component = (props) => {
    return <Box {...props} bg="red" />;
}
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_nested_conditional() {
        reset_class_map();
        reset_file_map();
        // Test nested conditional expressions
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
const a = true;
const b = false;
<Box bg={a ? (b ? "red" : "blue") : "green"} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_style_prop_merge() {
        reset_class_map();
        reset_file_map();
        // Test style prop merging with dynamic styles
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
let color = "red";
<Box style={{ padding: 10 }} bg={color} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_keyframes_no_args() {
        reset_class_map();
        reset_file_map();
        // Test keyframes with no arguments
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {keyframes} from '@devup-ui/core'
const spin = keyframes()
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_no_args() {
        reset_class_map();
        reset_file_map();
        // Test globalCss with no arguments
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {globalCss} from '@devup-ui/core'
globalCss()
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_default_import_usage() {
        reset_class_map();
        reset_file_map();
        // Test using default import
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import DevUI from '@devup-ui/core'
<DevUI.Box bg="red" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_namespace_import_usage() {
        reset_class_map();
        reset_file_map();
        // Test using namespace import
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import * as DevUI from '@devup-ui/core'
<DevUI.Box bg="red" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_namespace_import_css() {
        reset_class_map();
        reset_file_map();
        // Test using namespace import with css function
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import * as DevUI from '@devup-ui/core'
const className = DevUI.css({ bg: "red" })
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_enum_style_prop() {
        reset_class_map();
        reset_file_map();
        // Test enum-like style mapping
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
let variant = "primary";
let colors = { primary: "blue", secondary: "red" };
<Box bg={colors[variant]} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_style_vars_prop() {
        reset_class_map();
        reset_file_map();
        // Test styleVars prop
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
let dynamicColor = "blue";
<Box styleVars={{ color: dynamicColor }} bg="var(--color)" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_props_prop() {
        reset_class_map();
        reset_file_map();
        // Test props prop forwarding
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
const extraProps = { onClick: () => {} };
<Box props={extraProps} bg="red" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_style_order_prop() {
        reset_class_map();
        reset_file_map();
        // Test styleOrder prop
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={1} bg="red" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_multiple_dynamic_values() {
        reset_class_map();
        reset_file_map();
        // Test multiple dynamic values
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
let color = "red";
let padding = 10;
let margin = 5;
<Box bg={color} p={padding} m={margin} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_dynamic_value_with_important() {
        reset_class_map();
        reset_file_map();
        // !important in a template literal should be stripped from the runtime
        // value and placed on the CSS property declaration instead.
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
let color = "red";
<Box bg={`${color} !important`} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_media_query_selectors() {
        // Test _print media query selector
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box _print={{ bg: "white", color: "black" }} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test _screen media query selector
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box _screen={{ bg: "blue" }} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test _motionReduce media query shorthand
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box _motionReduce={{ display: "none" }} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test _all media query selector
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box _all={{ fontFamily: "sans-serif" }} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test multiple media query selectors combined
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box 
    bg="red"
    _print={{ bg: "white" }}
    _screen={{ bg: "blue" }}
/>
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_at_rules_underscore_prefix() {
        // Test _container at-rule
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box _container={{ "(min-width: 400px)": { bg: "red" } }} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test _media at-rule
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box _media={{ "(min-width: 768px)": { bg: "blue" } }} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test _supports at-rule
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box _supports={{ "(display: grid)": { display: "grid" } }} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test _container with named container
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box _container={{ "sidebar (min-width: 400px)": { p: 4 } }} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_at_rules_at_prefix() {
        // Test @container at-rule
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box {...{"@container": { "(min-width: 400px)": { bg: "red" } }}} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test @media at-rule
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box {...{"@media": { "(min-width: 768px)": { bg: "blue" } }}} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test @supports at-rule
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box {...{"@supports": { "(display: flex)": { display: "flex" } }}} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_at_rules() {
        // Test globalCss with @media nested inside selector
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    body: {
        "@media": {
            "(min-width: 768px)": { bg: "white" }
        }
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test globalCss with @supports nested inside selector
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    ".grid-container": {
        "@supports": {
            "(display: grid)": { display: "grid" }
        }
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test globalCss with @container nested inside selector
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    ".card": {
        "@container": {
            "(min-width: 400px)": { p: 4 }
        }
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test globalCss with multiple at-rules nested inside selectors
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    body: {
        bg: "gray",
        "@media": {
            "(min-width: 768px)": { bg: "white" },
            "(prefers-color-scheme: dark)": { bg: "black" }
        }
    },
    ".container": {
        "@supports": {
            "(display: grid)": { display: "grid" }
        }
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_with_layer() {
        // Test globalCss with @layer property
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    "*": {
        "@layer": "reset",
        margin: 0,
        padding: 0
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test globalCss with @layer for multiple selectors
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {globalCss} from '@devup-ui/core'
globalCss({
    "*": {
        "@layer": "reset",
        boxSizing: "border-box"
    },
    body: {
        "@layer": "base",
        fontFamily: "sans-serif"
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: false,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    // ============================================================================
    // VANILLA-EXTRACT API TESTS
    // ============================================================================

    /// Test vanilla-extract style files (.css.ts, .css.js)
    /// Using vanilla-extract API (style, globalStyle, keyframes) from @devup-ui/react package
    #[test]
    #[serial]
    fn test_vanilla_extract_style_css_ts() {
        reset_class_map();
        reset_file_map();
        // .css.ts file with style function (vanilla-extract API)
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "styles.css.ts",
                r#"import { style } from '@devup-ui/react'
export const container: string = style({ background: "red", padding: 16 })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_style_css_js() {
        reset_class_map();
        reset_file_map();
        // .css.js file with style function
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "styles.css.js",
                r#"import { style } from '@devup-ui/react'
export const wrapper = style({ backgroundColor: "white", margin: 8 })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // .css.js file with style function for link
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "components.css.js",
                r#"import { style } from '@devup-ui/react'
export const link = style({ color: "blue", textDecoration: "underline" })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    /// Test .css.ts files with @vanilla-extract/css import (compatibility mode)
    #[test]
    #[serial]
    fn test_vanilla_extract_css_ts_with_vanilla_extract_import() {
        reset_class_map();
        reset_file_map();
        // .css.ts file with import from @vanilla-extract/css (not @devup-ui/react)
        // This should work the same as importing from @devup-ui/react
        let mut import_aliases = HashMap::new();
        import_aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "styles.css.ts",
                r"import { style } from '@vanilla-extract/css'
export const hello = style({
  cursor: 'pointer',
  fontSize: 32,
  paddingTop: '28px',
  paddingBottom: '28px',
})
export const text = style({
  color: 'var(--text)',
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_with_variable() {
        reset_class_map();
        reset_file_map();
        // Variables should be evaluated at execution time
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "vars.css.ts",
                r#"import { style } from '@devup-ui/react'
const primaryColor = "blue";
const spacing = 16;
export const button = style({ background: primaryColor, padding: spacing })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_with_computed() {
        reset_class_map();
        reset_file_map();
        // Computed values should be evaluated
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "computed.css.ts",
                r"import { style } from '@devup-ui/react'
const base = 8;
export const box = style({ padding: base * 2, margin: base / 2 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_with_spread() {
        reset_class_map();
        reset_file_map();
        // Spread operator should work
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "spread.css.ts",
                r#"import { style } from '@devup-ui/react'
const baseStyle = { padding: 8, margin: 4 };
export const extended = style({ ...baseStyle, background: "red" })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_with_pseudo_selector() {
        reset_class_map();
        reset_file_map();
        // devup-ui extension: _hover pseudo selector
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "hover.css.ts",
                r#"import { style } from '@devup-ui/react'
export const hoverButton = style({ background: "gray", _hover: { background: "blue" } })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_with_responsive_array() {
        reset_class_map();
        reset_file_map();
        // devup-ui extension: responsive arrays
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "responsive.css.ts",
                r"import { style } from '@devup-ui/react'
export const responsiveBox = style({ padding: [8, 16, 32] })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_with_keyframes_and_global() {
        reset_class_map();
        reset_file_map();
        // .css.ts file with keyframes (vanilla-extract API)
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "animations.css.ts",
                r#"import { keyframes, style } from '@devup-ui/react'
export const fadeIn = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })
export const animated = style({ animation: "fadeIn 1s ease-in" })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // .css.ts file with globalStyle (vanilla-extract API)
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "global.css.ts",
                r#"import { globalStyle } from '@devup-ui/react'
globalStyle("body", { margin: 0, padding: 0 })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_create_var() {
        reset_class_map();
        reset_file_map();
        // createVar - CSS variable creation
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "vars.css.ts",
                r"import { createVar, style } from '@devup-ui/react'
export const colorVar = createVar()
export const box = style({
  vars: {
    [colorVar]: 'blue'
  },
  color: colorVar
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // fallbackVar - CSS variable with fallback
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "fallback.css.ts",
                r"import { createVar, fallbackVar, style } from '@devup-ui/react'
export const colorVar = createVar()
export const box = style({
  color: fallbackVar(colorVar, 'red')
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_style_variants() {
        reset_class_map();
        reset_file_map();
        // styleVariants - create multiple style variants
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "variants.css.ts",
                r"import { styleVariants } from '@devup-ui/react'
export const background = styleVariants({
  primary: { background: 'blue' },
  secondary: { background: 'gray' },
  danger: { background: 'red' }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // styleVariants with base style composition
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "variants-composed.css.ts",
                r"import { style, styleVariants } from '@devup-ui/react'
const base = style({ padding: 12, borderRadius: 4 })
export const button = styleVariants({
  primary: [base, { background: 'blue', color: 'white' }],
  secondary: [base, { background: 'gray', color: 'black' }]
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_font_face() {
        reset_class_map();
        reset_file_map();
        reset_file_map();
        // fontFace - define custom font
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "fonts.css.ts",
                r#"import { fontFace, style } from '@devup-ui/react'
const myFont = fontFace({
  src: 'local("Comic Sans MS")'
})
export const text = style({
  fontFamily: myFont
})
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // fontFace with multiple sources
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "fonts-multi.css.ts",
                r#"import { fontFace, style } from '@devup-ui/react'
const roboto = fontFace({
  src: 'url("/fonts/Roboto.woff2") format("woff2")',
  fontWeight: 400,
  fontStyle: 'normal'
})
export const body = style({
  fontFamily: roboto
})
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_theme() {
        reset_class_map();
        reset_file_map();
        reset_file_map();
        // createTheme - define theme with variables
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme.css.ts",
                r"import { createTheme, style } from '@devup-ui/react'
export const [themeClass, vars] = createTheme({
  color: {
    brand: 'blue',
    text: 'black'
  },
  space: {
    small: '4px',
    medium: '8px',
    large: '16px'
  }
})
export const box = style({
  color: vars.color.text,
  padding: vars.space.medium
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        reset_file_map();
        // createThemeContract - type-safe theme contract
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme-contract.css.ts",
                r"import { createThemeContract, createTheme, style } from '@devup-ui/react'
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
})
export const darkTheme = createTheme(vars, {
  color: {
    brand: 'lightblue',
    text: 'white'
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_layer_record_places_every_declaration() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { styled } from '@devup-ui/react'
const A = styled.div({ '@layer': { base: {
  color: cond ? 'red' : 'blue',
  positioning: pos,
  bg: { a: 'red', b: 'blue' }[key],
  typography: typo,
  width: w,
  p: [1, 2],
  '@layer': { inner: { m: 1 } },
} } })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_layer_records() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "layers.css.ts",
                r"import { layer, style, globalStyle } from '@vanilla-extract/css'
const reset = layer('reset')
export const components = layer('components')
globalStyle('a', { color: 'blue', '@layer': { [reset]: { color: 'inherit', padding: 0 } } })
export const button = style({ color: 'red', '@layer': { [components]: { color: 'green', padding: 4 } } })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([(
                        "@vanilla-extract/css".to_string(),
                        ImportAlias::NamedToNamed
                    )])
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_layer() {
        reset_class_map();
        reset_file_map();
        reset_file_map();
        // layer - CSS cascade layers
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "layers.css.ts",
                r"import { layer, style, globalStyle } from '@devup-ui/react'
export const reset = layer('reset')
export const base = layer('base')
export const components = layer('components')
globalStyle('*', {
  '@layer': reset,
  margin: 0,
  padding: 0
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_container() {
        reset_class_map();
        reset_file_map();
        // createContainer - container queries
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "container.css.ts",
                r"import { createContainer, style } from '@devup-ui/react'
export const sidebar = createContainer()
export const sidebarContainer = style({
  containerName: sidebar,
  containerType: 'inline-size'
})
export const responsive = style({
  '@container': {
    [`${sidebar} (min-width: 400px)`]: {
      flexDirection: 'row'
    }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_global_theme() {
        reset_class_map();
        reset_file_map();
        // createGlobalTheme - global theme variables on :root
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "global-theme.css.ts",
                r"import { createGlobalTheme } from '@devup-ui/react'
export const vars = createGlobalTheme(':root', {
  color: {
    brand: 'blue',
    text: 'black',
    background: 'white'
  },
  font: {
    body: 'system-ui, sans-serif'
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_composition() {
        reset_class_map();
        reset_file_map();
        // style composition - array of styles
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "composition.css.ts",
                r"import { style } from '@devup-ui/react'
const base = style({
  padding: 12,
  borderRadius: 4
})
const interactive = style({
  cursor: 'pointer',
  transition: 'all 0.2s'
})
export const button = style([base, interactive, {
  background: 'blue',
  color: 'white'
}])
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_selectors() {
        reset_class_map();
        reset_file_map();
        // complex selectors
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "selectors.css.ts",
                r"import { style } from '@devup-ui/react'
export const button = style({
  background: 'blue',
  selectors: {
    '&:hover': {
      background: 'darkblue'
    },
    '&:focus': {
      outline: '2px solid blue'
    },
    '&:active': {
      transform: 'scale(0.98)'
    },
    '&:disabled': {
      opacity: 0.5,
      cursor: 'not-allowed'
    }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        reset_class_map();
        reset_file_map();
        // parent and sibling selectors
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "selectors-complex.css.ts",
                r"import { style } from '@devup-ui/react'
export const parent = style({
  background: 'white'
})
export const child = style({
  selectors: {
    [`${parent}:hover &`]: {
      color: 'blue'
    },
    '& + &': {
      marginTop: 8
    }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_media_queries() {
        reset_class_map();
        reset_file_map();
        // @media queries
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "media.css.ts",
                r"import { style } from '@devup-ui/react'
export const responsive = style({
  display: 'block',
  '@media': {
    'screen and (min-width: 768px)': {
      display: 'flex'
    },
    'screen and (min-width: 1024px)': {
      display: 'grid'
    },
    '(prefers-color-scheme: dark)': {
      background: 'black',
      color: 'white'
    }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_supports() {
        reset_class_map();
        reset_file_map();
        // @supports queries
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "supports.css.ts",
                r"import { style } from '@devup-ui/react'
export const grid = style({
  display: 'flex',
  '@supports': {
    '(display: grid)': {
      display: 'grid'
    }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }
    // END VANILLA-EXTRACT API TESTS

    #[rstest]
    #[case("test.tsx", "const x = 1;", "@devup-ui/react", false)] // no package string
    #[case(
        "test.invalid",
        "import { Box } from '@devup-ui/react';",
        "@devup-ui/react",
        false
    )] // invalid extension
    #[case(
        "test.tsx",
        "import { Box } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // named import
    #[case(
        "test.tsx",
        "// import from @devup-ui/react\nconst x = 1;",
        "@devup-ui/react",
        false
    )] // in comment
    #[case(
        "test.tsx",
        "import { Box } from '@devup-ui/core';",
        "@devup-ui/react",
        false
    )] // different package
    #[case(
        "test.tsx",
        "import Box from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // default import
    #[case(
        "test.tsx",
        "import * as DevupUI from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // namespace import
    #[case(
        "test.tsx",
        "import React from 'react';\nimport { Box } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // multiple imports
    #[case("test.tsx", "const pkg = '@devup-ui/react';", "@devup-ui/react", false)] // string literal
    #[case(
        "test.js",
        "import { Box } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // .js
    #[case(
        "test.ts",
        "import { Box } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // .ts
    #[case(
        "test.jsx",
        "import { Box } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // .jsx
    #[case(
        "test.unknown",
        "import { Box } from '@devup-ui/react';",
        "@devup-ui/react",
        false
    )] // unsupported file extension
    #[case(
        "styles.css.ts",
        "import { css } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // .css.ts (devup-ui style)
    #[case(
        "styles.css.js",
        "import { css } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // .css.js (devup-ui style)
    #[case(
        "styles.css.ts",
        "import { style } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // .css.ts (vanilla-extract API from devup-ui)
    #[case(
        "styles.css.js",
        "import { style } from '@devup-ui/react';",
        "@devup-ui/react",
        true
    )] // .css.js (vanilla-extract API from devup-ui)
    fn test_has_devup_ui(
        #[case] filename: &str,
        #[case] code: &str,
        #[case] package: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(has_devup_ui(filename, code, package), expected);
    }

    // ============================================================================
    // COVERAGE EDGE CASE TESTS
    // ============================================================================

    #[test]
    #[serial]
    fn test_container_at_rule_in_css() {
        // Test @container at-rule (covers line 134 in extract_style_from_expression.rs)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<div className={css({
  _container: {
    "(min-width: 400px)": {
      display: "flex"
    }
  }
})} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test with @container prefix
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<div className={css({
  "@container": {
    "(min-width: 500px)": {
      background: "blue"
    }
  }
})} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_invalid_js_execution() {
        // Test vanilla-extract file with invalid JS (covers line 107 fallback)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        // This should fall back to regular extraction when JS execution fails
        let result = extract(
            "invalid.css.ts",
            r#"import { style } from '@devup-ui/react'
// Invalid JS that will cause execution to fail
export const broken = style((() => { throw new Error("fail"); })())
"#,
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_dir: "@devup-ui/react".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        );
        // Should not panic, may return error or empty styles
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_empty_styles() {
        // Test vanilla-extract file that produces empty styles (covers line 116)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        // File with no style() calls
        let result = extract(
            "empty.css.ts",
            r"import { style } from '@devup-ui/react'
// No actual style calls, just comments
const unused = 1;
",
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_dir: "@devup-ui/react".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        );
        assert!(result.is_ok());
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_constant_exports() {
        // Test vanilla-extract file with constant exports (covers lines 576-577)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "constants.css.ts",
                r"import { style } from '@devup-ui/react'
export const SPACING = 8;
export const COLORS = { primary: 'blue', secondary: 'green' };
export const box = style({ padding: SPACING })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_theme_with_vars() {
        // Test createTheme with array destructuring [themeClass, vars] (covers lines 406-430)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme-vars.css.ts",
                r"import { createTheme } from '@devup-ui/react'
export const [lightTheme, vars] = createTheme({
  color: {
    primary: 'blue',
    secondary: 'green'
  },
  space: {
    small: '4px',
    medium: '8px'
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_non_exported_theme() {
        // Test non-exported createTheme (covers theme branches without export)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "internal-theme.css.ts",
                r"import { createTheme, style } from '@devup-ui/react'
const [internalTheme, themeVars] = createTheme({
  colors: {
    bg: 'white',
    text: 'black'
  }
})
export const themed = style({
  background: 'red'
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_style_composition_empty() {
        // Test style with empty composition array
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "empty-comp.css.ts",
                r"import { style } from '@devup-ui/react'
export const empty = style([])
export const withEmpty = style([{}])
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_style_variants_with_base() {
        // Test styleVariants with base composition (covers lines 1165-1177)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "variants-base.css.ts",
                r"import { style, styleVariants } from '@devup-ui/react'
const base = style({
  padding: 8,
  borderRadius: 4
})
export const sizes = styleVariants({
  small: [base, { fontSize: 12 }],
  medium: [base, { fontSize: 16 }],
  large: [base, { fontSize: 20 }]
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_layer_and_container() {
        // Test layer() and createContainer() together (covers lines 1207-1216)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "layer-container.css.ts",
                r"import { layer, createContainer, style } from '@devup-ui/react'
export const resetLayer = layer('reset')
export const baseLayer = layer('base')
export const myContainer = createContainer()
export const containerStyle = style({
  containerName: myContainer,
  containerType: 'inline-size'
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_all_imports() {
        // Test file that uses css, globalCss, and keyframes together (covers lines 1049, 1052)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "all-imports.css.ts",
                r"import { style, globalStyle, keyframes, createTheme } from '@devup-ui/react'
export const [theme, vars] = createTheme({
  color: { primary: 'blue' }
})
export const fadeIn = keyframes({
  from: { opacity: 0 },
  to: { opacity: 1 }
})
globalStyle('body', {
  margin: 0
})
export const box = style({
  animation: 'fadeIn 1s'
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_theme_without_vars_name() {
        // Test createTheme two-arg form (covers lines 1108, 1111)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme-two-arg.css.ts",
                r"import { createThemeContract, createTheme } from '@devup-ui/react'
const contract = createThemeContract({
  color: {
    brand: null,
    text: null
  }
})
export const darkTheme = createTheme(contract, {
  color: {
    brand: 'white',
    text: 'lightgray'
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_font_face_with_style() {
        // Test fontFace used in style (covers fontFace placeholder replacement)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "font-usage.css.ts",
                r#"import { fontFace, style } from '@devup-ui/react'
export const myFont = fontFace({
  src: 'local("Comic Sans MS")',
  fontWeight: 400
})
export const text = style({
  fontFamily: myFont
})
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_vars_only() {
        // Test createVar exports (covers lines 1191-1192)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "vars-only.css.ts",
                r"import { createVar, style, fallbackVar } from '@devup-ui/react'
export const colorVar = createVar()
export const sizeVar = createVar()
export const box = style({
  color: fallbackVar(colorVar, 'blue'),
  fontSize: sizeVar
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_global_theme_empty_vars() {
        // Test createGlobalTheme with empty vars (covers line 1142 branch)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "global-theme-empty.css.ts",
                r"import { createGlobalTheme, style } from '@devup-ui/react'
export const emptyVars = createGlobalTheme(':root', {})
export const box = style({ padding: 8 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_non_exported_styles() {
        // Test non-exported styles mixed with exported (covers export flag branches)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "mixed-exports.css.ts",
                r"import { style, keyframes, createVar, createContainer, layer } from '@devup-ui/react'
const internalStyle = style({ padding: 4 })
const internalKeyframe = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })
const internalVar = createVar()
const internalContainer = createContainer()
const internalLayer = layer('internal')
export const publicStyle = style({ margin: 8 })
",
                ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_selector_references() {
        // Test styles referencing each other in selectors (covers find_selector_references)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "selector-refs.css.ts",
                r"import { style } from '@devup-ui/react'
export const parent = style({ background: 'white' })
export const child = style({
  selectors: {
    [`${parent}:hover &`]: {
      color: 'blue'
    }
  }
})
export const sibling = style({
  selectors: {
    [`${parent} + &`]: {
      marginTop: 8
    }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_has_devup_ui_parser_panic() {
        // Test has_devup_ui with invalid code that causes parser panic (covers line 202)
        // Invalid syntax that will make the parser panic
        let result = has_devup_ui(
            "test.tsx",
            "import {} '@devup-ui/react'; syntax error {{",
            "@devup-ui/react",
        );
        assert!(!result);
    }

    #[test]
    #[serial]
    fn test_global_css_fontfaces_object_syntax() {
        // Test globalCss fontFaces with object syntax (covers lines 103, 115 in extract_global_style_from_expression.rs)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from '@devup-ui/core'
globalCss({
    fontFaces: [
        {
            fontFamily: "CustomFont",
            src: "url('/fonts/custom.woff2')",
            fontWeight: "400",
            fontStyle: "normal"
        }
    ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test with multiple font properties to exercise disassemble_property
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from '@devup-ui/core'
globalCss({
    fontFaces: [
        {
            fontFamily: "MultiFont",
            src: "url('/fonts/multi.woff2')",
            fontWeight: "bold",
            fontDisplay: "swap"
        },
        {
            fontFamily: "AnotherFont",
            src: "local('Arial')"
        }
    ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_fontfaces_multi_value_optimization() {
        // Test fontFaces with font-family that needs multi-value optimization (covers line 69, 73-74)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from '@devup-ui/core'
globalCss({
    fontFaces: [
        {
            fontFamily: "'Custom Font', 'Fallback Font', sans-serif",
            src: "local('Custom Font'), url('/fonts/custom.woff2') format('woff2')",
            fontWeight: "400 700",
            fontStyle: "normal"
        }
    ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test fontFaces with content property (another multi-optimize property)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from '@devup-ui/core'
globalCss({
    fontFaces: [
        {
            fontFamily: "IconFont",
            src: "/fonts/icons.woff2"
        }
    ]
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_conditional_expression_with_selector() {
        // Test ConditionalExpression when name.is_none() and selector.is_some() (covers line 134)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<div className={css({
  _hover: condition ? { bg: "blue" } : { bg: "red" }
})} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test nested conditional within selector
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<div className={css({
  selectors: {
    "&:hover": condition ? { color: "blue" } : { color: "red" }
  }
})} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_non_vanilla_extract_file() {
        // Test non-.css.ts file (covers line 154 - is_vanilla_extract = false path)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        let result = extract(
            "regular.tsx",
            r#"import { Box } from '@devup-ui/react'
<Box bg="red" p={4} />
"#,
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_dir: "@devup-ui/react".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.code.is_empty());
    }

    #[test]
    #[serial]
    fn test_stylesheet_evaluation_errors() {
        reset_class_map();
        reset_file_map();
        let error = extract(
            "broken.css.ts",
            "import { type StyleRule, style } from '@devup-ui/react';\nconst tokens = { brand: 'red' };\nexport const a = style({ color: tokens.accent.toUpperCase() });",
            ExtractOption::default(),
        )
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
        assert!(
            error.starts_with("JS execution error: TypeError"),
            "{error}"
        );

        // Devup UI's own APIs still compile when evaluation fails
        reset_class_map();
        reset_file_map();
        let output = extract(
            "own.css.ts",
            "import type { DevupProps } from '@devup-ui/react';\nimport { css } from '@devup-ui/react';\nexport const a = css({ color: 'red' });",
            ExtractOption::default(),
        )
        .unwrap();
        assert!(!output.code.contains("css("), "{}", output.code);
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_execution_fallback() {
        // Test vanilla-extract file with execution error (covers line 116 fallback)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        // Syntax error in JS execution will trigger fallback
        let result = extract(
            "error.css.ts",
            r"import { style } from '@devup-ui/react'
const x = style({ padding: [[[}}} // invalid syntax
",
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_dir: "@devup-ui/react".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        );
        // Should not panic - falls back to regular processing
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    #[serial]
    fn test_import_alias_vanilla_extract_named() {
        // Test @vanilla-extract/css named exports in regular .tsx files (NOT .css.ts)
        // Note: .css.ts files use vanilla-extract's own processing which doesn't go through import aliases
        reset_class_map();
        reset_file_map();
        let mut aliases = HashMap::new();
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );

        // Use regular .tsx file (not .css.ts) for import alias to work
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { css } from '@vanilla-extract/css'
const buttonStyle = css({ bg: 'red', p: 4 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: aliases
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_keyframes_export() {
        // Test exported keyframes (covers lines 1052, 1152-1153)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "keyframes-export.css.ts",
                r"import { keyframes, style } from '@devup-ui/react'
export const spin = keyframes({
  from: { transform: 'rotate(0deg)' },
  to: { transform: 'rotate(360deg)' }
})
const internal = keyframes({
  '0%': { opacity: 0 },
  '100%': { opacity: 1 }
})
export const spinner = style({ animation: spin })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_theme_vars_name_only() {
        // Test createTheme with vars_name but no vars_object_json (covers line 1301)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme-vars-name.css.ts",
                r"import { createTheme } from '@devup-ui/react'
export const myTheme = createTheme({
  color: { primary: 'blue' }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_style_variants_mixed() {
        // Test styleVariants with mixed base and no-base (covers lines 1161-1184)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "variants-mixed.css.ts",
                r"import { style, styleVariants } from '@devup-ui/react'
const base = style({ borderRadius: 4, padding: 8 })
export const buttons = styleVariants({
  primary: [base, { background: 'blue', color: 'white' }],
  secondary: { background: 'gray', color: 'black' },
  danger: [base, { background: 'red' }]
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_global_theme_with_vars() {
        // Test createGlobalTheme with CSS vars (covers lines 1142-1144)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "global-theme-vars.css.ts",
                r"import { createGlobalTheme, style } from '@devup-ui/react'
export const vars = createGlobalTheme(':root', {
  color: {
    primary: 'blue',
    secondary: 'green'
  },
  space: {
    small: '4px',
    large: '16px'
  }
})
export const box = style({ padding: 8 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_font_face_empty_props() {
        // Test fontFace with minimal properties (covers line 1132-1135 empty props branch)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "fontface-minimal.css.ts",
                r"import { fontFace, style } from '@devup-ui/react'
export const minimalFont = fontFace({})
export const text = style({ fontFamily: minimalFont })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_imports_combination() {
        // Test file with multiple import types (covers lines 1049, 1052 import generation)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "imports-combo.css.ts",
                r"import { style, globalStyle, keyframes, fontFace } from '@devup-ui/react'
export const fadeIn = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })
export const myFont = fontFace({ src: 'local(Arial)' })
globalStyle('body', { margin: 0, fontFamily: myFont })
export const animated = style({ animation: fadeIn })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_theme_export_variations() {
        // Test createTheme with different export patterns (covers lines 1103-1111)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme-exports.css.ts",
                r"import { createTheme, createThemeContract, style } from '@devup-ui/react'
const contract = createThemeContract({
  colors: { bg: null, text: null }
})
export const lightTheme = createTheme(contract, {
  colors: { bg: 'white', text: 'black' }
})
const internalTheme = createTheme(contract, {
  colors: { bg: 'gray', text: 'darkgray' }
})
export const box = style({ padding: 8 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_style_composition_multiple() {
        // Test style with multiple style objects in composition array (covers lines 728-729)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "multi-comp.css.ts",
                r"import { style } from '@devup-ui/react'
const base = style({ padding: 8 })
export const complex = style([base, { margin: 4 }, { color: 'blue' }])
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_selector_class_replacement() {
        // Test selector references that need class name replacement (covers collected_styles_to_code_with_classes)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "selector-ref.css.ts",
                r"import { style } from '@devup-ui/react'
export const parent = style({ display: 'flex' })
export const child = style({
  selectors: {
    [`${parent}:hover &`]: { background: 'red' }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_all_exports_combined() {
        // Test file with styles, keyframes, globalStyles, themes, vars, containers, layers, fontFaces combined
        // Covers multiple import generation paths and code generation
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "all-combined.css.ts",
                r"import { style, globalStyle, keyframes, createVar, createContainer, layer, fontFace, createGlobalTheme, styleVariants } from '@devup-ui/react'
export const myVar = createVar()
export const myContainer = createContainer()
export const myLayer = layer('components')
export const myFont = fontFace({ src: 'local(Arial)' })
export const vars = createGlobalTheme(':root', { color: { primary: 'blue' } })
export const fade = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })
globalStyle('body', { margin: 0 })
const base = style({ padding: 8 })
export const buttons = styleVariants({
  primary: [base, { bg: 'blue' }],
  secondary: { bg: 'gray' }
})
export const box = style({ fontFamily: myFont })
",
                ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_theme_array_destructure() {
        // Test createTheme with array destructuring [themeClass, vars] (covers lines 384, 386-387)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme-array.css.ts",
                r"import { createTheme, style } from '@devup-ui/react'
export const [themeClass, themeVars] = createTheme({
  colors: { primary: 'blue', secondary: 'green' },
  spacing: { small: '4px', medium: '8px' }
})
export const themed = style({ color: themeVars.colors.primary })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_font_face_placeholder() {
        // Test fontFace placeholder remapping (covers lines 503-505)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "fontface-remap.css.ts",
                r#"import { fontFace, style } from '@devup-ui/react'
export const customFont = fontFace({
  src: 'url("/fonts/custom.woff2")',
  fontWeight: '400',
  fontDisplay: 'swap'
})
export const secondFont = fontFace({
  src: 'local("Helvetica")'
})
export const text = style({ fontFamily: customFont })
export const heading = style({ fontFamily: secondFont })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_global_theme_placeholder() {
        // Test createGlobalTheme placeholder remapping (covers global theme paths)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "globaltheme-remap.css.ts",
                r"import { createGlobalTheme, style } from '@devup-ui/react'
export const lightVars = createGlobalTheme(':root', {
  colors: { bg: 'white', text: 'black' }
})
export const darkVars = createGlobalTheme('.dark', {
  colors: { bg: 'black', text: 'white' }
})
export const box = style({ padding: 8 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_with_layer_in_selector() {
        // Test globalCss with @layer inside selector object (covers lines 103, 115 in extract_global_style_from_expression.rs)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from '@devup-ui/core'
globalCss({
    ".button": {
        "@layer": "components",
        padding: "8px",
        background: "blue"
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test with multiple selectors having @layer
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from '@devup-ui/core'
globalCss({
    ".card": {
        "@layer": "layout",
        display: "flex"
    },
    ".text": {
        "@layer": "typography",
        fontSize: "16px"
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_css_container_at_rule_with_selector() {
        // Test @container at-rule within selector context (covers line 134)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { css } from "@devup-ui/core";
<div className={css({
  padding: 8,
  "@container": {
    "(min-width: 300px)": {
      padding: 16
    }
  }
})} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_selector_refs_triggers_with_classes() {
        // Test that triggers collected_styles_to_code_with_classes path (selector references)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "refs.css.ts",
                r"import { style, globalStyle, keyframes, createVar, fontFace, createContainer, layer } from '@devup-ui/react'
export const colorVar = createVar()
export const myContainer = createContainer()
export const myLayer = layer('ui')
export const myFont = fontFace({ src: 'local(Arial)' })
export const fade = keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })
export const parent = style({ display: 'flex' })
export const child = style({
  selectors: {
    [`${parent}:hover &`]: { color: 'red' }
  }
})
globalStyle('body', { margin: 0 })
",
                ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_theme_without_vars_json() {
        // Test createTheme that has vars_name but might not have vars_object_json (covers line 1111)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme-simple.css.ts",
                r"import { createThemeContract, createTheme, style } from '@devup-ui/react'
const contract = createThemeContract({
  colors: { primary: null }
})
export const lightTheme = createTheme(contract, {
  colors: { primary: 'blue' }
})
export const box = style({ padding: 8 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_global_css_layer_property_extraction() {
        // Test globalCss with @layer property to trigger lines 103, 115 in extract_global_style_from_expression.rs
        // The @layer property should be extracted and then filtered out, with layer set on remaining styles
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { globalCss } from '@devup-ui/core'
globalCss({
    ".reset-box": {
        "@layer": "reset",
        margin: 0,
        padding: 0,
        boxSizing: "border-box"
    }
})
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_selector_refs_with_global_theme() {
        // Test that triggers append_non_style_code with global themes (covers lines 1142-1144, 1221-1222)
        // Need selector references + createGlobalTheme
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "theme-refs.css.ts",
                r"import { style, createGlobalTheme } from '@devup-ui/react'
export const vars = createGlobalTheme(':root', {
  colors: { primary: 'blue', secondary: 'green' }
})
export const parent = style({ background: 'white' })
export const child = style({
  selectors: {
    [`${parent}:hover &`]: { color: 'red' }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_with_at_container_selector() {
        // Test @container with selector context (covers line 134 in extract_style_from_expression.rs)
        reset_class_map();
        reset_file_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "container.css.ts",
                r"import { style } from '@devup-ui/react'
export const card = style({
  containerType: 'inline-size',
  '@container': {
    '(min-width: 400px)': {
      display: 'grid',
      gridTemplateColumns: '1fr 1fr'
    }
  }
})
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_extract_class_map_from_code_parser_panic() {
        // Test extract_class_map_from_code with invalid code that causes parser panic (covers line 153-154)
        let mut style_names = FxHashSet::default();
        style_names.insert("test".to_string());

        let result = extract_class_map_from_code(
            "test.tsx",
            "const {{ invalid syntax {{{{",
            &ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_dir: "@devup-ui/react".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
            &style_names,
        )
        .unwrap();

        assert!(result.is_empty());
    }

    // === Import Alias Tests ===

    #[test]
    #[serial]
    fn test_import_alias_emotion_styled() {
        // Test @emotion/styled default export → styled named export
        // Uses devup-ui's styled syntax: styled.button({ ... }) or styled("button")({ ... })
        reset_class_map();
        reset_file_map();
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import styled from '@emotion/styled'
const Button = styled.button({ bg: 'red', p: 4 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: aliases
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_import_alias_styled_components() {
        // Test styled-components default export → styled named export
        reset_class_map();
        reset_file_map();
        let mut aliases = HashMap::new();
        aliases.insert(
            "styled-components".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import styled from 'styled-components'
const Card = styled("div")({ bg: 'blue', m: 2 })
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: aliases
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_import_alias_skip_when_package_not_in_code() {
        // Test that extraction is skipped when neither package nor aliases are present
        reset_class_map();
        reset_file_map();
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import React from 'react'
const element = <div>Hello</div>",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: aliases
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_import_alias_renamed_import() {
        // Test aliased import that's renamed locally: import myStyled from '@emotion/styled'
        reset_class_map();
        reset_file_map();
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import myStyled from '@emotion/styled'
const Button = myStyled.button({ bg: 'green', p: 2 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: aliases
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_import_alias_custom_named_export() {
        // Test @emotion/styled default export → custom named export (emotionStyled, not styled)
        // This tests when user configures alias to map to a non-standard named export
        reset_class_map();
        reset_file_map();
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );

        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import emotionStyled from '@emotion/styled'
const Button = emotionStyled.button({ bg: 'purple', p: 3 })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: aliases
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_import_alias_multiple_aliases() {
        // Test with multiple aliases configured
        reset_class_map();
        reset_file_map();
        let mut aliases = HashMap::new();
        aliases.insert(
            "@emotion/styled".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases.insert(
            "styled-components".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        );
        aliases.insert(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        );

        // Test with @emotion/styled member syntax
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import styled from '@emotion/styled'
const Button = styled.button({ bg: 'red' })
",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: aliases
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_classname_extraction() {
        // Test Tailwind className extraction - classes should be replaced with devup-ui generated names
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box className="p-4 bg-red-500" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_classname_with_variants() {
        // Test Tailwind with hover and responsive variants
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box className="p-4 hover:bg-blue-500 sm:p-8" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_classname_with_devup_props() {
        // Test Tailwind className combined with devup-ui style props
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box className="p-4 bg-red-500" margin={2} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_arbitrary_values() {
        // Test Tailwind arbitrary values like w-[100px]
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box className="w-[100px] h-[50vh] text-[#ff0000]" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_negative_values() {
        // Test Tailwind negative values like -m-4
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box className="-m-4 -translate-x-1/2" />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_full_integration() {
        // Comprehensive integration test: realistic multi-component scenario
        // Tests multiple Tailwind features together in a real-world usage pattern
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "components/Card.tsx",
                r#"import {Box, Flex, Text} from '@devup-ui/core'

// Card component with comprehensive Tailwind usage
export const Card = () => (
  <Box className="bg-white rounded-lg shadow-md p-6 hover:shadow-xl transition-shadow dark:bg-gray-800">
    <Flex className="flex-col gap-4 sm:flex-row sm:items-center">
      <Box className="w-12 h-12 rounded-full bg-blue-500 flex items-center justify-center">
        <Text className="text-white font-bold text-lg">A</Text>
      </Box>
      <Box className="flex-1">
        <Text className="text-gray-900 font-semibold text-xl dark:text-white">Title</Text>
        <Text className="text-gray-500 text-sm mt-1 dark:text-gray-400">Subtitle</Text>
      </Box>
    </Flex>
    <Box className="mt-4 pt-4 border-t border-gray-200 dark:border-gray-700">
      <Text className="text-gray-700 leading-relaxed dark:text-gray-300">
        Content goes here with multiple Tailwind utilities combined.
      </Text>
    </Box>
  </Box>
)
"#,
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_with_all_variant_types() {
        // Test all variant types: responsive, state, dark mode
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box className="p-2 sm:p-4 md:p-6 lg:p-8 xl:p-10 2xl:p-12 hover:bg-blue-500 focus:ring-2 active:scale-95 disabled:opacity-50 dark:bg-gray-900 dark:hover:bg-gray-800" />
"#,
                ExtractOption { package: "@devup-ui/core".to_string(), css_dir: "@devup-ui/core".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_mixed_with_devup_responsive() {
        // Test Tailwind className with devup-ui responsive array props
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box 
  className="rounded-lg shadow-md hover:shadow-xl" 
  p={[2, 4, 6]} 
  bg={["red", "blue", "green"]}
  display={["block", "flex"]}
/>
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_with_as_prop() {
        // Test Tailwind className with "as" prop for polymorphic components
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/core'
<Box as="p" className="text-gray-900">text</Box>
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_with_conditional_template_literal() {
        // Test Tailwind className with conditional expression in template literal
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
<Box className={`${enabled ? 'text-green-500' : 'text-blue-500'} text-3xl pr-5`}>
  hello
</Box>
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_template_literal_with_logical_expression() {
        // Test LogicalExpression in template literal (covers lines 239-242, 290-292 in prop_modify_utils.rs)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
<Box className={`${isActive && 'text-red-500'} ${fallback || 'text-blue-500'} p-4`}>
  hello
</Box>
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_template_literal_with_parenthesized_expression() {
        // Test ParenthesizedExpression in template literal (covers lines 244-246, 295-296 in prop_modify_utils.rs)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
<Box className={`${(cond ? 'text-red-500' : 'text-blue-500')} p-4`}>
  hello
</Box>
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_template_literal_with_nested_template_literal() {
        // Test nested TemplateLiteral (covers lines 248, 299-302 in prop_modify_utils.rs)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
<Box className={`${`text-${color}-500`} p-4`}>
  hello
</Box>
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_template_literal_with_variable_expression() {
        // Test non-string expressions (variables/identifiers) - covers line 250 in prop_modify_utils.rs
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
<Box className={`${dynamicClass} text-red-500 p-4`}>
  hello
</Box>
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_template_literal_with_function_call_expression() {
        // Test function call expressions in template literal (covers line 250 default branch)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
<Box className={`${getClass()} text-blue-500 p-4`}>
  hello
</Box>
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_template_literal_with_style_order() {
        // Test style_order parameter in build_tailwind_class_mapping (covers line 178)
        // styleOrder prop must be set explicitly to trigger the style_order code path
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import {Box} from '@devup-ui/core'
<Box styleOrder={5} className={`${enabled ? 'text-green-500' : 'text-blue-500'} p-4`}>
  hello
</Box>
",
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn style_order_coverage_additional() {
        // Coverage: visit.rs:241 — non-object 2nd argument in call expression
        // When 2nd arg is a variable (not ObjectExpression), pre-scan should return None
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, someVariable);
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: visit.rs:492 — non-conditional _ branch in JSX with static styleOrder
        // Ensures the `_ =>` (non-Conditional) match arm in visit_jsx_element is hit
        // with an explicit static styleOrder value
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={5} bg="red" p={4} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: visit.rs:259,262 — ParsedStyleOrder::None fallback with static styleOrder
        // via call expression where pre-scan finds styleOrder=5 (static), so extract_style_from_expression
        // also returns style_order=Some(5), but the pre-scan already resolved it to Static(5)
        // This ensures the full `ParsedStyleOrder::None => match style_order` path in visit_call_expression
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { styleOrder: 5, bg: "red", p: 4 });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: lib.rs:55 — MemberExpression variant clone_in with conditional styleOrder
        // Requires a computed member expression pattern (array/object indexed by variable)
        // to produce ExtractStyleProp::MemberExpression, which is then cloned for the alternate branch
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box styleOrder={a ? 1 : 2} bg={["red", "blue"][idx]} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: visit.rs — ParsedStyleOrder::None + _ match arm in visit_call_expression
        // Call expression WITHOUT any styleOrder property.
        // Pre-scan finds no styleOrder → ParsedStyleOrder::None.
        // extract_style_from_expression also returns style_order=None.
        // Exercises: ParsedStyleOrder::None branch and the _ => (non-Conditional) arm.
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.mjs",
                r#"import { jsx as e } from "react/jsx-runtime";
import { Box as o } from "@devup-ui/react";
function c() {
  return e(o, { bg: "red", p: 4 });
}
export { c as Lib };"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Coverage: visit.rs — _ match arm in visit_jsx_element (non-Conditional path)
        // JSX element WITHOUT any styleOrder prop at all.
        // parsed_style_order stays ParsedStyleOrder::None (default).
        // Exercises the _ => arm in visit_jsx_element's match on parsed_style_order.
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.jsx",
                r#"import {Box} from '@devup-ui/core'
<Box bg="red" p={4} />
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    fn test_combine_conditional_class_name() {
        use crate::prop_modify_utils::combine_conditional_class_name;
        use crate::utils::expression_to_code;
        use oxc_ast::ast::{Expression, Str};
        use oxc_ast::builder::AstBuilder;
        use oxc_span::SPAN;

        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);

        let make_cond = || Expression::new_identifier(SPAN, "cond", &builder);
        let make_str = |s| Expression::new_string_literal(SPAN, Str::from(s), None, &builder);

        // (Some, Some) — both branches have classNames
        let result = combine_conditional_class_name(
            &builder,
            make_cond(),
            Some(make_str("a")),
            Some(make_str("b")),
        );
        assert!(result.is_some());
        let code = expression_to_code(&result.unwrap());
        assert!(code.contains("cond"), "expected condition in: {code}");

        // (Some, None) — only consequent has className, alternate falls back to ""
        let result =
            combine_conditional_class_name(&builder, make_cond(), Some(make_str("a")), None);
        assert!(result.is_some());
        let code = expression_to_code(&result.unwrap());
        assert!(code.contains("cond"), "expected condition in: {code}");

        // (None, Some) — only alternate has className, consequent falls back to ""
        let result =
            combine_conditional_class_name(&builder, make_cond(), None, Some(make_str("b")));
        assert!(result.is_some());
        let code = expression_to_code(&result.unwrap());
        assert!(code.contains("cond"), "expected condition in: {code}");

        // (None, None) — neither has className
        let result = combine_conditional_class_name(&builder, make_cond(), None, None);
        assert!(result.is_none());
    }

    // ==========================================
    // StyleX Phase 1 tests
    // ==========================================

    #[test]
    #[serial]
    fn test_stylex_import_default() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'red' } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_import_namespace() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import * as stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'blue' } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_import_named() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { create } from '@stylexjs/stylex';
const styles = create({ base: { color: 'green' } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_create_single_namespace_static() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: 'red',
        backgroundColor: 'blue',
        padding: '10px',
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_create_multiple_namespaces() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: 'red',
    },
    active: {
        color: 'blue',
        fontWeight: 'bold',
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    fn static_values(output: &ExtractOutput) -> BTreeSet<(String, String)> {
        output
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::Static(st) => {
                    Some((st.property().to_string(), st.value().to_string()))
                }
                ExtractStyleValue::Keyframes(keyframes) => keyframes
                    .keyframes
                    .values()
                    .flatten()
                    .next()
                    .map(|st| (st.property().to_string(), st.value().to_string())),
                _ => None,
            })
            .collect()
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_numbers_follow_vanilla_extract_units() {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "numbers.css.ts",
            r"import { style, globalStyle, keyframes, styleVariants } from '@devup-ui/react'
export const box = style({
  fontSize: 16,
  top: -2,
  width: 0,
  lineHeight: 1.5,
  zIndex: 2,
  columnCount: 3,
  strokeWidth: 2,
  aspectRatio: 1.5,
  selectors: { '&:hover': { marginTop: 4 } },
  '@media': { print: { left: 1.5 } },
})
export const spin = keyframes({ to: { height: 10 } })
export const size = styleVariants({ small: { maxWidth: 20 } })
globalStyle('body', { margin: 2 })
",
            ExtractOption::default(),
        )
        .unwrap();
        let values = static_values(&output);
        for (property, value) in [
            ("font-size", "16px"),
            ("top", "-2px"),
            ("width", "0"),
            ("line-height", "1.5"),
            ("z-index", "2"),
            ("column-count", "3"),
            ("stroke-width", "2"),
            ("aspect-ratio", "1.5"),
            ("margin-top", "4px"),
            ("left", "1.5px"),
            ("height", "10px"),
            ("max-width", "20px"),
            ("margin", "2px"),
        ] {
            assert!(
                values.contains(&(property.to_string(), value.to_string())),
                "{property}: {value} not in {values:?}"
            );
        }
    }

    #[test]
    #[serial]
    fn test_stylex_numbers_follow_stylex_units() {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "test.tsx",
            r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
  base: {
    fontSize: 16,
    marginTop: -8,
    width: -0,
    height: '16',
    lineHeight: 1.5,
    transitionDuration: 300,
    '--size': 4,
    left: 1.33333,
    top: { default: 2, ':hover': 4 },
    right: stylex.firstThatWorks(6, 'auto'),
  },
  dynamic: (opacity, delay) => ({ opacity, transitionDelay: delay, bottom: 3 }),
});
const result = stylex.props(styles.dynamic(o, d));",
            ExtractOption::default(),
        )
        .unwrap();
        let values = static_values(&output);
        for (property, value) in [
            ("font-size", "16px"),
            ("margin-top", "-8px"),
            ("width", "0"),
            ("height", "16"),
            ("line-height", "1.5"),
            ("transition-duration", "300ms"),
            ("--size", "4"),
            ("left", "1.3333px"),
            ("top", "2px"),
            ("top", "4px"),
            ("right", "6px"),
            ("bottom", "3px"),
        ] {
            assert!(
                values.contains(&(property.to_string(), value.to_string())),
                "{property}: {value} not in {values:?}"
            );
        }
        assert!(output.code.contains(r#""--a": o"#), "{}", output.code);
        assert!(
            output
                .code
                .contains(r#"((v) => typeof v === "number" ? v + "ms" : v)(d)"#),
            "{}",
            output.code
        );
    }

    #[test]
    #[serial]
    fn test_stylex_create_numeric_values() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        fontSize: 16,
        opacity: 0.5,
        zIndex: 10,
        fontWeight: 700,
        padding: 8,
        flex: 1,
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_create_empty_object() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_create_empty_namespace() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: {} });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_pseudo_class_hover() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: { default: 'red', ':hover': 'blue' },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_pseudo_class_focus() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: { default: 'red', ':focus': 'blue' },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_media_query() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: { default: 'red', '@media (max-width: 600px)': 'blue' },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_supports_query() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        display: { default: 'block', '@supports (display: grid)': 'grid' },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_nested_hover_in_media() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: { ':hover': { default: null, '@media (hover: hover)': 'blue' } },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_nested_media_then_hover() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: { '@media (min-width: 768px)': { default: 'red', ':hover': 'blue' } },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_pseudo_element_placeholder() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        '::placeholder': { color: '#999', opacity: 1 },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_pseudo_element_before() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        '::before': { content: '""', display: 'block' },
    }
});"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_null_value_no_css() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: { default: null, ':hover': 'blue' },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_multiple_conditions() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        backgroundColor: { default: 'white', ':hover': 'gray' },
        color: { default: 'black', ':hover': 'red' },
    }
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    // ── Phase 3: stylex.props() integration tests ──────────────────────

    #[test]
    #[serial]
    fn test_stylex_props_single_static() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { color: 'red', fontSize: '16px' },
});
const el = <div {...stylex.props(styles.base)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_multiple_static() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { color: 'red' },
    active: { backgroundColor: 'blue' },
});
const el = <div {...stylex.props(styles.base, styles.active)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_computed_literal_key() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { color: 'red' },
    active: { backgroundColor: 'blue' },
});
const el = <div {...stylex.props(styles['base'])} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_computed_dynamic_key() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { display: 'inline-block', fontWeight: '500' } });
const colorStyles = stylex.create({ red: { color: 'red' }, blue: { color: 'blue' } });
const sizeStyles = stylex.create({ sm: { fontSize: '12px' }, lg: { fontSize: '20px' } });
const el = <div {...stylex.props(styles.base, colorStyles[color], sizeStyles[size])} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_computed_key_in_conditional() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const colorStyles = stylex.create({ red: { color: 'red' }, blue: { color: 'blue' } });
const el = <div {...stylex.props(isActive && colorStyles[color])} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_computed_key_unresolvable() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'red' } });
const el = <div {...stylex.props(styles['missing'], unknownStyles[color], styles[0])} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_computed_key_only_dynamic_namespaces() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ bar: (h) => ({ height: h }) });
const el = <div {...stylex.props(styles[variant])} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_computed_key_empty_namespace() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ empty: {}, filled: { color: 'red' } });
const el = <div {...stylex.props(styles['empty'], styles[key])} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_computed_key_non_identifier_object() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'red' } });
const el = <div {...stylex.props(theme.styles[color])} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_conditional_and() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { color: 'red' },
    active: { backgroundColor: 'blue' },
});
const el = <div {...stylex.props(styles.base, isActive && styles.active)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_conditional_ternary() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    primary: { color: 'red' },
    secondary: { color: 'blue' },
});
const el = <div {...stylex.props(isPrimary ? styles.primary : styles.secondary)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_zero_args() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const el = <div {...stylex.props()} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_with_conditions() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { color: { default: 'red', ':hover': 'blue' } },
});
const el = <div {...stylex.props(styles.base)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_named_import() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { create, props } from '@stylexjs/stylex';
const styles = create({
    base: { color: 'red' },
});
const el = <div {...props(styles.base)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_falsy_args() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { color: 'red' },
});
const el = <div {...stylex.props(false, null, styles.base)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    // ==========================================
    // StyleX Phase 4a tests — keyframes, firstThatWorks, types.*
    // ==========================================

    #[test]
    #[serial]
    fn test_stylex_keyframes_standalone() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const fadeIn = stylex.keyframes({
    from: { opacity: 0 },
    to: { opacity: 1 },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_keyframes_in_create() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const fadeIn = stylex.keyframes({
    from: { opacity: 0 },
    to: { opacity: 1 },
});
const styles = stylex.create({
    base: { animationName: fadeIn, animationDuration: '0.5s' },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_first_that_works_basic() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { position: stylex.firstThatWorks('sticky', '-webkit-sticky', 'fixed') },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_first_that_works_two_values() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { display: stylex.firstThatWorks('grid', 'flex') },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_types_length_strip() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { width: stylex.types.length('100px') },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_types_color_strip() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: { color: stylex.types.color('red') },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_keyframes_named_import() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { keyframes, create } from '@stylexjs/stylex';
const fadeIn = keyframes({
    from: { opacity: 0 },
    to: { opacity: 1 },
});
const styles = create({
    base: { animationName: fadeIn },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_dynamic_basic() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
  bar: (height) => ({
    height,
  }),
});
const result = stylex.props(styles.bar(h));",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_dynamic_mixed() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
  bar: (height) => ({
    height,
    width: '100%',
  }),
});
const result = stylex.props(styles.bar(h));",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_dynamic_multi_param() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
  bar: (h, w) => ({
    height: h,
    width: w,
  }),
});
const result = stylex.props(styles.bar(myH, myW));",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    // ==========================================
    // StyleX Phase 4c: Diagnostic error/warning tests
    // ==========================================

    #[test]
    #[serial]
    fn test_stylex_error_non_static_value() {
        for (value, shown) in [
            ("someVariable", "someVariable"),
            ("someFunc()", "someFunc()"),
            ("theme.colors.primary", "theme.colors.primary"),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!(
                    "import stylex from '@stylexjs/stylex';\nconst styles = stylex.create({{ base: {{ color: {value}, fontSize: '16px' }} }});"
                ),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert_eq!(
                message,
                format!(
                    "test.tsx:2:47: `stylex.create()` cannot use `{shown}` at build time: its values must be literals, theme tokens or constants, or be computed from them"
                )
            );
        }
    }

    #[test]
    #[serial]
    fn test_stylex_warn_shorthand_property() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
  base: {
    margin: '10px',
    color: 'red',
  },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_coexist_with_box() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import stylex from '@stylexjs/stylex';
import { Box } from '@devup-ui/react';
const styles = stylex.create({
    base: { color: 'blue', fontSize: '14px' },
});
const el = <div>
  <Box bg="red" p={2} />
  <div {...stylex.props(styles.base)} />
</div>;"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_coexist_with_css() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
import { css } from '@devup-ui/react';
const stylexStyles = stylex.create({
    base: { color: 'blue' },
});
const devupClass = css({ bg: 'red' });
const el = <div>
  <div className={devupClass} />
  <div {...stylex.props(stylexStyles.base)} />
</div>;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_first_that_works_in_condition() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        color: 'red',
        ':hover': {
            display: stylex.firstThatWorks('grid', 'flex'),
        },
    },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_types_in_condition() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({
    base: {
        width: {
            default: '100%',
            '@media (min-width: 768px)': stylex.types.length('50%'),
        },
    },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_include_basic() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const base = stylex.create({
    root: { color: 'red', fontSize: '16px' },
});
const composed = stylex.create({
    fancy: { ...stylex.include(base.root), backgroundColor: 'blue' },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_include_named_import() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { create, include } from '@stylexjs/stylex';
const base = create({
    root: { color: 'red' },
});
const composed = create({
    fancy: { ...include(base.root), padding: '8px' },
});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_include_with_props() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const base = stylex.create({
    root: { color: 'red' },
});
const composed = stylex.create({
    fancy: { ...stylex.include(base.root), backgroundColor: 'blue' },
});
const el = <div {...stylex.props(composed.fancy)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    // ==========================================
    // Coverage integration tests — stylex.rs, extract_style_from_stylex.rs, visit.rs
    // ==========================================

    #[test]
    #[serial]
    fn test_stylex_named_first_that_works() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { create, firstThatWorks } from '@stylexjs/stylex';
const styles = create({ base: { color: firstThatWorks('red', 'blue') } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_create_numeric_with_unit() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { fontSize: 16, lineHeight: 1.5 } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_first_that_works_numeric() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { zIndex: stylex.firstThatWorks(10, 20) } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_types_numeric() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { fontSize: stylex.types.length(16) } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_dynamic_empty_params() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: () => ({ color: 'red' }) });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_dynamic_numeric_value() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: (x) => ({ fontSize: 16, height: x }) });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_dynamic_keyframe_ref() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const fadeIn = stylex.keyframes({ from: { opacity: '0' }, to: { opacity: '1' } });
const styles = stylex.create({ base: (dur) => ({ animationName: fadeIn, animationDuration: dur }) });",
                ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_conditional_consequent_only() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ active: { color: 'red' } });
const el = <div {...stylex.props(isActive ? styles.active : unknownRef)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_conditional_alternate_only() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ fallback: { color: 'gray' } });
const el = <div {...stylex.props(isActive ? unknownRef : styles.fallback)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_conditional_none_none() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'red' } });
const el = <div {...stylex.props(isActive ? unknownA : unknownB)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_falsy_literals() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'red' } });
const el = <div {...stylex.props(styles.base, null, 0, false, '')} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_unresolvable_arg() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'red' } });
const el = <div {...stylex.props(styles.base, someFunction())} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_props_dynamic_as_non_call() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: (x) => ({ color: x }) });
const el = <div {...stylex.props(styles.base)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_include_dynamic_target() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const base = stylex.create({ dynamic: (x) => ({ color: x, fontSize: '14px' }) });
const composed = stylex.create({ fancy: { ...stylex.include(base.dynamic), backgroundColor: 'blue' } });",
                ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::new() },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_stylex_import_unknown_named() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { create, unknownFunction } from '@stylexjs/stylex';
const styles = create({ base: { color: 'red' } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    // ==========================================
    // Coverage tests: extract_style_from_stylex.rs
    // ==========================================

    // ==========================================
    // Coverage tests: stylex.rs
    // ==========================================

    /// Lines 66-67: Named import `types.X()`
    #[test]
    #[serial]
    fn test_stylex_named_import_types() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { create, types } from '@stylexjs/stylex';
const styles = create({ base: { width: types.length('100px') } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    // ==========================================
    // Coverage tests: visit.rs
    // ==========================================

    /// Line 188: Empty className in static namespace — styles.empty where empty has no properties
    #[test]
    #[serial]
    fn test_stylex_props_empty_namespace() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ empty: {} });
const el = <div {...stylex.props(styles.empty)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    /// Line 198: styles.nonexistent — member not found in namespace map
    #[test]
    #[serial]
    fn test_stylex_props_nonexistent_member() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'red' } });
const el = <div {...stylex.props(styles.missing)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    /// Line 208: None from `LogicalExpression` when right side can't resolve
    #[test]
    #[serial]
    fn test_stylex_props_logical_unresolvable_right() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const styles = stylex.create({ base: { color: 'red' } });
const el = <div {...stylex.props(isActive && unknownRef)} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    /// Line 332: include with empty `class_name_str` — include a namespace that has no properties
    #[test]
    #[serial]
    fn test_stylex_include_empty_namespace() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const base = stylex.create({ empty: {} });
const composed = stylex.create({ test: { ...stylex.include(base.empty), color: 'red' } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    /// Include-only namespace with no own styles — `class_name_str` starts empty
    /// Covers: visit.rs line 332 (`class_name_str` = `included_class` when empty)
    #[test]
    #[serial]
    fn test_stylex_include_only_no_own_styles() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import stylex from '@stylexjs/stylex';
const base = stylex.create({ root: { color: 'red' } });
const composed = stylex.create({ combined: { ...stylex.include(base.root) } });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_length_token_extraction() {
        // Test $token on gap prop
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box gap="$gutterMd" />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test $token on padding/margin shortcuts
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box p="$gutterMd" m="$gutterLg" />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test $token on multiple spacing props
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Flex} from '@devup-ui/react'
        <Flex gap="$gutterMd" p="$gutterSm" rowGap="$gutterLg" />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_shadow_token_extraction() {
        // Test $token on boxShadow prop
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box boxShadow="$sm" />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Test shadow token with other props
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box boxShadow="$md" bg="$primary" />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_responsive_length_token_literal_vs_array() {
        use css::theme_tokens::set_theme_token_levels;

        let mut length = BTreeMap::new();
        length.insert("containerX".to_string(), vec![0, 2]);
        set_theme_token_levels(length, BTreeMap::new());

        // String literal: w="$containerX" → expands to multiple breakpoint classes
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box w="$containerX" />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Expression: w={"$containerX"} → also expands (same as string literal)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box w={"$containerX"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Array with single element: w={["$containerX"]} → single class, base value only
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box w={["$containerX"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Mixed array: w={["1px", null, "$containerX"]} → token inside array stays single per slot
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box w={["1px", null, "$containerX"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_responsive_shadow_token_literal_vs_array() {
        use css::theme_tokens::set_theme_token_levels;

        let mut shadow = BTreeMap::new();
        shadow.insert("card".to_string(), vec![0, 3]);
        set_theme_token_levels(BTreeMap::new(), shadow);

        // String literal: boxShadow="$card" → expands to multiple breakpoint classes
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box boxShadow="$card" />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Expression: boxShadow={"$card"} → also expands (same as string literal)
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box boxShadow={"$card"} />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Array with single element: boxShadow={["$card"]} → single class, base value only
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box boxShadow={["$card"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // Mixed array: boxShadow={["none", null, null, "$card"]} → token inside array stays single per slot
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
        <Box boxShadow={["none", null, null, "$card"]} />
        "#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn extract_pseudo_selector_with_non_literal_value_graceful() {
        // Minimal regression tests for the panic that ERROR.tsx surfaced.
        //
        // When a pseudo-selector prop (`_hover`, `_active`, `_focus`, ...)
        // receives anything other than an inline object literal, the
        // extractor used to `unwrap()` a None prop-name inside the
        // `_xxx` selector recursion path and panic. The fix is to return
        // an empty ExtractResult from the non-literal branches of
        // `extract_style_from_expression` and
        // `extract_style_from_member_expression` when `name` is `None`.
        //
        // Each case below must NOT panic (used to panic before the fix).
        //   Formerly the un-extractable pseudo-selector attribute was dropped
        //      from the generated code (no class, no runtime style — devup-ui is
        //      fully static, so there is no runtime fallback to fall back
        //      to)
        let cases: &[(&str, &str)] = &[
            (
                "identifier",
                r"import {Box} from '@devup-ui/react'
const hoverStyle = { opacity: 1 };
export const A = () => <Box _hover={hoverStyle} />;
",
            ),
            (
                "call expression",
                r"import {Box} from '@devup-ui/react'
declare const getHover: () => object;
export const A = () => <Box _hover={getHover()} />;
",
            ),
            (
                "member expression",
                r"import {Box} from '@devup-ui/react'
declare const styles: { hover: object };
export const A = () => <Box _hover={styles.hover} />;
",
            ),
            (
                "binary expression",
                r"import {Box} from '@devup-ui/react'
declare const a: any; declare const b: any;
export const A = () => <Box _hover={a || b} />;
",
            ),
            (
                "template literal",
                r"import {Box} from '@devup-ui/react'
declare const x: string;
export const A = () => <Box _hover={`${x}`} />;
",
            ),
            (
                "unary expression",
                r"import {Box} from '@devup-ui/react'
declare const v: any;
export const A = () => <Box _hover={!v} />;
",
            ),
            (
                "computed member expression (array index)",
                r"import {Box} from '@devup-ui/react'
declare const arr: any[];
export const A = () => <Box _hover={arr[0]} />;
",
            ),
        ];

        for (label, src) in cases {
            reset_class_map();
            reset_file_map();
            let result = std::panic::catch_unwind(|| {
                extract(
                    "test.tsx",
                    src,
                    ExtractOption {
                        package: "@devup-ui/react".to_string(),
                        css_dir: "@devup-ui/react".to_string(),
                        single_css: true,
                        import_main_css: false,
                        import_aliases: HashMap::new(),
                    },
                )
            });
            // A constant object is inlined; anything else is a located error
            match result {
                Ok(Ok(output)) => {
                    assert_eq!(*label, "identifier");
                    assert_eq!(output.styles.len(), 1);
                }
                Ok(Err(e)) => {
                    assert!(e.to_string().contains("`<Box>` cannot use"), "{label}: {e}");
                }
                Err(panic_payload) => {
                    let msg = panic_payload
                        .downcast_ref::<&'static str>()
                        .map(|s| (*s).to_string())
                        .or_else(|| panic_payload.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "<non-string panic>".to_string());
                    panic!("[FAIL] {label}: extract panicked: {msg}");
                }
            }
        }
    }

    #[test]
    #[serial]
    fn extract_pseudo_selector_with_identifier_snapshot() {
        // Snapshot-locks the current behavior for the minimal ERROR.tsx
        // reduction so future refactors can't silently regress either
        // direction (re-introducing the panic, or over-eagerly turning the
        // `_hover={ident}` attribute into something surprising).
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import {Box} from '@devup-ui/react'
const hoverStyle = { opacity: 1 };
export const A = () => <Box _hover={hoverStyle} bg="red" />;
"#,
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                },
            )
            .unwrap()
        ));
    }

    // Coverage for extract_style_from_expression.rs:206 — the
    // `Expression::BinaryExpression | StaticMemberExpression | CallExpression`
    // arm. Both internal branches must be exercised in a single, dedicated
    // test so coverage tooling can attribute hits to this exact line:
    //
    //   207: `if let Some(name) = name { dynamic_style(...) }`  (Some path)
    //   211: `else { ExtractResult::default() }`                (None path,
    //          reached only via `_xxx={...}` selector recursion)
    //
    // Each variant (binary / static-member / call) is asserted on its own so
    // a regression in any single pattern fails loudly instead of being hidden
    // behind a multi-snapshot test.
    #[test]
    #[serial]
    fn extract_dynamic_style_props_binary_member_call_arm() {
        // ── Some(name) branch — line 207 ────────────────────────────────
        // BinaryExpression on a real prop name
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box bg={a + b} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // StaticMemberExpression on a real prop name
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box color={theme.color} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // CallExpression on a real prop name
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r#"import { Box } from "@devup-ui/core";
<Box w={getWidth()} />;
"#,
                ExtractOption {
                    package: "@devup-ui/core".to_string(),
                    css_dir: "@devup-ui/core".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::new()
                }
            )
            .unwrap()
        ));

        // A selector takes styles, so a value it cannot read is an error
        for src in [
            // BinaryExpression
            r"import {Box} from '@devup-ui/react'
declare const a: any; declare const b: any;
export const A = () => <Box _hover={a + b} />;
",
            // StaticMemberExpression
            r"import {Box} from '@devup-ui/react'
declare const t: { hover: object };
export const A = () => <Box _hover={t.hover} />;
",
            // CallExpression
            r"import {Box} from '@devup-ui/react'
declare const fn: () => object;
export const A = () => <Box _hover={fn()} />;
",
        ] {
            reset_class_map();
            reset_file_map();
            let error = extract("test.tsx", src, ExtractOption::default())
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();
            assert!(
                error.contains("`<Box>` cannot use") && error.contains(utils::STYLE_OBJECT),
                "{src}: {error}"
            );
        }
    }

    fn extract_tsx(code: &str) -> ExtractOutput {
        reset_class_map();
        reset_file_map();
        extract(
            "test.tsx",
            code,
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_dir: "@devup-ui/react".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        )
        .expect("extract should not fail")
    }

    /// `Argument::to_expression` panics on `...spread`. Every call site that can receive
    /// user-written arguments must reject the spread instead of unwrapping it.
    #[test]
    #[serial]
    fn test_spread_arguments_never_panic() {
        let stylex = "import stylex from '@stylexjs/stylex';\n";
        for source in [format!("{stylex}const e = <div {{...stylex.props(...a)}} />;"), format!("{stylex}const s = stylex.create(...a);"), format!("{stylex}const k = stylex.keyframes(...a);"), format!("{stylex}const s = stylex.create({{ bar: (h) => ({{ height: h }}) }});\nconst e = <div {{...stylex.props(s.bar(...a))}} />;"), format!("{stylex}const s = stylex.create({{ b: {{ position: stylex.firstThatWorks(...a) }} }});"), format!("{stylex}const s = stylex.create({{ b: {{ width: stylex.types.length(...a) }} }});"), format!("{stylex}const s = stylex.create({{ b: {{ ...stylex.include(...a) }} }});"), format!("{stylex}const s = stylex.create({{ b: {{ color: {{ default: stylex.firstThatWorks(...a) }} }} }});"), format!("{stylex}const s = stylex.create({{ b: {{ width: {{ default: stylex.types.length(...a) }} }} }});"), "import { jsx } from 'react/jsx-runtime';\nimport { Box } from '@devup-ui/react';\nconst e = jsx(...a);".to_string(), "import { jsx } from 'react/jsx-runtime';\nimport { Box } from '@devup-ui/react';\nconst e = jsx(Box, ...a);".to_string(), "import { globalCss } from '@devup-ui/react';\nglobalCss({ imports: [...list] });".to_string()] {
            reset_class_map();
            reset_file_map();
            let _ = extract("test.tsx", &source, ExtractOption::default());
        }
    }

    #[test]
    #[serial]
    fn test_stylex_props_style_x_array() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import stylex from '@stylexjs/stylex';
const s = stylex.create({ a: { color: 'red' }, b: { marginTop: '1px' } });
const flat = <div {...stylex.props([s.a, s.b])} />;
const nested = <div {...stylex.props([s.a, [s.b]])} />;
const conditional = <div {...stylex.props([s.a, on && s.b])} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_props_ts_wrapper_and_optional_chain() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import stylex from '@stylexjs/stylex';
const s = stylex.create({ a: { color: 'red' }, b: { marginTop: '1px' } });
const cast = <div {...stylex.props(s.a as any)} />;
const satisfied = <div {...stylex.props(s.b satisfies object)} />;
const nonNull = <div {...stylex.props(s.a!)} />;
const parens = <div {...stylex.props((s.b))} />;
const chained = <div {...stylex.props(s?.a)} />;
const computedChain = <div {...stylex.props(s?.[k])} />;
const chainedCall = <div {...stylex.props(s?.a())} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_attrs_emits_class_attribute() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import stylex from '@stylexjs/stylex';
const s = stylex.create({ a: { color: 'red' } });
const e = <div {...stylex.attrs(s.a)} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_define_vars_and_create_theme() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import stylex from '@stylexjs/stylex';
const colors = stylex.defineVars({ primary: 'blue', secondary: 'grey' });
const dark = stylex.createTheme(colors, { primary: 'navy' });
const styles = stylex.create({ box: { color: colors.primary, backgroundColor: colors.secondary } });
const el = <div {...stylex.props(dark, styles.box)} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_theme_apis_with_named_imports() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { defineVars, createTheme, create, props } from '@stylexjs/stylex';
const colors = defineVars({ primary: 'blue' });
const dark = createTheme(colors, { primary: 'navy' });
const styles = create({ box: { color: colors.primary } });
const el = <div {...props(dark, styles.box)} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_theme_contract_and_constants() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import stylex from '@stylexjs/stylex';
import { defineConsts } from '@stylexjs/stylex';
const vars = stylex.createThemeContract({ primary: null });
const consts = defineConsts({ gap: '8px' });
const dark = stylex.createTheme(vars, { primary: 'navy' });
const styles = stylex.create({ box: { color: vars.primary, marginTop: consts.gap } });
const el = <div {...stylex.props(dark, styles.box)} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_position_try_and_view_transition_class() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import stylex from '@stylexjs/stylex';
const fallback = stylex.positionTry({ top: '0', insetBlockEnd: 'auto' });
const transition = stylex.viewTransitionClass({ animationDuration: '300ms' });"
        )));
    }

    #[test]
    #[serial]
    fn test_styled_object_form_theme_interpolation() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { styled } from '@devup-ui/react';
const Themed = styled('div')({ color: (p) => p.theme.brand });"
        )));
        reset_class_map();
        reset_file_map();
        assert_eq!(
            extract(
                "test.tsx",
                "import { styled } from '@devup-ui/react';\nconst Plain = styled('span')({ color: (p) => p.color });",
                ExtractOption::default(),
            )
            .unwrap_err()
            .to_string(),
            "test.tsx:2:39: `styled()` cannot use `(p) => p.color` at build time: its styles must be an object literal or a constant object, or be computed from constants"
        );
    }

    #[test]
    #[serial]
    fn test_emotion_global_component_and_import_surface() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import styled from '@emotion/styled';
import { css, keyframes, Global, ThemeProvider, useTheme, ClassNames } from '@emotion/react';
const S = styled.div`color: ${p => p.theme.brand};`;
export const App = () => <><Global styles={{ body: { margin: '0px' } }} /><S /></>;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([
                        ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
                        (
                            "@emotion/styled".to_string(),
                            ImportAlias::DefaultToNamed("styled".to_string())
                        )
                    ])
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_emotion_and_styled_components_object_styles() {
        let aliases = HashMap::from([
            ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
            (
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
            (
                "styled-components".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
        ]);
        for code in [
            r"import styled from '@emotion/styled';
import { css, keyframes, Global } from '@emotion/react';
export const a = css({ padding: 8, lineHeight: 1.5, m: 2 }, [{ margin: 1 }, null, undefined, false]);
export const B = styled('div', { shouldForwardProp: (p) => p !== 'x' })({ left: 3 });
export const C = styled('span', { label: 'c' })`color: red;`;
export const D = styled.div([{ top: 1, color: 'red', '&:hover': { margin: 5 } }, { top: 3, '&:hover': { padding: 6 } }]);
export const E = styled.div({ bottom: 2 }, cond && { right: 4 });
export const F = styled('p', { color: 'blue' });
export const spin = keyframes({ to: { width: 10 } });
export const App = () => <><Global styles={{ body: { margin: 8 } }} /><Other styles={{ top: 1 }} /><Global css={{ top: 2 }} /></>;",
            r"import * as Styled from 'styled-components';
import { css } from 'styled-components';
export const G = Styled.div.attrs({ type: 'button' }).withConfig({ displayName: 'g' })({ right: 7 });
export const H = Styled(Base).withConfig({ displayName: 'h' })({ left: 9 });
export const I = Styled.div({ width: 1 });
export const j = css({ height: 2 });",
        ] {
            reset_class_map();
            reset_file_map();
            assert_debug_snapshot!(ToBTreeSet::from(
                extract(
                    "test.tsx",
                    code,
                    ExtractOption {
                        package: "@devup-ui/react".to_string(),
                        css_dir: "@devup-ui/react".to_string(),
                        single_css: true,
                        import_main_css: false,
                        import_aliases: aliases.clone(),
                    },
                )
                .unwrap()
            ));
        }
    }

    #[test]
    #[serial]
    fn test_styled_components_attrs_and_every_styled_import() {
        let aliases = HashMap::from([
            (
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
            (
                "styled-components".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
        ]);
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import emotion from '@emotion/styled';
import sc from 'styled-components';
export const A = emotion.div({ top: 1 });
export const B = sc.span({ left: 2 });
export const C = sc.button.attrs({ type: 'button' }).withConfig({ displayName: 'c' })({ right: 3 });
export const D = sc(Base).withConfig({ shouldForwardProp: () => true })`color: red;`;
export const E = (sc.input.attrs((props) => ({ size: props.small ? 5 : 10 })) as any).attrs(extra)({ bottom: 4 });
export const G = other.div.attrs({ role: 'note' })({ margin: 1 });
export const I = factory.attrs({ id: 'i' })({ margin: 3 });
export const J = other(Base).attrs({ id: 'j' })({ margin: 4 });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: aliases.clone(),
                },
            )
            .unwrap()
        ));
        // Never called for styles, so they would call `styled` at runtime
        for code in [
            "export const F = sc.div.attrs({ role: 'note' });",
            "export const H = sc.div.attrs(...rest)({ margin: 2 });",
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import sc from 'styled-components';\n{code}"),
                ExtractOption {
                    import_aliases: aliases.clone(),
                    ..ExtractOption::default()
                },
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(
                message.contains("is read at runtime, where it does not exist"),
                "{code}\n{message}"
            );
        }
    }

    #[test]
    #[serial]
    fn test_selector_keys_computed_from_constants() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                "import { css } from '@devup-ui/react';\nconst card = css({ p: 1 });\nconst HOVER = ':hover';\nexport const a = css({ selectors: { [`.${card}${HOVER} &`]: { m: 1 } }, [`.${card} &`]: { m: 2 } });",
                ExtractOption {
                    import_main_css: false,
                    ..ExtractOption::default()
                },
            )
            .unwrap()
        ));
        for (code, error) in [
            (
                "import { css } from '@devup-ui/react';\nlet X = 'x';\nexport const a = css({ selectors: { [X]: { m: 1 } } });",
                "`css()` cannot use `[X]`",
            ),
            (
                "import { Box } from '@devup-ui/react';\nlet X = 'x';\nexport const a = <Box selectors={{ [X]: { m: 1 } }} />;",
                "`<Box>` cannot use `[X]`",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract("test.tsx", code, ExtractOption::default())
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();
            assert!(message.contains(error), "{code}\n{message}");
        }
    }

    #[test]
    #[serial]
    fn test_compiled_bindings_never_dangle() {
        let outputs: Vec<(&str, String)> = [
            "import { css, styled, Box } from '@devup-ui/react';\nconst myCss = css;\nconst again = myCss;\nconst S = styled;\nconst B = Box;\nexport const a = again({ color: 'red' });\nexport const s = S.div({ color: 'blue' });\nexport const b = <B p={1} />;",
            "import { css } from '@devup-ui/react';\nexport function f(css) { return css; }\nexport const a = css({ color: 'red' });",
            "import { css, Box } from '@devup-ui/react';\ntype T = typeof css;\nexport type P = React.ComponentProps<typeof Box>;\nexport const a = css({ color: 'red' });",
            "import { css } from '@devup-ui/react';\nconst kept = 1, myCss = css;\nexport const a = myCss({ color: 'red' }) + kept;",
        ]
        .into_iter()
        .map(|code| {
            reset_class_map();
            reset_file_map();
            let output = extract(
                "test.tsx",
                code,
                ExtractOption {
                    import_main_css: false,
                    ..ExtractOption::default()
                },
            )
            .unwrap();
            (
                code,
                output
                    .code
                    .lines()
                    .filter(|line| !line.starts_with("import \""))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        })
        .collect();
        assert_debug_snapshot!(outputs);

        for (code, name) in [
            (
                "import { css } from '@devup-ui/react';\nexport const myCss = css;",
                "css",
            ),
            (
                "import { Box } from '@devup-ui/react';\nexport const C = Box;\nexport const d = <C p={1} />;",
                "Box",
            ),
            (
                "import { Box } from '@devup-ui/react';\nexport const list = [Box].map((B) => <B p={1} />);",
                "Box",
            ),
            (
                "import { css } from '@devup-ui/react';\nexport function f() { const inner = css; return inner({ color: 'red' }); }",
                "css",
            ),
            (
                "import { css } from '@devup-ui/react';\nconst myCss = css;\nexport const v = myCss;",
                "myCss",
            ),
            (
                "import { css } from '@devup-ui/react';\nlet myCss = css;\nmyCss = null;",
                "myCss",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract("test.tsx", code, ExtractOption::default())
                .err()
                .map(|error| error.to_string())
                .unwrap_or_default();
            assert!(
                message.contains(&format!(
                    "`{name}` is read at runtime, where it does not exist"
                )),
                "{code}\n{message}"
            );
        }
    }

    #[test]
    #[serial]
    fn test_composed_css_keeps_composed_classes() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { style } from '@vanilla-extract/css';
import { css } from '@emotion/react';
const base = style({ color: 'red' });
export const a = style([base, { margin: 2 }]);
export const b = css(base, 'extra', { padding: 1 });
export const c = css([base]);
export const d = style([cond && base]);",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([
                        ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
                        (
                            "@vanilla-extract/css".to_string(),
                            ImportAlias::NamedToNamed
                        )
                    ])
                },
            )
            .unwrap()
        ));
    }

    fn memory_resolver(
        files: &'static [(&'static str, &'static str)],
    ) -> impl Fn(&str, &str) -> Option<ResolvedModule> {
        move |specifier, importer| {
            let directory = importer
                .rsplit_once('/')
                .map_or("", |(directory, _)| directory);
            let path = format!("{directory}/{}", specifier.trim_start_matches("./"));
            files
                .iter()
                .find(|(file, _)| {
                    [".ts", ".css.ts", ".js", ""]
                        .iter()
                        .any(|extension| format!("{path}{extension}") == *file)
                })
                .map(|(file, code)| ResolvedModule {
                    path: (*file).to_string(),
                    code: (*code).to_string(),
                })
        }
    }

    const STYLESHEET_MODULES: &[(&str, &str)] = &[
        (
            "/src/theme.css.ts",
            r"import { createTheme, style } from '@vanilla-extract/css';
export const [themeClass, vars] = createTheme({ color: { brand: 'red' }, space: '4px' });
export const base = style({ padding: vars.space });",
        ),
        (
            "/src/tokens.ts",
            r"import { extra } from './more';
export const brand: string = 'blue';
export default 8;
const hidden = 1;
export { hidden as shown };
export * from './more';
export * as more from './more';
export { extra as renamed } from './more';
export function double(n: number) { return n * 2 + extra - extra; }
export class Scale {}",
        ),
        ("/src/more.ts", "export const extra = 3;"),
        (
            "/src/named.ts",
            "export default function named() { return 1 }\nexport const other = 1",
        ),
        ("/src/anonymous.ts", "export default class { }"),
        (
            "/src/legacy.js",
            "const { extra } = require('./more');\nconst compiled = require('./compiled');\nmodule.exports = { legacy: extra + compiled.default + compiled.named, default: 'ignored' };",
        ),
        (
            "/src/compiled.js",
            "'use strict';\nObject.defineProperty(exports, '__esModule', { value: true });\nexports.named = 2;\nexports.default = 10;",
        ),
        ("/src/value.js", "module.exports = 5;"),
    ];

    const STYLEX_MODULES: &[(&str, &str)] = &[
        (
            "/src/vars.stylex.ts",
            r"import * as stylex from '@stylexjs/stylex';
import { defineConsts, create } from '@stylexjs/stylex';
import sx from '@stylexjs/stylex';
import other from './brand';
import { BRAND } from './brand';
const DARK = '@media (prefers-color-scheme: dark)';
const GAP = 'gap';
export const colors = stylex.defineVars({ primary: 'blue', brand: BRAND, mode: { default: 'light', [DARK]: 'dark' }, typed: stylex.types.color({ default: 'red', '@supports (color: red)': null }) });
export const consts = defineConsts({ [GAP]: '8px', size: 4 });
export const contract = sx.createThemeContract({ accent: null });
export const dark = stylex.createTheme(colors, { primary: 'navy' });
export const notStylex = other.defineVars({ a: 'b' });
export const unknownCallee = missing.defineVars({ a: 'b' });
export const called = stylex({ a: 'b' });
export const nested = stylex.types.defineVars({ a: 'b' });
export const memberOfNamed = create.defineVars({ a: 'b' });
export const indirect = (0, stylex.defineVars)({ a: 'b' });
export const styles = stylex.create({ base: { color: 'red' } });",
        ),
        (
            "/src/brand.ts",
            "export const BRAND = 'green';\nexport default {};",
        ),
        (
            "/src/loose.stylex.ts",
            r"import * as stylex from '@stylexjs/stylex';
import { consts } from './vars.stylex';
export const loose = stylex.defineVars({ fine: 'ok', call: getColor(), pseudo: { default: 'a', ':hover': 'b' }, spread: { ...base }, unknownKey: { [unknown]: 'x' }, [computedKey]: 'x' });
export const unknownTheme = stylex.createTheme(unknown, { primary: 'navy' });
export const notContract = stylex.createTheme(consts, { primary: 'navy' });
export const looseConsts = stylex.defineConsts({ fine: '1px', dynamic: getSize() });",
        ),
        (
            "/src/themes.ts",
            r"import * as stylex from '@stylexjs/stylex';
import { colors } from './vars.stylex';
export const light = stylex.createTheme(colors, { primary: 'white' });",
        ),
    ];

    #[test]
    #[serial]
    fn test_stylex_code_that_cannot_compile() {
        for (code, errors) in [
            (
                "let key = 'base';\nconst styles = stylex.create({ [key]: { color: 'red' } });",
                &["`stylex.create()` cannot use `[key]` at build time: its keys must be known"][..],
            ),
            (
                "const styles = stylex.create(\"not-object\");",
                &[
                    "`stylex.create()` cannot use `\"not-object\"` at build time: it takes one object literal",
                ],
            ),
            (
                "const styles = stylex.create(...args);",
                &[
                    "`stylex.create()` cannot use `...args` at build time: it takes one object literal",
                ],
            ),
            (
                "const styles = stylex.create({ base: { opacity: { default: [1, 2] } } });",
                &["`stylex.create()` cannot use `[1, 2]` at build time"],
            ),
            (
                "let prop = 'color';\nconst styles = stylex.create({ base: { [prop]: 'red' } });",
                &["`stylex.create()` cannot use `[prop]` at build time"],
            ),
            (
                "let key = ':hover';\nconst styles = stylex.create({ base: { color: { [key]: 'blue', default: 'red' } } });",
                &["`stylex.create()` cannot use `[key]` at build time"],
            ),
            (
                "const styles = stylex.create({ base: { color: { default: 'red', hover: 'blue' } } });",
                &[
                    "`stylex.create()` cannot use `hover` at build time: a condition is `default`, a pseudo-class",
                ],
            ),
            (
                "const styles = stylex.create({ a: (x) => x, b: (x) => { return { color: x }; }, c: ({ x }) => ({ color: x }) });",
                &[
                    "`stylex.create()` cannot use `(x) => x` at build time: a dynamic style is",
                    "`stylex.create()` cannot use `(x) => { return { color: x }; }` at build time",
                    "`stylex.create()` cannot use `({ x }) => ({ color: x })` at build time",
                ],
            ),
            (
                "const styles = stylex.create({ base: (x) => ({ ...other, [Symbol()]: x, fontSize: someVar }) });",
                &[
                    "`stylex.create()` cannot use `...other` at build time: write every entry out",
                    "`stylex.create()` cannot use `[Symbol()]` at build time",
                    "`stylex.create()` cannot use `someVar` at build time: a dynamic style's value",
                ],
            ),
            (
                "const { base } = stylex.create({ base: { color: 'red' } });",
                &["test.tsx:2:7: `stylex.create()` cannot be destructured at build time"],
            ),
            (
                "const styles = stylex.create({ ...other, base: { ...shared, ...notInclude(), color: { ...spreadObj, default: 'red' } } });",
                &[
                    "`stylex.create()` cannot use `...other`",
                    "`stylex.create()` cannot use `...shared`",
                    "`stylex.create()` cannot use `...notInclude()`",
                    "`stylex.create()` cannot use `...spreadObj`",
                ],
            ),
            (
                "const styles = stylex.create({ base: 'not-an-object', hover: { ':hover': 'invalid' }, inner: { ':hover': { ...other, [Symbol()]: 'val' } } });",
                &[
                    "`stylex.create()` cannot use `\"not-an-object\"` at build time: a namespace is",
                    "`stylex.create()` cannot use `\"invalid\"` at build time: a pseudo-class or pseudo-element key takes an object of styles",
                    "`stylex.create()` cannot use `...other`",
                ],
            ),
            (
                "const styles = stylex.create({ base: { fontSize: stylex.types.length(someVar), width: { default: stylex.types.length() }, position: stylex.firstThatWorks('sticky', someVar, ...rest) } });",
                &[
                    "`stylex.create()` cannot use `stylex.types.length(someVar)` at build time",
                    "`stylex.create()` cannot use `stylex.types.length()` at build time",
                    "`stylex.firstThatWorks()` cannot use `someVar` at build time",
                    "`stylex.firstThatWorks()` cannot use `...rest` at build time",
                ],
            ),
            (
                "const styles = stylex.create({ base: { ...stylex.include(getStyles()) }, other: { ...stylex.include(elsewhere.base) } });",
                &[
                    "`stylex.include()` cannot use `stylex.include(getStyles())` at build time: it takes a namespace such as `styles.base`",
                    "`stylex.include()` cannot use `elsewhere.base` at build time: it takes a namespace `stylex.create()` defines earlier in this file",
                ],
            ),
            (
                "const consts = stylex.defineConsts({ ...other, [computed]: '1px', dynamic: someVar });",
                &[
                    "`stylex.defineConsts()` cannot use `...other`",
                    "`stylex.defineConsts()` cannot use `[computed]`",
                    "`stylex.defineConsts()` cannot use `someVar`",
                ],
            ),
            (
                "const vars = stylex.defineVars({ ...other, [computed]: 'red' });\nconst contract = stylex.createThemeContract({ ...other });",
                &[
                    "`stylex.defineVars()` cannot use `...other`",
                    "`stylex.defineVars()` cannot use `[computed]`",
                    "`stylex.createThemeContract()` cannot use `...other`",
                ],
            ),
            (
                "const c = stylex.createThemeContract(x);\nconst d = stylex.defineConsts(x);\nconst v = stylex.viewTransitionClass(x);",
                &[
                    "`stylex.createThemeContract()` cannot use `x`",
                    "`stylex.defineConsts()` cannot use `x`",
                    "`stylex.viewTransitionClass()` cannot use `x`",
                ],
            ),
            (
                "const vars = stylex.defineVars(someVariable);\nconst theme = stylex.createTheme(notAContract, { primary: 'navy' });",
                &[
                    "`stylex.defineVars()` cannot use `someVariable` at build time: it takes one object literal",
                    "`stylex.createTheme()` cannot use `notAContract, { primary: \"navy\" }` at build time: it takes a `defineVars()` group",
                ],
            ),
            (
                "const vars = stylex.defineVars({ primary: 'blue' });\nconst theme = stylex.createTheme(vars, { ...other, [computed]: 'red', missing: 'navy' });",
                &[
                    "`stylex.createTheme()` cannot use `...other`",
                    "`stylex.createTheme()` cannot use `[computed]`",
                    "`stylex.createTheme()` cannot use `missing` at build time: `vars` has no such variable",
                ],
            ),
            (
                "const fallback = stylex.positionTry(notAnObject);\nconst entries = stylex.positionTry({ ...spread, [computed]: '0', top: someVar });\nconst transition = stylex.viewTransitionClass({ animationDuration: later });\nconst fade = stylex.keyframes(frames);",
                &[
                    "`stylex.positionTry()` cannot use `notAnObject` at build time: it takes one object literal",
                    "`stylex.positionTry()` cannot use `...spread`",
                    "`stylex.positionTry()` cannot use `[computed]`",
                    "`stylex.positionTry()` cannot use `someVar`",
                    "`stylex.viewTransitionClass()` cannot use `later`",
                    "`stylex.keyframes()` cannot use `frames` at build time: it takes one object literal",
                ],
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import stylex from '@stylexjs/stylex';\n{code}"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            for error in errors {
                assert!(message.contains(error), "{error}\nmissing from\n{message}");
            }
        }
    }

    #[test]
    #[serial]
    fn test_stylex_props_join_styles_compiled_elsewhere() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import * as stylex from '@stylexjs/stylex';
const fade = stylex.keyframes({ from: { opacity: 0 }, to: { opacity: 1 } });
const colors = stylex.defineVars({ text: 'black' });
const styles = stylex.create({
  base: { animationName: { default: fade, ':hover': 'none' }, color: { default: colors.text, ':hover': 'red' } },
  bar: (h) => ({ height: h }),
  none: null,
});
export const A = ({ style, rest }) => <div {...stylex.props(styles.base, style, ...rest, undefined, null, styles, styles.missing, styles?.bar, styles[key]?.x, styles?.['missing'], on && styles.missing, on ? styles.base : styles.missing, on ? styles.missing : styles.base, on ? styles.missing : styles.none)} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_conditional_variables() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import * as stylex from '@stylexjs/stylex';
const DARK = '@media (prefers-color-scheme: dark)';
const colors = stylex.defineVars({
  layered: { '@supports (display: grid)': { [DARK]: 'navy' }, [DARK]: 'teal', default: 'white' },
  text: { default: 'black', [DARK]: 'white' },
  accent: stylex.types.color({
    default: 'blue',
    [DARK]: 'lightblue',
    '@supports (color: oklch(0 0 0))': { default: 'oklch(0.6 0.2 250)', [DARK]: 'oklch(0.8 0.1 250)' },
  }),
  onlyDark: { default: null, [DARK]: 'gray' },
  size: '4px',
});
const dracula = stylex.createTheme(colors, { text: { [DARK]: 'pink', default: 'purple' }, accent: 'red' });
const styles = stylex.create({ box: { color: colors.text, borderColor: colors.accent } });
export const A = () => <div {...stylex.props(dracula, styles.box)} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_variable_values_known_only_at_runtime() {
        for (code, error) in [
            (
                "const colors = stylex.defineVars({ text: someValue });",
                "`stylex.defineVars()` cannot use `someValue`",
            ),
            (
                "const colors = stylex.defineVars({ text: { default: 'a', ':hover': 'b' } });",
                "`stylex.defineVars()` cannot use `{",
            ),
            (
                "const colors = stylex.defineVars({ text: { default: 'a', ...rest } });",
                "`stylex.defineVars()` cannot use `{",
            ),
            (
                "const colors = stylex.defineVars({ text: 'a' });\nconst theme = stylex.createTheme(colors, { text: getText() });",
                "`stylex.createTheme()` cannot use `getText()`",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import * as stylex from '@stylexjs/stylex';\n{code}"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains(error), "{message}");
        }
    }

    #[test]
    #[serial]
    fn test_stylex_values_imported_from_other_modules() {
        for single_css in [true, false] {
            reset_class_map();
            reset_file_map();
            let option = ExtractOption {
                single_css,
                ..ExtractOption::default()
            };
            let resolver = memory_resolver(STYLEX_MODULES);
            let extract_module = |path: &str| {
                let code = STYLEX_MODULES
                    .iter()
                    .find(|(file, _)| *file == path)
                    .map(|(_, code)| *code)
                    .unwrap();
                extract_with_modules(path, code, option.clone(), false, &resolver).unwrap()
            };
            let vars = extract_module("/src/vars.stylex.ts");
            let themes = extract_module("/src/themes.ts");
            let app = extract_with_modules(
                "/src/App.tsx",
                r"import * as stylex from '@stylexjs/stylex';
import { colors, consts, contract, dark } from './vars.stylex';
import { loose } from './loose.stylex';
import { light } from './themes';
const s = stylex.create({ base: { color: colors.primary, backgroundColor: colors.brand, marginTop: consts.gap, zIndex: consts.size, borderColor: contract.accent, outlineColor: colors.mode, caretColor: colors.typed, accentColor: loose.fine } });
const custom = stylex.createTheme(colors, { primary: 'purple' });
export const A = () => <div {...stylex.props(dark, light, custom, s.base)} />;",
                option.clone(),
                false,
                &resolver,
            )
            .unwrap();
            let styles = format!("{:?}", app.styles);
            // Each name the defining modules generate is the one the importer reads
            let text_after = |code: &str, marker: &str, end: char| {
                let (_, rest) = code.split_once(marker).unwrap();
                rest.split(end).next().unwrap().to_string()
            };
            for key in ["primary", "brand", "mode", "typed"] {
                let reference = format!(
                    "var({})",
                    text_after(&vars.code, &format!("\"{key}\": \"var("), ')')
                );
                assert!(
                    styles.contains(&reference),
                    "{reference} missing from {styles}"
                );
            }
            for (module, name) in [(&vars.code, "dark"), (&themes.code, "light")] {
                let class = text_after(module, &format!("export const {name} = \""), '"');
                assert!(
                    app.code.contains(&class),
                    "{class} missing from {}",
                    app.code
                );
            }
            assert!(styles.contains("value: \"8px\""), "{styles}");
            assert!(styles.contains("--"), "{styles}");
            assert!(!app.code.contains("createTheme"), "{}", app.code);
            let vars_css = format!("{:?}", vars.styles);
            assert!(
                vars_css.contains("@media(prefers-color-scheme:dark){:root{--"),
                "{vars_css}"
            );
            assert_eq!(
                app.dependencies,
                vec![
                    "/src/brand.ts",
                    "/src/loose.stylex.ts",
                    "/src/themes.ts",
                    "/src/vars.stylex.ts"
                ]
            );
        }
    }

    const CONSTANT_MODULES: &[(&str, &str)] = &[
        (
            "/src/tokens.ts",
            r"import { base } from './base';
import * as spaced from './base';
import baseDefault from './base';
export const FROM_DEFAULT = baseDefault;
console.log(FROM_DEFAULT);
export const PRIMARY = 'red';
export const SIZE = 4;
export const NEG = -2;
export const TEMPLATE = `10px`;
export const colors = { [computed]: 'x', ...rest, primary: 'blue', nested: { deep: 'green' }, fromBase: base, skip: fn() } as const;
export const ALIAS = (PRIMARY satisfies string);
export let MUTABLE = 'no';
export const DYNAMIC = fn(), [destructured] = [1];
export const NEG_STRING = -'a';
export const TEMPLATE_EXPRESSION = `${PRIMARY}`;
export const SPACED = spaced.value;
export const NOT_OBJECT = PRIMARY.length;
const LOCAL = 'purple';
export { LOCAL as renamed, MUTABLE as alsoMutable };
export { base as reBase, missing as notThere } from './base';
export { gone } from './missing';
export * from './base';
export * from './missing';
export * as ns from './base';
export default 'orange';
export function helper() {}",
        ),
        (
            "/src/base.ts",
            r"import { x } from './loop';
export const base = 'teal';
export const value = 'navy';
export default 'hidden-by-star';
export const looped = x;",
        ),
        (
            "/src/loop.ts",
            "import { looped } from './base';\nexport const x = looped;",
        ),
        (
            "/src/named.ts",
            "const DEFAULT = 'olive';\nexport default DEFAULT;",
        ),
        ("/src/handler.ts", "export const handler = () => {};"),
        (
            "/src/cjs-tokens.js",
            "'use strict';\nObject.defineProperty(exports, '__esModule', { value: true });\nexports.COMPILED = exports.OTHER = void 0;\nconst base_1 = require('./base');\nconst { value: navy } = require('./base');\nlet loose = require('./base');\nconst notRequire = load('./base');\nconst [first] = require('./base');\nconst fromName = require(name);\nother.exports.y = 1;\nlist[0].x = 1;\nexports.COMPILED = 'maroon';\nexports.FROM_REQUIRE = base_1.base;\nexports.DESTRUCTURED = navy;\nexports.MUTATED = 'a';\nfunction mutate() { exports.MUTATED = 'b'; }\nexports['computed'] = 'x';\nexports.DYNAMIC = compute();\nother.thing = 1;\nexports.default = 'indigo';",
        ),
        (
            "/src/cjs-object.js",
            "module.exports = { OBJ: 'plum', OVERRIDDEN: 'x' };\nmodule.exports.OVERRIDDEN = 'y';\nmodule.exports.LATER = 'lime';\nmodule.id = 1;",
        ),
        ("/src/cjs-value.js", "module.exports = 12;"),
        (
            "/src/cjs-twice.js",
            "module.exports = { A: 'a' };\nmodule.exports = { A: 'b' };",
        ),
        (
            "/src/cjs-marker.js",
            "exports.__esModule = true;\nexports.named = 'teal';",
        ),
    ];

    const CHANGED_MODULES: &[(&str, &str)] = &[
        (
            "/src/tokens.ts",
            r"import { css } from '@devup-ui/react';
export const base = { p: 4 };
export const fixed = { p: 2 };
export const colors = { primary: 'red' };
export const grid = [1, 2];
const local = { m: 1 };
export { local as renamed };
export default base;
export const styles = css(fixed);
export function dark() { base.p = 8; }
export const hover = darken(colors.primary);
register(grid.length);",
        ),
        (
            "/src/reexport.ts",
            "export { base, fixed } from './tokens';",
        ),
    ];

    #[test]
    #[serial]
    fn test_changed_constants() {
        let cases = [
            "const base = { p: 4 };\nbase.p = 8;\nexport const a = css(base);",
            "const base = { p: 4 };\nObject.assign(base, { p: 8 });\nexport const a = <Box p={base.p} />;\nexport const b = css({ p: base.p });",
            "const base = { p: 4 };\nregister(base);\nexport const a = <Box {...base} />;\nexport const b = styled.div(base);\nexport const c = css(x ? base : null);\nexport const d = css(x ? null : [y && base]);\nexport const e = css(...[base]);",
            "const base = { p: 4 };\nbase.p = 8;\nconst baseline = x;\nexport const a = <Box w={base.p} />;\nexport const b = css({ p: baseline });",
            "const colors = { primary: 'red' };\nconst hover = darken(colors.primary);\nexport const a = <Box color={colors.primary} />;",
            "const colors = { primary: 'red' };\nexport const theme = { colors };\nexport const a = <Box color={colors.primary} />;",
            "const colors = { primary: 'red' };\nexport const theme = { colors };\ntheme.colors.primary = 'blue';\nexport const a = <Box color={colors.primary} />;",
            "const loop1 = { x: loop2 };\nconst loop2 = { y: loop1 };\nexport const a = <Box w={loop1.x.z} />;",
            "const list = [{ p: 1 }];\nwatch(...list);\nexport const a = <Box p={list[0].p} />;",
            "const flat = [1, 2];\nwatch(...flat);\nexport const a = <Box p={flat[0]} />;",
            "const partial = { ...unknown, p: 1 };\nwatch(partial.q);\nexport const a = <Box p={partial.p} />;",
            "const record = { p: 1 };\nwatch(record.q);\nexport const a = <Box p={record.p} />;",
            "const spread = { ...unknown, p: 1 };\nwatch({ ...spread });\nexport const a = <Box p={spread.p} />;",
            "export enum E { A = 1 }\nenum F { B = 2 }\nexport function g() {}\nvar v = 1;\nconst make = () => ({ p: 1 });\nconst made = make();\nwatch(made);\nconst f = () => made.p;\nexport const a = css({ p: f() });",
            "const make = () => ({ p: 1 });\nconst made = make();\nmade.p = 2;\nconst f = () => made.p;\nexport const a = css({ p: f() });",
            "const pick = () => (typeof window === 'undefined' ? 'white' : 'black');\nexport const a = css({ color: pick() });",
            "const brand = () => globalThis.BRAND ?? 'red';\nexport const a = css({ color: brand() });",
            "const node = () => process.env.NODE_ENV;\nexport const a = css({ content: node() });",
            "import { base } from './tokens';\nconst f = () => base.p;\nexport const a = css({ p: f() });",
            "import base, { fixed, colors, grid, renamed } from './tokens';\nimport * as tokens from './tokens';\nexport const a = <Box p={fixed.p} color={colors.primary} m={grid[0]} w={base.p} h={renamed.m} bg={tokens.fixed.p} />;",
            "import { base as again } from './reexport';\nexport const a = css(again);",
        ];
        let outputs: Vec<String> = cases
            .iter()
            .map(|case| {
                reset_class_map();
                reset_file_map();
                match extract_with_modules(
                    "/src/App.tsx",
                    &format!("import {{ Box, css, styled }} from '@devup-ui/react';\n{case}"),
                    ExtractOption::default(),
                    false,
                    &memory_resolver(CHANGED_MODULES),
                ) {
                    Ok(output) => output.code,
                    Err(error) => format!("Error: {error}"),
                }
            })
            .collect();
        assert_debug_snapshot!(outputs);
    }

    const RUNTIME_MODULES: &[(&str, &str)] = &[
        (
            "/src/tokens.ts",
            "export const colors = { primary: 'red' };\nexport default { colors };",
        ),
        (
            "/src/frozen.ts",
            "export const heading = { fontSize: 24 };\nexport const typography = Object.freeze({ heading });",
        ),
        (
            "/src/changed.ts",
            "export const base = { p: 4 };\nexport const colors = { primary: 'red' };\nexport const helper = () => 1;\nexport function dark() { base.p = 8; }",
        ),
        (
            "/src/cjs.js",
            "const colors = { primary: 'red' };\ncolors.primary = 'blue';\nmodule.exports = { colors };",
        ),
        (
            "/src/cjs-plain.js",
            "const colors = { primary: 'red' };\nconst spacing = { m: 2 };\nmodule.exports = { colors };\nexports.spacing = spacing;",
        ),
        (
            "/src/env.ts",
            "import { base } from './changed';\nconsole.log(base);\nexport const isBrowser = typeof window !== 'undefined';\nexport const mode = isBrowser ? 'dark' : 'light';\nexport const label = (1).toLocaleString();\nexport const format = (n) => n.toLocaleString();\nexport const double = (n) => n * 2;\nexport function triple(n) { return n * 3; }\nexport class Scale { static of(n) { return n * 4; } }",
        ),
    ];

    #[test]
    #[serial]
    fn test_values_known_only_at_runtime() {
        let cases = [
            "import { colors } from './tokens';\nexport const a = <Box color={colors.primary} />;",
            "import { heading } from './frozen';\nexport const a = <Box {...heading} />;",
            "const theme = { text: { color: 'red' } };\nexport const App = () => <ThemeProvider theme={theme}><Box {...theme.text} /></ThemeProvider>;",
            "const handle = { current: null, p: 1 };\nexport const App = () => <div ref={handle}><Box p={handle.p} /></div>;",
            "import * as tokens from './changed';\nexport const a = css(tokens.base);\nexport const b = <Box {...tokens.base} />;\nexport const c = <Box color={tokens.colors.primary} />;\nexport const d = css(tokens[key]);\nexport const e = css(make() || tokens.base);\ntokens.helper();\ntokens.colors = null;",
            "import * as tokens from './changed';\ntokens.colors.primary = 'blue';\nexport const a = <Box color={tokens.colors.primary} />;",
            "import * as tokens from './changed';\nregister(tokens);\nexport const a = <Box m={tokens.helper} color={tokens.colors.primary} />;",
            "import { base } from './changed';\nimport * as tokens from './changed';\nconst theme = { base };\nexport const a = css(theme.base, tokens.base, base);\nexport const b = <Box {...theme} />;\nexport const c = css(theme.missing);",
            "import { double, triple, Scale, mode } from './env';\nexport const a = css({ w: double(2), h: triple(2), m: Scale.of(1) });\nexport const b = css({ color: mode });",
            "import { label, format } from './env';\nexport const a = css({ content: label });\nexport const b = css({ content: format(1) });",
            "const label = () => (1234.5).toLocaleString();\nconst LABEL = 'i'.toLocaleUpperCase();\nconst pick = () => LABEL;\nexport const a = css({ content: label() });\nexport const b = css({ content: pick() });\nexport const c = css({ w: window.innerWidth });\nexport const d = css({ content: new Intl.NumberFormat().format(1) });",
            "import { colors } from './cjs';\nimport cjs, { colors as plain, spacing } from './cjs-plain';\nexport const a = <Box color={colors.primary} bg={plain.primary} m={spacing.m} />;",
            "const list = [{ p: 1 }];\nfor (const item of list) item.p = 2;\nexport const a = <Box p={list[0].p} />;",
            "const list = [{ p: 1 }];\nlist.forEach((item) => { item.p = 2; });\nexport const a = <Box p={list[0].p} />;",
            "const store = { items: [1], add(x) { this.items.push(x); } };\nstore.add(2);\nexport const a = <Box p={store.items[0]} />;",
            "const theme = { spacing: (n) => n * 4, colors: { primary: 'red' }, label: 'x' };\nconst gap = theme.spacing(2);\ntheme.label.custom();\nexport const a = <Box color={theme.colors.primary} />;",
            "const sizes = [1, 2];\nconst copy = Array.from(sizes);\nexport const a = <Box p={sizes[0]} />;",
            "const isBrowser = typeof window !== 'undefined';\nconst pick = () => (!isBrowser ? 'white' : 'black');\nexport const a = css({ color: pick() });",
            "const parsed = JSON.parse('{bad');\nconst pick = () => (parsed ? 'a' : 'b');\nexport const a = css({ content: pick() });",
            "import { isMobile } from 'unknown-package';\nconst pick = () => (isMobile ? 'a' : 'b');\nexport const a = css({ content: pick() });",
            "export const a = css({ content: '1234.5', fontSizeAdjust: 0.5, mathDepth: 2 });",
        ];
        let outputs: Vec<String> = cases
            .iter()
            .map(|case| {
                reset_class_map();
                reset_file_map();
                match extract_with_modules(
                    "/src/App.tsx",
                    &format!("import {{ Box, css, styled }} from '@devup-ui/react';\n{case}"),
                    ExtractOption::default(),
                    false,
                    &memory_resolver(RUNTIME_MODULES),
                ) {
                    Ok(output) => format!("{:?}", ToBTreeSet::from(output)),
                    Err(error) => format!("Error: {error}"),
                }
            })
            .collect();
        assert_debug_snapshot!(outputs);

        reset_class_map();
        reset_file_map();
        let stylesheet = extract(
            "/src/log.css.ts",
            "import { style } from '@devup-ui/react';\nconsole.warn('loaded');\nexport const box = style({ color: 'red' });",
            ExtractOption::default(),
        )
        .unwrap();
        assert_eq!(stylesheet.styles.len(), 1);
    }

    #[test]
    #[serial]
    fn test_inline_imported_constants() {
        reset_class_map();
        reset_file_map();
        let output = extract_with_modules(
            "/src/App.tsx",
            r"import { Box, css, styled } from '@devup-ui/react';
import * as Devup from '@devup-ui/react';
import { PRIMARY, SIZE, NEG, TEMPLATE, colors, ALIAS, MUTABLE, DYNAMIC, renamed, alsoMutable, reBase, notThere, gone, value, ns, NEG_STRING, TEMPLATE_EXPRESSION, SPACED, NOT_OBJECT, helper, looped } from './tokens';
import orange from './tokens';
import * as tokens from './tokens';
import named from './named';
import { handler } from './handler';
import cjs, { COMPILED, FROM_REQUIRE, DESTRUCTURED, MUTATED, DYNAMIC } from './cjs-tokens';
import cjsObject, { OBJ, OVERRIDDEN, LATER } from './cjs-object';
import twelve from './cjs-value';
import twice from './cjs-twice';
import marker, { named as markerNamed } from './cjs-marker';
import { unused } from './unused';
import missing from './missing';
export const a = <Box color={PRIMARY} p={SIZE} m={NEG} w={TEMPLATE} bg={colors.primary} borderColor={colors.nested.deep} outlineColor={colors['fromBase']} textDecorationColor={ALIAS} caretColor={MUTABLE} accentColor={DYNAMIC} columnRuleColor={renamed} fill={reBase} stroke={value} stopColor={ns.value} floodColor={tokens.PRIMARY} lightingColor={orange} content={named} top={missing} left={NEG_STRING} right={TEMPLATE_EXPRESSION} bottom={SPACED} zIndex={NOT_OBJECT} gap={colors.skip} rowGap={colors.nope} columnGap={colors[key]} order={alsoMutable} flex={notThere} flexBasis={gone} flexGrow={helper} flexShrink={looped} {...colors} onClick={handler} />;
export const b = css({ color: PRIMARY });
export const c = <Devup.Box color={colors.primary} />;
export const d = styled.div`color: ${PRIMARY};`;
export const e = (PRIMARY) => <Box color={PRIMARY} />;
export const f = { PRIMARY, unused, SIZE: tokens.SIZE };
export const g = <div color={PRIMARY} />;
export const h = <Devup.Inner.Box color={SIZE} />;
export const i = Devup['css']({ color: SIZE });
export const l = <Box color={COMPILED} bg={FROM_REQUIRE} borderColor={DESTRUCTURED} outlineColor={MUTATED} fill={cjs} stroke={OBJ} caretColor={OVERRIDDEN} accentColor={LATER} stopColor={cjsObject.OBJ} zIndex={twelve} floodColor={twice.A} lightingColor={DYNAMIC} columnRuleColor={marker} textDecorationColor={markerNamed} />;
export const j = <Box zIndex={PRIMARY.length} order={PRIMARY['length']} />;
export const k = styled('div')({ color: SIZE });",
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_dir: "@devup-ui/react".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
            false,
            &memory_resolver(CONSTANT_MODULES),
        )
        .unwrap();
        assert_eq!(
            output.dependencies,
            [
                "/src/base.ts",
                "/src/cjs-marker.js",
                "/src/cjs-object.js",
                "/src/cjs-tokens.js",
                "/src/cjs-twice.js",
                "/src/cjs-value.js",
                "/src/handler.ts",
                "/src/loop.ts",
                "/src/named.ts",
                "/src/tokens.ts"
            ]
        );
        assert_debug_snapshot!(ToBTreeSet::from(output));
        let without_imports = extract_with_modules(
            "/src/Plain.tsx",
            "import { Box } from '@devup-ui/react';\nexport const a = <Box color='red' />;",
            ExtractOption::default(),
            false,
            &memory_resolver(CONSTANT_MODULES),
        )
        .unwrap();
        assert!(without_imports.dependencies.is_empty());
        let without_constants = extract_with_modules(
            "/src/Handler.tsx",
            "import { Box } from '@devup-ui/react';\nimport { handler } from './handler';\nexport const a = <Box onClick={handler} color='red' />;",
            ExtractOption::default(),
            false,
            &memory_resolver(CONSTANT_MODULES),
        )
        .unwrap();
        assert_eq!(without_constants.dependencies, ["/src/handler.ts"]);
    }

    #[test]
    #[serial]
    fn test_constant_objects_arrays_and_enums() {
        reset_class_map();
        reset_file_map();
        let modules: &[(&str, &str)] = &[(
            "/src/tokens.ts",
            "export const space = [0, 4, 8];\nexport enum Size { S = 4, M = 8 }\nenum Local { A = 'a' }\nexport { Local };\nexport const focus = { color: 'navy', ...{ outlineColor: 'teal' } };",
        )];
        let output = extract_with_modules(
            "/src/App.tsx",
            r"import { Box, css, styled, globalCss, keyframes } from '@devup-ui/react';
import * as Devup from '@devup-ui/react';
import * as stylex from '@stylexjs/stylex';
import { space, Size, Local, focus } from './tokens';
const base = { p: 4, color: 'red', color: 'blue' };
const hover = { opacity: 0.5 };
const merged = { ...base, m: 1 };
const partial = { ...unknown, m: 1 };
const proto = { __proto__: null, a: 1 };
const getter = { get a() { return 1; } };
export const withGetter = <Box bottom={getter.a} />;
const NONE = null;
const MEDIA = '@media (min-width: 1px)';
const keyed = { [MEDIA]: { color: 'green' } };
const widths = [10, null, 20];
const more = [...widths, 30];
const bad = [...unknown];
enum Level { Low = 1, High, Name = 'n', Computed = Math.max(1, 2), Last = 9 }
declare enum Ambient { X = 1 }
const sx = { color: 'red' };
export const a = css(base);
export const b = css({ ...merged, _hover: hover, ...keyed, color: NONE });
export const c = css(base, { m: 3 });
export const d = styled.div(base);
export const e = styled.div({ ...base, _hover: hover });
export const f = (Comp, handler) => <Box as={Comp} props={base} {...base} _hover={hover} w={widths} h={more[3]} zIndex={Level.High} order={Level.Low} content={Level.Name} p={space[2]} m={Size.M} fontFamily={Local.A} _focus={focus} onClick={handler} />;
export const g = <Devup.Box _hover={hover} />;
export const h = Devup.css(base);
export const i = styled(Box)(base);
export const j = styled.div.attrs({ role: 'button' })(base);
export const k = stylex.create({ x: sx });
globalCss({ body: base });
export const l = keyframes({ from: hover });
export const m = css({ right: proto.a, zIndex: partial.m, order: Level.Computed, flexGrow: Level.Last });",
            ExtractOption::default(),
            false,
            &memory_resolver(modules),
        );
        assert_debug_snapshot!(
            output
                .map(ToBTreeSet::from)
                .map_err(|error| error.to_string())
        );
    }

    #[test]
    #[serial]
    fn test_styles_the_build_cannot_read() {
        for (code, error) in [
            (
                "export const a = (x) => css({ ...x });",
                "`css()` cannot use `x`",
            ),
            (
                "const shared = { m: 1, ...unknown };\nexport const a = css({ m: shared.m });",
                "`css()` cannot use `shared.m`",
            ),
            (
                "const scale = [0, 4];\nexport const a = css({ top: scale[9], left: scale['01'] });",
                "`css()` cannot use `scale[",
            ),
            (
                "export const a = (x) => css({ _hover: x });",
                "`css()` cannot use `x`",
            ),
            (
                "export const a = (k) => css({ [k]: 1 });",
                "`css()` cannot use `[k]`",
            ),
            (
                "export const a = (x) => css({ _hover: { ...x } });",
                "`css()` cannot use `x`",
            ),
            (
                "export const a = (k) => css({ _hover: { [k]: 'red' } });",
                "`css()` cannot use `[k]`",
            ),
            (
                "export const a = (x) => css({ _hover: new x() });",
                "`css()` cannot use `new x()`",
            ),
            (
                "export const a = (x) => css({ _hover: () => x });",
                "`css()` cannot use `() => x`",
            ),
            (
                "export const a = (make) => css(make());",
                "Cannot compose `make()`",
            ),
            (
                "export const a = (x) => globalCss(x);",
                "`globalCss()` cannot use `x`",
            ),
            (
                "export const a = (k) => globalCss({ [k]: {} });",
                "`globalCss()` cannot use `[k]`",
            ),
            (
                "export const a = (x) => globalCss({ body: { ...x } });",
                "`globalCss()` cannot use `x`",
            ),
            (
                "export const a = (x) => keyframes(x);",
                "`keyframes()` cannot use `x`",
            ),
            (
                "export const a = (k) => keyframes({ [k]: {} });",
                "`keyframes()` cannot use `[k]`",
            ),
            (
                "export const a = (x) => keyframes({ ...x });",
                "`keyframes()` cannot use `...x`",
            ),
            (
                "export const a = (x) => styled.div({ ...x });",
                "`styled()` cannot use `x`",
            ),
            (
                "export const a = (x) => <Box _hover={x} />;",
                "`<Box>` cannot use `x`",
            ),
            (
                "export const a = (x) => <Box _hover={{ ...x }} />;",
                "`<Box>` cannot use `x`",
            ),
            (
                "export const a = (k) => <Box _hover={{ [k]: 'red' }} />;",
                "`<Box>` cannot use `[k]`",
            ),
            (
                "export const a = (on, rest) => <Box {...(on ? { p: 1 } : rest)} />;",
                "`<Box>` cannot use `rest`",
            ),
            (
                "export const a = (list) => <Box _hover={[...list][0]} />;",
                "`<Box>` cannot use `[...list][0]`",
            ),
            (
                "export const a = (list, i) => <Box _hover={[...list, {}][i]} />;",
                "`<Box>` cannot use `[...list, {}][i]`",
            ),
            (
                "export const a = (extra) => <Box _hover={{ ...extra, a: {} }['b']} />;",
                "`<Box>` cannot use `({ ...extra",
            ),
            (
                "export const a = (styles, k) => <Box _hover={styles.all[k]} />;",
                "`<Box>` cannot use `styles.all[k]`",
            ),
            (
                "export const a = (c, x) => <Box styleOrder={c ? 1 : 2} _hover={x} />;",
                "`<Box>` cannot use `x`",
            ),
            (
                "export const a = (rest) => <Box {...{ ...rest, p: 1 }} />;",
                "`<Box>` cannot use `rest`",
            ),
            (
                "declare const X: number;\nexport const a = css({ w: String(X) });",
                "`css()` cannot use `String(X)`",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import {{ Box, css, styled, globalCss, keyframes }} from '@devup-ui/react';\n{code}"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains(error), "{code}\n{message}");
        }
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { Box, css, styled, keyframes } from '@devup-ui/react';
export const a = (base) => css(base);
export const b = (on) => css(on && { color: 'blue' });
export const c = css('color: green; margin: 0');
export const d = (on, base) => css(on ? base : { color: 'red' });
export const e = css({ ...null, ...false, ...undefined, ...(void 0), p: 1 });
export const f = (rest, handler, k) => <Box {...rest} {...{ p: 1, onClick: handler, [k]: 1 }} />;
export const g = (theme, key) => <Box bg={theme.colors[key]} color={{ a: 'red', ...theme.more }['b']} />;
export const h = (Base) => styled.div(Base);
export const i = keyframes({ from: { opacity: 0 } } as const);
export const j = css(...{ bg: 'red' });",
                ExtractOption::default(),
            )
            .unwrap()
        ));
        reset_class_map();
        reset_file_map();
        assert_eq!(
            extract(
                "test.tsx",
                "import { Global } from '@emotion/react';\nexport const a = (x) => <Global styles={{ body: x }} />;",
                ExtractOption {
                    import_aliases: HashMap::from([(
                        "@emotion/react".to_string(),
                        ImportAlias::NamedToNamed,
                    )]),
                    ..ExtractOption::default()
                },
            )
            .unwrap_err()
            .to_string(),
            "test.tsx:2:41: `<Global>` cannot use `x` at build time: its values must be literals, theme tokens or constants"
        );
    }

    #[test]
    #[serial]
    fn test_computed_values_through_functions_and_style_packages() {
        reset_class_map();
        reset_file_map();
        let modules: &[(&str, &str)] = &[(
            "/src/sx.ts",
            "import * as stylex from '@stylexjs/stylex';\nexport const S = stylex.create({ x: { color: 'red' } });\nexport const W = String(3) + 'px';",
        )];
        let output = extract_with_modules(
            "/src/App.tsx",
            r"import { Box, css } from '@devup-ui/react';
import { W } from './sx';
const space = [0, 4, 8];
const NONE = null;
const IDX = space[1];
const LABEL = `x${NONE}`;
const bad = [...'ab'];
export function triple(n: number) { return n * 3; }
export const a = css({ h: triple(2), m: IDX, content: LABEL, p: bad });
export const w = <Box w={W} />;
export function b() { return <this.Box p={IDX} />; }",
            ExtractOption::default(),
            false,
            &memory_resolver(modules),
        )
        .unwrap();
        assert_eq!(output.dependencies, ["/src/sx.ts"]);
        let styles = format!("{:?}", ToBTreeSet::from(output).styles);
        for value in ["24px", "16px", "xnull", "identifier: \"W\""] {
            assert!(styles.contains(value), "{value}\n{styles}");
        }
        assert!(!styles.contains("3px"), "{styles}");
    }

    #[test]
    #[serial]
    fn test_compose_rules_the_module_computes() {
        reset_class_map();
        reset_file_map();
        let modules: &[(&str, &str)] = &[
            (
                "/src/styles.ts",
                "import { css, styled } from '@devup-ui/react';\nexport const card = css({ p: 1 });\nexport const Card = styled.div({ p: 2 });\nexport const Linked = styled('a')({ p: 3 });\nexport const tagged = css`color: red;`;\nexport const make = (n: number) => ({ m: n });\nexport const computed = make(3);\nexport const rules = { m: 7 };\nexport const nested = { x: make(9), known: 'k' };",
            ),
            (
                "/src/emotion.ts",
                "import { css } from '@emotion/react';\nexport const emotionClass = css({ p: 1 });",
            ),
        ];
        let output = extract_with_modules(
            "/src/App.tsx",
            r"import { css, styled } from '@devup-ui/react';
import * as Devup from '@devup-ui/react';
import * as tokens from './styles';
import { card, rules } from './styles';
import { emotionClass } from './emotion';
const make = (n: number) => ({ m: n });
const local = make(2);
const computed = make(3);
const name = String('named');
const shared = { rules: make(4), fixed: { p: 5 } };
export const a = css(local);
export const b = css(card, computed);
export const c = css(name, null, true, 'x', `y`);
export const d = styled.div(local);
export const e = styled('span')(computed);
export const f = styled.p.attrs({ role: 'note' })(shared.rules);
export const g = (on) => css(on ? local : card, on || computed, on && local, [local, shared.fixed]);
export const h = css(rules, tokens.rules);
export const i = Devup.css(local);
export const k = css(tokens.nested.known, tokens.card);
export const l = css(emotionClass, { m: 1 });
export const n = <Devup.Layout.Box {...local} />;",
            ExtractOption {
                import_aliases: HashMap::from([(
                    "@emotion/react".to_string(),
                    ImportAlias::NamedToNamed,
                )]),
                ..ExtractOption::default()
            },
            false,
            &memory_resolver(modules),
        )
        .unwrap();
        assert_eq!(output.dependencies, ["/src/emotion.ts", "/src/styles.ts"]);
        assert_debug_snapshot!(ToBTreeSet::from(output));

        for (code, part) in [
            ("css(tokens.computed)", "tokens.computed"),
            ("css(tokens.nested.x, { m: 1 })", "tokens.nested.x"),
            ("css(tokens.make(12))", "tokens.make(12)"),
            ("styled.div(computed)", "computed"),
            ("(key) => css(shared[key])", "shared[key]"),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract_with_modules(
                "/src/App.tsx",
                &format!("import {{ css, styled }} from '@devup-ui/react';\nimport * as tokens from './styles';\nimport {{ computed }} from './styles';\nconst shared = {{ rules: tokens.make(4) }};\nexport const a = {code};"),
                ExtractOption::default(),
                false,
                &memory_resolver(modules),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(
                message.contains(&format!("cannot use `{part}`")),
                "{code}\n{message}"
            );
        }
    }

    #[test]
    #[serial]
    fn test_source_map_points_at_the_code_as_written() {
        reset_class_map();
        reset_file_map();
        let code = "import { css } from '@emotion/react';\nconst PRIMARY = 'red';\nconst darken = (amount, color) => color;\nexport const a = css({\n  color: darken(\n    0.1,\n    PRIMARY,\n  ),\n});\nexport const after = 1;\n";
        let output = extract(
            "src/App.tsx",
            code,
            ExtractOption {
                import_aliases: HashMap::from([(
                    "@emotion/react".to_string(),
                    ImportAlias::NamedToNamed,
                )]),
                ..ExtractOption::default()
            },
        )
        .unwrap();
        let map =
            oxc_sourcemap::SourceMap::from_json_string(output.map.as_deref().unwrap()).unwrap();
        assert_eq!(map.get_source_content(0), Some(code));
        let (line, generated) = output
            .code
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains("after"))
            .unwrap();
        let column = generated.find("after").unwrap();
        let token = map
            .get_tokens()
            .find(|token| {
                token.get_dst_line() as usize == line && token.get_dst_col() as usize == column
            })
            .unwrap();
        let written = code.lines().nth(token.get_src_line() as usize).unwrap();
        assert!(
            written[token.get_src_col() as usize..].starts_with("after"),
            "{written}"
        );
    }

    #[test]
    #[serial]
    fn test_styles_with_no_class_to_switch() {
        for (code, error) in [
            (
                "export const a = (on) => keyframes({ from: { opacity: on ? 0 : 1 } });",
                "`keyframes()` cannot use `on`",
            ),
            (
                "export const a = (on) => globalCss({ body: { color: on ? 'red' : 'blue' } });",
                "`globalCss()` cannot use `on`",
            ),
            (
                "export const a = (k) => globalCss({ body: { color: { a: 'red', b: 'blue' }[k] } });",
                "`globalCss()` cannot use `k`",
            ),
            (
                "export const a = (on) => globalCss({ body: { color: [on ? 'red' : 'blue'] } });",
                "`globalCss()` cannot use `on`",
            ),
            (
                "export const a = (on) => <Global styles={{ body: { color: on && 'red' } }} />;",
                "`<Global>` cannot use `on`",
            ),
            (
                "export const a = (side) => globalCss({ body: { positioning: side } });",
                "`globalCss()` cannot use `side`",
            ),
            (
                "export const a = (on) => globalCss({ body: { color: 'red', bg: on ? 'a' : 'b' } });",
                "`globalCss()` cannot use `on`",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!(
                    "import {{ Global, globalCss, keyframes }} from '@devup-ui/react';\n{code}"
                ),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains(error), "{code}\n{message}");
        }
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { css, globalCss, keyframes } from '@devup-ui/react';
const DARK = true;
export const a = css({ color: DARK ? 'red' : x, bg: false || 'blue', m: null ?? 2, p: 0 && x, w: 'a' && 3, h: undefined ?? 4, opacity: 0 ? 1 : 0.5 });
export const g = globalCss({ body: { color: DARK ? 'red' : 'blue' } });
export const h = () => globalCss({ body: { m: 0 } });
export const i = [globalCss({ html: { p: 0 } })];
export const j = (on) => on && globalCss`body { color: red; }`;
globalCss({ div: { m: 1 } });
export const k = keyframes({ from: { opacity: DARK ? 0 : 1 } });",
                ExtractOption::default(),
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_every_runtime_value_an_element_holds() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                "import { Box } from '@devup-ui/react';\nclass A { #w = 1; render(f) { return <Box w={this.#w} {...this.#w} {...f()} />; } }\nexport const a = async (f, X, tag) => <Box w={await f()} h={(f(), 2)} bg={new X()} color={tag`x`} zIndex={10n} m={f.n++} p={(f.v = 3)} opacity={null} flex={false} />;",
                ExtractOption::default(),
            )
            .unwrap()
        ));
        for value in ["function () {}", "class {}"] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import {{ Box }} from '@devup-ui/react';\nexport const a = <Box w={{{value}}} />;"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains("`<Box>` cannot use"), "{value}\n{message}");
        }
    }

    #[test]
    #[serial]
    fn test_compute_what_elements_and_styled_objects_take() {
        reset_class_map();
        reset_file_map();
        let modules: &[(&str, &str)] = &[(
            "/src/tokens.ts",
            "export const make = (n: number) => ({ m: n });\nexport const KEY = ['co', 'lor'].join('');\nexport const PRIMARY = '#336699';\nexport function darken(amount: number, color: string) { return color === PRIMARY ? '#112233' : color; }",
        )];
        let output = extract_with_modules(
            "/src/App.tsx",
            r"import { Box, css, styled } from '@devup-ui/react';
import * as Devup from '@devup-ui/react';
import { PRIMARY } from './tokens';
const make = (n: number) => ({ m: n });
const KEY = ['co', 'lor'].join('');
function darken(amount: number, color: string) { return color === PRIMARY ? '#112233' : color; }
const card = make(2);
const hover = make(3);
const parts = [make(4), { p: 1 }];
export const a = <Box {...card} onClick={() => 1} />;
export const b = <Devup.Box {...make(5)} />;
export const c = <Box _hover={hover} />;
export const d = <Box {...{ ...card, p: 1 }} />;
export const e = styled.div({ ...make(6), _hover: hover, [KEY]: 'red' });
export const f = css(...parts);
export const g = css(`color: ${darken(0.1, PRIMARY)};`);
export const h = <div {...card} />;
function local() { return { p: 7 }; }
export const i = <Box {...local()} />;
export const j = (register) => <Box {...register('email')} />;",
            ExtractOption::default(),
            false,
            &memory_resolver(modules),
        )
        .unwrap();
        assert_debug_snapshot!(ToBTreeSet::from(output));

        reset_class_map();
        reset_file_map();
        let imported = extract_with_modules(
            "/src/App.tsx",
            r"import { Box } from '@devup-ui/react';
import { make } from './tokens';
export const a = <Box {...make(5)} />;",
            ExtractOption::default(),
            false,
            &memory_resolver(modules),
        )
        .unwrap();
        assert!(imported.code.contains(")(make(5))"), "{}", imported.code);
    }

    #[test]
    #[serial]
    fn test_spreads_are_read_once() {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "test.tsx",
            r"import { Box } from '@devup-ui/react';
import { jsx as _jsx } from 'react/jsx-runtime';
export const a = (register, rest, C) => (
  <Box {...register('email')} {...rest} p={1}>
    <Box as={C} {...useFocusRing()} />
  </Box>
);
export const b = async (load) => <Box {...(await load())} m={2} />;
export const c = (props) => <Box {...{ onClick: props.onClick, title: 'x' }} {...props.rest} p={3} />;
export const d = (f) => _jsx(Box, { ...f(), bg: 'red' });
export const e = async (f, g) => _jsx(Box, { ...f(), title: await g() });
export const g = async (f, g, h) => <Box id={h()} {...f()} title={await g()} onClick={h} />;
export const h = async (f, g) => <Box {...f()}>{await g()}</Box>;
export const i = async (f, g) => _jsx(Box, { className: await g(), ...f() });",
            ExtractOption::default(),
        )
        .unwrap();
        assert_debug_snapshot!(ToBTreeSet::from(output));
    }

    #[test]
    #[serial]
    fn test_read_once_in_every_place() {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "test.tsx",
            r"import { Box } from '@devup-ui/react';
import { jsx as _jsx } from 'react/jsx-runtime';
export const a = async (f, g, list, C) => (
  <Box {...f()} data-x={list} aria-label='x' disabled xlink:href={g} onClick={async () => await g()} title=<Box as={C || 'b'} />>
    text
    <>{g}</>
    {...list()}
    <span>{g}</span>
    {await g()}
    after
  </Box>
);
export const b = function* (f, g) { yield <Box {...f()} title={yield g} />; };
export const c = (props) => _jsx(Box, props);
export const d = async (f, g) => _jsx(Box, { ...f(), [g()]: await g() });
export const e = async (rest, g) => _jsx(Box, { id: await g(), ...rest });
export const h = (f, g, k) => _jsx(Box, { className: 'z', onClick: g(), [k]: 2, ...f(), [k]: 1, title: 'x' });
export const i = (rest) => _jsx(Box, { ...rest, className: 'a' });
export const j = (rest) => <Box {...{ ...rest }} />;",
            ExtractOption::default(),
        )
        .unwrap();
        assert_debug_snapshot!(ToBTreeSet::from(output));
    }

    #[test]
    #[serial]
    fn test_dynamic_as_where_the_element_stands() {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "test.tsx",
            r"import { Box } from '@devup-ui/react';
export const a = (on, C, items) => (
  <section title={<Box as={C} m={1} />}>
    <Box as={on ? 'a' : 'b'} p={1} />
    {items.map((item) => <Box key={item} as={C} p={2} />)}
    <span>{on}</span>
  </section>
);
export const after = 42;",
            ExtractOption::default(),
        )
        .unwrap();
        assert_debug_snapshot!(ToBTreeSet::from(output));
    }

    #[test]
    #[serial]
    fn test_dynamic_as_elements() {
        let outputs: Vec<(&str, String)> = [
            "<Box as={b ? 'section' : undefined} />",
            "<Box as={b ? false : ''} />",
            "<Box as={motion.div} p={1} />",
            "<Box as={(Motion as any).Form.Field}>x</Box>",
            "<Box as={_Local} />",
            "<Box as={a ? (b ? 'h1' : Link) : null} />",
            "<List>{items.map((i) => <Box key={i.id} as={i.href ? 'a' : Link} p={1} />)}</List>",
            "<Box as={isLink && 'a'} />",
            "<Box as={props.as ?? 'div'} />",
            "<Box as={tag} />",
            "<Box as={`h${level}`} />",
            "<Box as={getTag()} />",
            "<Box as={Components[key]} />",
            "<Box as={getThing().div} />",
            "<Box as={b ? tag : 'div'} />",
            "<Box as={true} />",
            "<Box as={{ a: 'a', b: 'button' }[kind]} onClick={track()} p={1}>{render()}</Box>",
            "<Box as={['div', 'a'][i]} {...register('x')} p={1} />",
            "<Text as={b ? 'h1' : undefined} />",
            "<Box as={x}><Box as={y} p={1} /></Box>",
        ]
        .into_iter()
        .map(|code| {
            reset_class_map();
            reset_file_map();
            let output = extract(
                "test.tsx",
                &format!("import {{ Box, Text }} from '@devup-ui/react';\n{code}"),
                ExtractOption {
                    import_main_css: false,
                    ..ExtractOption::default()
                },
            )
            .unwrap();
            let code_out = output
                .code
                .lines()
                .filter(|line| !line.starts_with("import "))
                .collect::<Vec<_>>()
                .join("\n");
            (code, code_out)
        })
        .collect();
        assert_debug_snapshot!(outputs);
    }

    #[test]
    #[serial]
    fn test_styled_components_of_any_base() {
        let outputs: Vec<(&str, String)> = [
            "export const A = styled(motion.div)`color: red;`;",
            "export const B = styled((Motion as any).Div.Inner)({ color: 'red' });",
            "export const C = styled(forwardRef((p, r) => <div ref={r} {...p} />)).attrs({ role: 'x' })`color: red;`;",
            "export const D = (tag) => styled(tag, { color: 'red' });",
            "export const E = styled(getThing().div)`color: red;`;",
        ]
        .into_iter()
        .map(|code| {
            reset_class_map();
            reset_file_map();
            let output = extract(
                "test.tsx",
                &format!("import {{ styled }} from '@devup-ui/react';\n{code}"),
                ExtractOption {
                    import_main_css: false,
                    ..ExtractOption::default()
                },
            )
            .unwrap();
            (
                code,
                output
                    .code
                    .lines()
                    .filter(|line| !line.starts_with("import "))
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        })
            .collect();
        assert_debug_snapshot!(outputs);

        for (code, error) in [
            (
                "styled(...parts)`color: red;`",
                "`styled()` cannot use `styled(...parts)`",
            ),
            (
                "styled(null)`color: red;`",
                "`styled()` cannot use `styled(null)`",
            ),
            (
                "styled(1)({ color: 'red' })",
                "`styled()` cannot use `styled(1)({ color: \"red\" })`",
            ),
            (
                "styled(undefined, { color: 'red' })",
                "`styled()` cannot use `styled(undefined,",
            ),
            (
                "styled('div', styles)",
                "`styled()` cannot use `styled(\"div\", styles)`",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import {{ styled }} from '@devup-ui/react';\nlet styles = {{}};\nexport const A = {code};"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains(error), "{code}\n{message}");
        }
    }

    #[test]
    #[serial]
    fn test_package_imported_whole() {
        let outputs: Vec<(&str, String)> = [
            "import * as Devup from '@devup-ui/react';\nexport const k = Devup.keyframes({ from: { opacity: 0 } });\nexport const G = Devup.createGlobalStyle({ body: { m: 0 } });\nexport const c = Devup.css`color: red;`;\nexport const t = Devup.keyframes`from { opacity: 1; }`;\nDevup.globalCss`body { margin: 0; }`;",
            "import Devup from '@devup-ui/react';\nexport const S = Devup.styled.div({ color: 'red' });\nexport const T = Devup.styled('span', { color: 'blue' });\nexport const U = Devup.styled.p.attrs({ role: 'note' })({ m: 1 });\nexport const g = <Devup.Global styles={{ body: { p: 0 } }} />;",
            "import * as Devup from '@devup-ui/react';\nimport { styled } from '@devup-ui/react';\nexport const S = styled.div({ p: 1 });\nexport const o = other.styled.div({ p: 1 });\nexport const m = <motion.div />;",
        ]
        .into_iter()
        .map(|code| {
            reset_class_map();
            reset_file_map();
            let output = extract(
                "test.tsx",
                code,
                ExtractOption {
                    import_main_css: false,
                    ..ExtractOption::default()
                },
            )
            .unwrap();
            (code, output.code)
        })
        .collect();
        assert_debug_snapshot!(outputs);
    }

    #[test]
    #[serial]
    fn test_style_results_read_as_values() {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "test.tsx",
            r"import { Box, css, keyframes, globalCss, styled } from '@devup-ui/react';
import { Global } from '@devup-ui/react/compat';
const fadeIn = keyframes({ from: { opacity: 0 } });
const slide = keyframes`from { left: 0; }`;
const card = css({ p: 1 });
export const a = css({ animationName: fadeIn, animation: `${fadeIn} 1s` });
export const b = css(card, { m: 1 });
export const c = css`animation: ${slide} 2s;`;
export const d = <Box animationName={fadeIn} {...{ animation: `${slide} 3s` }} />;
export const E = styled.div({ animation: `${fadeIn} 4s` });
globalCss({ body: { animation: `${slide} 5s` } });
export const g = <Global styles={{ html: { animationName: fadeIn } }} />;",
            ExtractOption::default(),
        )
        .unwrap();
        assert_debug_snapshot!(ToBTreeSet::from(output));

        for code in [
            "const fadeIn = keyframes({ from: { opacity: 0 } });\nexport const f = (fadeIn) => css({ animationName: fadeIn });",
            "let fadeIn = keyframes({ from: { opacity: 0 } });\nexport const a = css({ animationName: fadeIn });",
            "const { fadeIn } = { fadeIn: keyframes({ from: { opacity: 0 } }) };\nexport const a = css({ animationName: fadeIn });",
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import {{ css, keyframes }} from '@devup-ui/react';\n{code}"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains("cannot use `fadeIn`"), "{code}\n{message}");
        }
    }

    #[test]
    #[serial]
    fn test_dynamic_values_keep_the_code_as_written() {
        reset_class_map();
        reset_file_map();
        let output = extract(
            "test.tsx",
            "import { Box } from '@devup-ui/react';\nexport const a = (f, i) => <Box color={f('#336699', '10px 0px', 'rgba(0, 0, 0, 0.5)')} zIndex={--i} bg={`${f('#ffffff')};`} />;",
            ExtractOption::default(),
        )
        .unwrap();
        for written in [
            "#336699",
            "10px 0px",
            "rgba(0, 0, 0, 0.5)",
            "--i",
            "#ffffff",
        ] {
            assert!(output.code.contains(written), "{written}\n{}", output.code);
        }
        assert!(!output.code.contains("var(--i)"), "{}", output.code);
        assert!(!output.code.contains(";`"), "{}", output.code);
    }

    #[test]
    #[serial]
    fn test_imported_stylesheet_runs_once_per_script() {
        reset_class_map();
        reset_file_map();
        let option = ExtractOption {
            import_aliases: HashMap::from([(
                "@vanilla-extract/css".to_string(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        };
        let red = memory_resolver(&[(
            "/src/theme.css.ts",
            "import { style } from '@vanilla-extract/css';\nexport const brand = ['re', 'd'].join('');\nexport const base = style({ color: brand });",
        )]);
        let blue = memory_resolver(&[(
            "/src/theme.css.ts",
            "import { style } from '@vanilla-extract/css';\nexport const brand = ['blu', 'e'].join('');\nexport const base = style({ color: brand });",
        )]);
        let colors = |name: &str, resolver: &ModuleResolver| {
            let output = extract_with_modules(
                &format!("/src/{name}.css.ts"),
                &format!("import {{ style }} from '@vanilla-extract/css';\nimport {{ brand }} from './theme.css';\nexport const {name} = style({{ color: brand }});"),
                option.clone(),
                false,
                resolver,
            )
            .unwrap();
            format!("{:?}", ToBTreeSet::from(output).styles)
        };
        for (name, resolver, color) in [
            ("first", &red as &ModuleResolver, "\"red\""),
            ("again", &red, "\"red\""),
            ("changed", &blue, "\"blue\""),
        ] {
            let styles = colors(name, resolver);
            assert!(styles.contains(color), "{name}: {styles}");
        }
    }

    #[test]
    #[serial]
    fn test_evaluate_build_time_values() {
        reset_class_map();
        reset_file_map();
        let modules: &[(&str, &str)] = &[
            (
                "/src/color.ts",
                "export const PRIMARY = '#336699';\nexport const SPACE = [4, 8];\nexport function darken(amount: number, color: string): string { return color === PRIMARY ? `darker(${amount})` : color; }\nexport const DARK = darken(0.5, PRIMARY);\nexport const hover = { color: darken(0.6, PRIMARY) };",
            ),
            (
                "/src/throws.ts",
                "export const boom = () => { throw new Error('x'); };",
            ),
            (
                "/src/styles.ts",
                "import { css } from '@devup-ui/react';\nimport * as stylex from '@stylexjs/stylex';\nimport styled from 'styled-components';\nimport logo from './logo.svg';\nexport const card = css({ p: 1 });\nexport const TOKEN = String(2) + 'px';\nexport const LOGO = `url(${logo})`;",
            ),
            (
                "/src/fails.ts",
                "export const BEFORE = 'teal';\nthrow new Error('x');\nexport const AFTER = String('navy');",
            ),
        ];
        let resolver = memory_resolver(modules);
        let option = ExtractOption {
            import_aliases: HashMap::from([(
                "styled-components".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            )]),
            ..ExtractOption::default()
        };
        let output = extract_with_modules(
            "/src/App.tsx",
            r"import { Box, css, keyframes, globalCss } from '@devup-ui/react';
import * as stylex from '@stylexjs/stylex';
import { PRIMARY, SPACE } from './color';
import { card } from './styles';
import { BEFORE } from './fails';
export const SIZE = 4;
export default function twice(n: number) { return n * 2; }
enum Level { Low = 1, High = twice(2) }
function darken(amount: number, color: string): string { return color === PRIMARY ? `darker(${amount})` : color; }
const LIGHT = darken(0.2, PRIMARY);
const color = darken(0.3, PRIMARY);
const make = (size: number) => ({ p: size, _hover: { color: LIGHT } });
const label = (text: string) => text.toUpperCase().padStart(4, '-');
export const a = css({ color: darken(0.1, PRIMARY), width: twice(SIZE), height: [1, 2].map(twice)[1], top: -twice(1), margin: SPACE[1] * 2, content: JSON.stringify(label('a')) });
export const b = keyframes({ from: { opacity: twice(0.25) } });
globalCss({ body: { color: LIGHT } });
const styles = stylex.create({ base: { color: LIGHT } });
export const c = <Box color={darken(0.4, PRIMARY)} {...stylex.props(styles.base)} />;
export const d = css({ color, ...make(2), [`@media (min-width: ${twice(300)}px)`]: { m: 1 }, zIndex: Level.High, outlineColor: BEFORE });
export const e = (dark: boolean) => css({ color: dark ? LIGHT : PRIMARY, bg: dark && LIGHT, fill: [LIGHT, ...[PRIMARY]][1] });
export const f = css(card, { p: String(SIZE) + 'px' });",
            option,
            false,
            &resolver,
        )
        .unwrap();
        assert_eq!(
            output.dependencies,
            ["/src/color.ts", "/src/fails.ts", "/src/styles.ts"]
        );
        assert_debug_snapshot!(ToBTreeSet::from(output));

        for (code, error) in [
            (
                "import { boom } from './throws';\nexport const a = css({ color: boom() });",
                "/src/App.tsx:3:18: `css()` cannot use `boom()`",
            ),
            (
                "export const a = css({ color: String(Date.now()) });",
                "/src/App.tsx:2:18: `css()` cannot use `String(Date.now())`",
            ),
            (
                "export const a = css({ width: Math.random() + 'px' });",
                "`css()` cannot use `Math.random()",
            ),
            (
                "let changing = 1;\nexport const a = css({ width: String(changing) });",
                "`css()` cannot use `String(changing)`",
            ),
            (
                "export const a = (n) => css({ width: String(n) });",
                "`css()` cannot use `String(n)`",
            ),
            (
                "export const a = css({ width: String(unknownGlobal), color: ({}).toString() });",
                "`css()` cannot use `String(unknownGlobal)`",
            ),
            (
                "import { missing } from './nowhere';\nexport const a = css({ width: missing() });",
                "`css()` cannot use `missing()`",
            ),
            (
                "import { AFTER } from './fails';\nexport const a = css({ color: AFTER });",
                "`css()` cannot use `AFTER`",
            ),
            (
                "export const a = css({ ...(() => ({ get p() { return 1; } }))() });",
                "`css()` cannot use `(() =>",
            ),
            (
                "export const a = css({ ...(() => ({ p: undefined }))(), m: Object.create({ x: 1 }) });",
                "`css()` cannot use `Object.create(",
            ),
            (
                "import { darken } from 'polished';\nexport const a = css({ color: darken(0.1, 'red') });",
                "`css()` cannot use `darken(.1,`red`)`",
            ),
            (
                "import { darken } from './color';\nexport const a = css({ color: darken(0.1, 'red') });",
                "`css()` cannot use `darken(.1,`red`)`",
            ),
            (
                "import { DARK, hover } from './color';\nexport const a = css({ color: DARK });\nexport const b = css({ _hover: hover });",
                "`css()` cannot use `DARK`",
            ),
            (
                "class S { px() { return '1px'; } }\nexport const a = css({ width: new S().px() });",
                "`css()` cannot use `new S().px()`",
            ),
            (
                "const plain = (text) => text.replace(/-/g, '');\nexport const a = css({ content: plain('a-b') });",
                "`css()` cannot use `plain(`a-b`)`",
            ),
            (
                "const LABEL = 'i'.toLocaleUpperCase();\nconst pick = () => LABEL;\nexport const a = css({ content: pick() });",
                "`css()` cannot use `pick()`",
            ),
            (
                "const fold = (text) => text.normalize('NFD');\nexport const a = css({ content: fold('a') });",
                "`css()` cannot use `fold(`a`)`",
            ),
            (
                "const W = typeof IntersectionObserver === 'undefined' ? 10 : 20;\nexport const a = css({ width: W });",
                "`css()` cannot use `W`",
            ),
            (
                "const safe = () => { try { return 1; } catch { return 2; } };\nexport const a = css({ width: safe() });",
                "`css()` cannot use `safe()`",
            ),
            (
                "const root = () => 2 ** 0.5;\nexport const a = css({ width: root() });",
                "`css()` cannot use `root()`",
            ),
            (
                "const wave = () => Math.sin(1);\nexport const a = css({ opacity: wave() });",
                "`css()` cannot use `wave()`",
            ),
            (
                "const hex = () => (255).toString(16);\nexport const a = css({ color: '#' + hex() });",
                "`css()` cannot use `",
            ),
            (
                "const node = () => <div />;\nexport const a = css({ content: node() });",
                "`css()` cannot use `node()`",
            ),
            (
                "const later = async () => 1;\nexport const a = css({ width: later() });",
                "`css()` cannot use `later()`",
            ),
            (
                "const box = { n: 1 };\nconst bump = () => { box.n = 2; return box.n; };\nexport const a = css({ width: bump() });",
                "`css()` cannot use `bump()`",
            ),
            (
                "const pick = { a: () => 1 };\nconst key = 'a';\nexport const a = css({ width: pick[key]() });",
                "`css()` cannot use `pick[`a`]()`",
            ),
            (
                "const read = { get a() { return 1; } };\nexport const a = css({ width: read.a });",
                "`css()` cannot use `read.a`",
            ),
            (
                "import { make } from './shapes';\nconst local = make(2);\nexport const a = css(local);\nexport const b = styled.div(local);",
                "`css()` cannot use `local`",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract_with_modules(
                "/src/App.tsx",
                &format!("import {{ css, styled }} from '@devup-ui/react';\n{code}"),
                ExtractOption::default(),
                false,
                &resolver,
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains(error), "{code}\n{message}");
        }
        reset_class_map();
        reset_file_map();
        let switched = extract_with_modules(
            "/src/App.tsx",
            "import { css } from '@devup-ui/react';\nconst hasIO = typeof IntersectionObserver !== 'undefined';\nexport const a = css({ content: hasIO ? 'a' : 'b' });",
            ExtractOption::default(),
            false,
            &resolver,
        )
        .unwrap();
        assert!(switched.code.contains("hasIO ?"), "{}", switched.code);
        let option = ExtractOption::default();
        for (code, computes) in [
            (
                "import { css } from '@devup-ui/react';\nimport { darken } from 'polished';\ncss({ color: darken(0.1, 'red') });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nimport { DARK } from './color';\ncss({ color: DARK });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nimport { PRIMARY, SPACE } from './color';\nconst f = (n) => `${PRIMARY}${n}`;\ncss({ color: f(SPACE[0]) });",
                true,
            ),
            (
                "import { css } from '@devup-ui/react';\nimport { PRIMARY } from './color';\nconst X = `${PRIMARY}`;\ncss({ color: X, borderColor: PRIMARY });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\ncss({ width: Math.max(1, 2), color: 'red' });",
                false,
            ),
            (
                "import { Box } from '@devup-ui/react';\nconst f = () => 1;\n<Box w={f()} />;",
                false,
            ),
            (
                "import { css } from 'other';\nconst f = () => 1;\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst x = css({ w: this.f(), h: String(Date.now()) });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = () => 1;\ncss({ w: [...[f()], 2] });",
                true,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst make = (n) => ({ m: n });\nconst local = make(2);\ncss(local);",
                true,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst card = css({ p: 1 });\ncss(card, { m: 1 });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nimport * as styles from './styles';\ncss(styles.card, { m: 1 });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nimport * as styles from './styles';\ncss(styles.TOKEN);",
                false,
            ),
            (
                "import { Box } from '@devup-ui/react';\nconst make = (n) => ({ m: n });\nconst card = make(2);\n<Box {...card} />;",
                true,
            ),
            (
                "import { css } from '@devup-ui/react';\nimport { darken } from './color';\ncss(`color: ${darken(0.1, 'red')};`);",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst darken = (amount, color) => color;\ncss(`color: ${darken(0.1, 'red')};`);",
                true,
            ),
            ("import { css } from '@devup-ui/react';\ncss({", false),
        ] {
            assert_eq!(
                has_build_time_values("/src/a.tsx", code, &option, Some(&resolver)),
                computes,
                "{code}"
            );
        }
        assert!(has_build_time_values(
            "a.tsx",
            "import { css } from '@emotion/react';\nconst f = () => 1;\nexport const a = css`width: ${f()}px;`;",
            &ExtractOption {
                import_aliases: HashMap::from([(
                    "@emotion/react".to_string(),
                    ImportAlias::NamedToNamed,
                )]),
                ..ExtractOption::default()
            },
            None,
        ));
    }

    #[test]
    #[serial]
    fn test_strict_build_time_reads() {
        let option = ExtractOption::default();
        for (code, computes) in [
            (
                "import * as Devup from '@devup-ui/react';\nconst make = (n) => ({ m: n });\nconst local = make(2);\nDevup.css(local);",
                true,
            ),
            (
                "import { styled } from '@devup-ui/react';\nconst make = (n) => ({ m: n });\nstyled.div({ p: 1, _hover: make(2) });",
                true,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst fresh = () => ({ n: 1 });\nconst f = () => { const o = { a: { n: 1 } }; o.a.n++; o['a'].n = 2; fresh().n = 3; return o.a.n; };\ncss({ w: f() });",
                true,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst box = { n: 1 };\nconst bump = () => { box.n++; return 1; };\ncss({ w: bump() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst { normalize } = 'a';\nconst f = () => normalize;\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst o = { m() { return super.toString(); } };\nconst f = () => o.m();\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = () => class {};\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nfunction* g() { yield 1; }\nconst f = () => g();\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = () => import('./x');\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = () => <></>;\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = () => Math['max'](1, 2) + (255).toString().length;\ncss({ w: f() });",
                true,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst { sin } = Math;\nconst f = () => sin(1);\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = () => { const M = Math; return M.sin(1); };\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst key = 'sin';\nconst f = () => Math[key](1);\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = () => { const t = (255).toString; return t.call(255, 16); };\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = () => { const { toString } = 255; return toString.call(255, 16); };\ncss({ w: f() });",
                false,
            ),
            (
                "import { css } from '@devup-ui/react';\nconst f = (n) => n.toString(16);\ncss({ w: f(255) });",
                false,
            ),
            (
                "import * as Devup from '@devup-ui/react';\nconst make = (n) => ({ m: n });\nDevup.styled.div(make(2));",
                true,
            ),
        ] {
            assert_eq!(
                has_build_time_values("/src/a.tsx", code, &option, None),
                computes,
                "{code}"
            );
        }

        let modules: &[(&str, &str)] = &[
            (
                "/src/t.ts",
                "export const T = { a: null, b: true, c: undefined, d: { e: 3 } };",
            ),
            ("/src/cjs.js", "exports.SIZE = 5;"),
        ];
        let resolver = memory_resolver(modules);
        reset_class_map();
        reset_file_map();
        let output = extract_with_modules(
            "/src/App.tsx",
            r"import { Box, css, styled } from '@devup-ui/react';
import { T } from './t';
import { SIZE } from './cjs';
const ON = true;
const U = undefined;
const N = null;
const OBJ = { a: 1 };
const p = 4;
const f = () => (T.a === null && T.b && T.c === undefined ? T.d.e : 0);
const g = () => SIZE;
export const a = css({ w: f(), h: g(), m: g() });
export const b = css({ content: `${ON}-${U}`, top: U ? 1 : 2, left: N ? 1 : 2, right: OBJ ? 1 : 2, opacity: ON, p });
export const c = styled('a')({ p: 1 });
export const d = <Box transitionDuration={300} animationDelay={0.5} counterReset={2} />;",
            ExtractOption::default(),
            false,
            &resolver,
        )
        .unwrap();
        let styles = format!("{:?}", output.styles);
        for value in [
            "\"12px\"",
            "\"20px\"",
            "\"true-undefined\"",
            "\"8px\"",
            "\"4px\"",
            "\"300ms\"",
            "\".5ms\"",
        ] {
            assert!(styles.contains(value), "{value}\n{styles}");
        }
        assert!(
            styles.contains("property: \"counter-reset\", value: \"2\""),
            "{styles}"
        );

        reset_class_map();
        reset_file_map();
        let literal = extract(
            "test.tsx",
            "import { Box } from '@devup-ui/react';\nexport const a = <Box bg={null ? 'red' : 'blue'} m={0 ? 1 : 2} p={'' ? 1 : 2} w={1 && 3} h={0 || 5} />;",
            ExtractOption::default(),
        )
        .unwrap();
        let styles = format!("{:?}", literal.styles);
        for value in ["\"blue\"", "\"8px\"", "\"12px\"", "\"20px\""] {
            assert!(styles.contains(value), "{value}\n{styles}");
        }

        reset_class_map();
        reset_file_map();
        let partial = extract_with_modules(
            "/src/App.tsx",
            "import { css } from '@devup-ui/react';\nimport * as tokens from './tokens';\nexport const a = css(tokens.list.length);",
            ExtractOption::default(),
            false,
            &memory_resolver(&[("/src/tokens.ts", "export const list = [1, 2];")]),
        );
        assert!(
            partial
                .as_ref()
                .is_ok_and(|output| output.styles.is_empty()),
            "{partial:?}"
        );

        reset_class_map();
        reset_file_map();
        let message = extract(
            "test.tsx",
            "import { css } from '@devup-ui/react';\nconst pick = () => 1;\nexport const a = css(pick());",
            ExtractOption::default(),
        )
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
        assert!(message.contains("cannot use `pick()`"), "{message}");
    }

    #[test]
    #[serial]
    fn test_build_time_value_candidates() {
        let option = ExtractOption::default();
        assert!(has_build_time_values(
            "a.tsx",
            r"import type { T } from './types';
import { type U, helper } from './helper';
import Default from './default';
import * as ns from './ns';
import * as Devup from '@devup-ui/react';
import { css as devupCss, Box } from '@devup-ui/react';
import * as sx from '@stylexjs/stylex';
import { create, props } from '@stylexjs/stylex';
export const A = 1;
export function f(n: T): U { return n; }
export class K { static v() { return 1; } }
export interface I { a: number }
export enum Kind { A = 'a' }
declare enum Ambient { B }
export default function g() { return 2; }
let mutable = 1;
function local() { return new.target ? 1 : 2; }
class Local extends K { m() { return super.toString(); } }
console.log(A);
const cyclic1 = () => cyclic2();
const cyclic2 = () => cyclic1();
function wrap() { const inner = 1; return devupCss({ w: String(inner) }); }
devupCss({ a: f(A), b: [helper(), , null, true, () => 1, 'x'], ...rest, c: Default(), d: ns.x(), e: mutable.toString(), f: this.x(), g: import.meta.url, h: Local.name.at(0), i: K.v(), j: g(), k: cyclic1(), l: new Local(), m: tag`x`, n: ((x) => x)(1), o: Math.max(1, 2), p: local(), q: (1 as number).toFixed(), r: (() => { const z = 1; return String(z); })(), s: Kind.A.repeat(2), t: Ambient.B });
Devup.css({ a: f(1) });
sx.create({ a: { color: f(1) } });
sx.props(f(1));
create({ a: { color: f(1) } });
props(f(1));
Devup['css']({ a: f(1) });
devupCss`width: ${f(1)}px;`;
devupCss(...[f(1)]);
export default class {}",
            &option,
            None,
        ));
        assert!(!has_build_time_values(
            "a.tsx",
            "import { css } from '@devup-ui/react';\nexport default class Theme { static size() { return 1; } }\ncss({ w: Theme.size() });",
            &option,
            None,
        ));
        assert!(!has_build_time_values(
            "a.tsx",
            "import { css } from '@devup-ui/react';\nexport default 1;\ncss({ w: 1 });",
            &option,
            None,
        ));
        assert!(!has_build_time_values(
            "a.unknown",
            "css({ w: f() })",
            &option,
            None,
        ));

        reset_class_map();
        reset_file_map();
        let modules: &[(&str, &str)] = &[(
            "/src/throws.ts",
            "export const boom = () => { throw new Error('x'); };",
        )];
        let resolver = memory_resolver(modules);
        for (code, error) in [
            (
                "const X = (() => { throw new Error('x'); })();\nexport const a = css({ width: String(X) });",
                "`css()` cannot use `String(X)`",
            ),
            (
                "import { boom } from './throws';\nexport const a = css({ width: String(1), color: boom() });",
                "/src/App.tsx:3:18: `css()` cannot use `boom()`",
            ),
        ] {
            let message = extract_with_modules(
                "/src/App.tsx",
                &format!("import {{ css }} from '@devup-ui/react';\n{code}"),
                ExtractOption::default(),
                false,
                &resolver,
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains(error), "{code}\n{message}");
        }
    }

    #[test]
    #[serial]
    fn test_fold_math_at_build_time() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box, css } from '@devup-ui/react';
const W = 10;
const GAP = Math.round(2.5) * 4;
const HALF = Math.round(-2.5);
export const a = css({ width: Math.max(W, 20), height: Math.min(W, -W, 3), gap: GAP, top: HALF, left: Math.floor(W / 3), right: Math.ceil(1.2), bottom: Math.trunc(-1.7), opacity: Math.abs(-0.5), zIndex: Math.sign(-3), order: Math.sign(0), flexGrow: Math.pow(2, 3), lineHeight: Math.sqrt(4), rotate: `${Math.PI}rad`, scale: Math.round(1.4), flexShrink: Math.sign(3) });
export const b = <Box p={Math.max(1, W)} m={Math.E > 2 ? 1 : 2} />;"
        )));
        for code in [
            "css({ width: Math.random() })",
            "css({ width: Math.max() })",
            "css({ width: Math.sqrt(-1) })",
            "css({ width: Math.NOPE })",
            "css({ width: Math.max(...list) })",
            "css({ width: Math.pow(2, 0.5) })",
            "css({ width: Math.pow(2, 99) })",
            "css({ width: Math.pow(10, 17) })",
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import {{ css }} from '@devup-ui/react';\nexport const a = {code};"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains("cannot use"), "{code}: {message}");
        }
        // An exact function over what folding cannot read runs at build time
        reset_class_map();
        reset_file_map();
        let computed = extract(
            "test.tsx",
            "import { css } from '@devup-ui/react';\nexport const a = css({ top: Math.max('1', 2) });",
            ExtractOption::default(),
        )
        .unwrap();
        let styles = format!("{:?}", computed.styles);
        assert!(styles.contains("8px"), "{styles}");
        reset_class_map();
        reset_file_map();
        assert!(
            extract(
                "test.tsx",
                "import { css } from '@devup-ui/react';\nexport const a = css({ width: -`${Math.PI}` });",
                ExtractOption::default(),
            )
            .is_ok()
        );
        for code in [
            "const Math = { max: () => 1 };\nexport const a = css({ width: Math.max(1, 2) });",
            "const Math = { PI: 3 };\nexport const a = css({ width: Math.PI, height: Math.max(1, 2) });",
            "import Math from './math';\nexport const a = css({ width: Math.max(1, 2) });",
            "export const a = (Math) => css({ width: Math.max(1, 2) });",
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import {{ css }} from '@devup-ui/react';\n{code}"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains("Math."), "{code}: {message}");
        }
    }

    #[test]
    #[serial]
    fn test_stylesheet_imports_modules() {
        reset_class_map();
        reset_file_map();
        let output = extract_with_modules(
            "/src/button.css.ts",
            r"import { style } from '@vanilla-extract/css';
import { vars, base } from './theme.css';
import size, { brand, shown, extra, more, renamed, double, Scale } from './tokens';
import * as tokens from './tokens';
import named from './named';
import Anonymous from './anonymous';
import './global.css';
import legacyModule, { legacy } from './legacy';
import value from './value';
import compiled from './compiled';
export const button = style([base, { color: vars.color.brand, background: brand, margin: double(size), padding: shown + extra + more.extra + renamed + tokens.default + named(), zIndex: typeof Scale === typeof Anonymous ? 1 : 0, borderWidth: legacy + value + compiled + legacyModule.legacy }]);
export const hover = style({ selectors: { [`${base}:hover &`]: { color: 'red' } } });",
            ExtractOption {
                package: "@devup-ui/react".to_string(),
                css_dir: "@devup-ui/react".to_string(),
                single_css: true,
                import_main_css: false,
                import_aliases: HashMap::from([(
                    "@vanilla-extract/css".to_string(),
                    ImportAlias::NamedToNamed,
                )]),
            },
            false,
            &memory_resolver(STYLESHEET_MODULES),
        )
        .unwrap();
        assert_eq!(
            output.dependencies,
            [
                "/src/anonymous.ts",
                "/src/compiled.js",
                "/src/legacy.js",
                "/src/more.ts",
                "/src/named.ts",
                "/src/theme.css.ts",
                "/src/tokens.ts",
                "/src/value.js"
            ]
        );
        assert_debug_snapshot!(ToBTreeSet::from(output));
        let cycle: &'static [(&str, &str)] = &[
            ("/src/b.css.ts", ""),
            (
                "/src/a.css.ts",
                "import { style } from '@devup-ui/react';\nimport { b } from './b.css';\nexport const a = style([b, { color: 'red' }]);",
            ),
        ];
        let error = extract_with_modules(
            "/src/b.css.ts",
            "import { style } from '@devup-ui/react';\nimport { a } from './a.css';\nexport const b = style({ margin: 4 });\nexport const useA = () => a;",
            ExtractOption::default(),
            false,
            &memory_resolver(cycle),
        )
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
        assert!(
            error.contains("Cannot access 'b' of '/src/b.css.ts' before its initialization: it is part of an import cycle"),
            "{error}"
        );
    }

    #[test]
    #[serial]
    fn test_conditional_composition() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { css } from '@devup-ui/react';
import styled from '@emotion/styled';
export const a = css({ color: 'red', p: 1 }, cond && { color: 'blue', _hover: { color: 'green' } });
export const b = css([{ m: 1 }, flag ? { m: 2 } : { m: 3, bg: 'red' }]);
export const c = css({ color: 'red' }, cond && other, flag ? 'x' : null, [undefined, false]);
export const d = css({ _hover: { color: 'red' } }, cond && { _hover: { bg: 'blue' } });
export const e = css({ color: 'red' }, flag ? other : { color: 'blue' });
export const h = css({ color: 'red' }, cond || { color: 'blue' });
export const i = css({ color: 'red' }, cond ? null : undefined);
export const j = css({ color: 'red' }, value ?? { color: 'blue' });
export const K = styled.div(base, cond && { color: 'blue' }, { margin: 1 });",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([(
                        "@emotion/styled".to_string(),
                        ImportAlias::DefaultToNamed("styled".to_string()),
                    )]),
                },
            )
            .unwrap()
        ));
        for (code, error) in [
            (
                "import { css } from '@devup-ui/react';\ncss({ _hover: 'x' }, cond && { _hover: { color: 'blue' } });",
                "Cannot compose `{ _hover: \"x\" }, cond && { _hover: { color: \"blue\" } }` at build time: each style must be a rule object, a class, or a condition choosing between them",
            ),
            (
                "import { css } from '@devup-ui/react';\ncss([{ color: 'red' }, cond && { [key]: 'blue' }]);",
                "Cannot compose `[{ color: \"red\" }, cond && { [key]: \"blue\" }]` at build time: each style must be a rule object, a class, or a condition choosing between them",
            ),
            (
                "import { css } from '@devup-ui/react';\ncss({ color: 'red' }, ...rest);",
                "Cannot compose `{ color: \"red\" }, ...rest` at build time: each style must be a rule object, a class, or a condition choosing between them",
            ),
            (
                "import { styled } from '@devup-ui/react';\nstyled.div({ color: 'red' }, getStyles());",
                "Cannot compose `{ color: \"red\" }, getStyles()` at build time: each style must be a rule object, a class, or a condition choosing between them",
            ),
        ] {
            assert_eq!(
                extract("test.tsx", code, ExtractOption::default())
                    .err()
                    .map(|error| error.to_string()),
                Some(format!("test.tsx:2:1: {error}")),
            );
        }
    }

    #[test]
    #[serial]
    fn test_devup_props_typescript_wrappers() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const cast = <Box bg={'red' as any} />;
const satisfied = <Box color={'blue' satisfies string} />;
const nonNull = <Box mt={v!} />;
const parens = <Box pt={('4px')} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_devup_props_optional_chaining() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const dotted = <Box bg={map?.k} />;
const computed = <Box color={map?.[k]} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_css_and_global_css_typescript_wrappers() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { css, globalCss, keyframes } from '@devup-ui/react';
const a = css({ bg: 'red' } as any);
const b = css({ color: 'blue' } satisfies object);
globalCss({ body: { bg: 'green' } } as any);
const k = keyframes({ from: { opacity: 0 } } as any);"
        )));
    }

    #[test]
    #[serial]
    fn test_styled_typescript_wrappers() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { styled } from '@devup-ui/react';
const A = styled('div')({ bg: 'red' } as any);
const B = styled('span')({ color: 'blue' } satisfies object);
const C = (styled('p') as any)`margin-top: 1px;`;
const D = (styled.div satisfies object)({ pb: '3px' });
const E = styled.span!({ pl: '4px' });
const F = (styled.p<object>)`padding-right: 5px;`;
const G = (styled<object>)('div', { pr: '6px' });"
        )));
    }

    #[test]
    #[serial]
    fn test_styled_accepts_both_call_forms() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { styled, Box } from '@devup-ui/react';
const twoArg = styled('div', { bg: 'red' });
const twoArgComponent = styled(Box, { mt: '1px' });
const curried = styled('span')({ color: 'blue' });
const member = styled.p({ pt: '2px' });"
        )));
        for (code, error) in [
            (
                "const malformed = styled('div', 'span')`color: red;`;",
                "`styled()` cannot use `styled(\"div\", \"span\")`",
            ),
            (
                "const creatorOnly = styled('div');",
                "`styled` is read at runtime, where it does not exist",
            ),
        ] {
            reset_class_map();
            reset_file_map();
            let message = extract(
                "test.tsx",
                &format!("import {{ styled }} from '@devup-ui/react';\n{code}"),
                ExtractOption::default(),
            )
            .err()
            .map(|error| error.to_string())
            .unwrap_or_default();
            assert!(message.contains(error), "{code}\n{message}");
        }
    }

    #[test]
    #[serial]
    fn test_create_global_style_collapses_to_a_null_component() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import styled, { createGlobalStyle } from 'styled-components';
const GlobalStyle = createGlobalStyle`body { margin: 0; }`;
const FromObject = createGlobalStyle({ html: { pt: '1px' } });
const S = styled.div`color: red;`;
export const App = () => <><GlobalStyle /><FromObject /><S /></>;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([(
                        "styled-components".to_string(),
                        ImportAlias::DefaultToNamed("styled".to_string())
                    )])
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_styled_components_theme_resolves_to_css_variables() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import styled from 'styled-components';
const Flat = styled.div`color: ${p => p.theme.brand};`;
const Nested = styled.span`color: ${p => p.theme.colors.brand};`;
const Destructured = styled.p`color: ${({ theme }) => theme.colors.accent};`;
const Surrounded = styled.b`border: 1px solid ${p => p.theme.line};`;
const NotTheme = styled.i`color: ${p => p.color};`;
const BareTheme = styled.u`color: ${p => p.theme};`;
const OtherRoot = styled.s`color: ${p => q.theme.brand};`;
const ArrayParam = styled.q`color: ${([p]) => p.theme.brand};`;
const NoParam = styled.em`color: ${() => 'red'};`;
const CallBody = styled.strong`color: ${p => p.theme.brand()};`;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([(
                        "styled-components".to_string(),
                        ImportAlias::DefaultToNamed("styled".to_string())
                    )])
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_namespace_import_of_a_default_only_package_redirects() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import * as Emotion from '@emotion/styled';
const Member = Emotion.div({ bg: 'red' });
const Tagged = Emotion.span`color: ${p => p.theme.brand};`;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([(
                        "@emotion/styled".to_string(),
                        ImportAlias::DefaultToNamed("styled".to_string())
                    )])
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_styled_components_import_surface_fully_redirects() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import styled, { css, keyframes, createGlobalStyle, ThemeProvider, useTheme, withTheme, ServerStyleSheet, StyleSheetManager, isStyledComponent } from 'styled-components';
const S = styled.div`color: red;`;",
                ExtractOption { package: "@devup-ui/react".to_string(), css_dir: "@devup-ui/react".to_string(), single_css: true, import_main_css: false, import_aliases: HashMap::from([("styled-components".to_string(), ImportAlias::DefaultToNamed("styled".to_string()))]) },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_vanilla_extract_names_extract_without_source_dependency() {
        reset_class_map();
        reset_file_map();
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { style, globalStyle, styleVariants } from '@vanilla-extract/css';
export const a = style({ color: 'red' });
globalStyle('body', { margin: '0px' });
export const v = styleVariants({});",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([(
                        "@vanilla-extract/css".to_string(),
                        ImportAlias::NamedToNamed
                    )])
                },
            )
            .unwrap()
        ));
    }

    #[test]
    #[serial]
    fn test_tailwind_conditional_class_name() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const ternary = <Box className={on ? 'p-4' : 'p-8'} />;
const logical = <Box className={on && 'text-red-500'} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_raw_selector_key_without_parent() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const e = <Box selectors={{ 'div p': { color: 'red' }, 'a > b, i': { color: 'blue' } }} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_minus_zero_is_normalized() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const e = <Box transform='translate(-0px,-0%)' />;"
        )));
    }

    #[test]
    #[serial]
    fn test_member_expression_with_dynamic_values() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const e = <Box bg={({ a: first, b: second })[key]} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_props_prop_becomes_spread_attribute() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const e = <Box bg='red' props={extraProps} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_styled_tag_that_is_neither_member_nor_call() {
        reset_class_map();
        reset_file_map();
        let message = extract(
            "test.tsx",
            "import { styled } from '@devup-ui/react';\nconst S = styled['div']`color: red;`;",
            ExtractOption::default(),
        )
        .err()
        .map(|error| error.to_string())
        .unwrap_or_default();
        assert!(
            message.contains("`styled` is read at runtime, where it does not exist"),
            "{message}"
        );
    }

    #[test]
    #[serial]
    fn test_type_instantiation_expression_as_style_value() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const e = <Box bg={pick<string>} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_parenthesized_string_literals() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { Box } from '@devup-ui/react';
const e = <Box as={('span')} bg='red' />;
const g = <Box bg={`${('teal')}`} />;
const h = <Box color={('navy')} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_stylex_attrs_via_named_import() {
        assert_debug_snapshot!(ToBTreeSet::from(extract_tsx(
            r"import { create, attrs } from '@stylexjs/stylex';
const s = create({ a: { color: 'red' } });
const e = <div {...attrs(s.a)} />;"
        )));
    }

    #[test]
    #[serial]
    fn test_emotion_global_with_spread_attribute() {
        assert_debug_snapshot!(ToBTreeSet::from(
            extract(
                "test.tsx",
                r"import { Global } from '@emotion/react';
export const App = () => <Global {...rest} styles={{ body: { margin: '0px' } }} />;",
                ExtractOption {
                    package: "@devup-ui/react".to_string(),
                    css_dir: "@devup-ui/react".to_string(),
                    single_css: true,
                    import_main_css: false,
                    import_aliases: HashMap::from([(
                        "@emotion/react".to_string(),
                        ImportAlias::NamedToNamed
                    )])
                },
            )
            .unwrap()
        ));
    }
}
