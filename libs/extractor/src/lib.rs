mod as_visit;
mod component;
pub mod context;
mod css_utils;
pub mod extract_style;
mod extractor;
mod gen_class_name;
mod gen_style;
mod import_alias_visit;
mod prop_modify_utils;
mod tailwind;
mod util_type;
mod utils;
mod vanilla_extract;
mod visit;
pub use crate::context::ExtractionContext;
pub use crate::extract_style::extract_style_value::ExtractStyleValue;
use crate::visit::DevupVisitor;
use css::file_map::get_file_num_by_filename;
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_ast_visit::VisitMut;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_parser::{Parser, ParserReturn};
use oxc_span::SourceType;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;
use std::path::PathBuf;

/// Import alias configuration for redirecting imports from other CSS-in-JS libraries
#[derive(Debug, Clone, PartialEq)]
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
}

impl ExtractStyleProp<'_> {
    pub fn extract(&self) -> Vec<ExtractStyleValue> {
        match self {
            ExtractStyleProp::Static(style) => vec![style.clone()],
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                let mut styles = vec![];
                if let Some(consequent) = consequent {
                    styles.append(&mut consequent.extract());
                }
                if let Some(alternate) = alternate {
                    styles.append(&mut alternate.extract());
                }
                styles
            }
            ExtractStyleProp::StaticArray(array) => {
                array.iter().flat_map(|s| s.extract()).collect()
            }
            ExtractStyleProp::Expression { styles, .. } => styles.to_vec(),
            ExtractStyleProp::MemberExpression { map, .. } => {
                map.values().flat_map(|s| s.extract()).collect()
            }
            ExtractStyleProp::Enum { map, .. } => map
                .values()
                .flat_map(|s| s.iter().flat_map(|s| s.extract()))
                .collect(),
        }
    }
}
/// Style property for props
#[derive(Debug)]
pub struct ExtractOutput {
    // used styles
    pub styles: HashSet<ExtractStyleValue>,

    // output source
    pub code: String,

    pub map: Option<String>,
    pub css_file: Option<String>,
}

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
    // Step 1: Transform import aliases
    // e.g., `import styled from '@emotion/styled'` → `import { styled } from '@devup-ui/react'`
    // e.g., `import { style } from '@vanilla-extract/css'` → `import { style } from '@devup-ui/react'`
    let transformed_code = import_alias_visit::transform_import_aliases(
        code,
        filename,
        &option.package,
        &option.import_aliases,
    );

    // Step 2: Check if code contains the target package (after transformation)
    let has_relevant_import = transformed_code.contains(option.package.as_str());

    if !has_relevant_import {
        // skip if not using package
        return Ok(ExtractOutput {
            styles: HashSet::new(),
            code: code.to_string(),
            map: None,
            css_file: None,
        });
    }

    // Step 3: Handle vanilla-extract style files (.css.ts, .css.js)
    let is_ve_file = vanilla_extract::is_vanilla_extract_file(filename);
    let (processed_code, is_vanilla_extract) = if is_ve_file {
        // Use transformed code (with imports already pointing to @devup-ui/react)
        match vanilla_extract::execute_vanilla_extract(&transformed_code, &option.package, filename)
        {
            Ok(collected) => {
                // Check if any styles are referenced in selectors
                let referenced = vanilla_extract::find_selector_references(&collected);

                if referenced.is_empty() {
                    // No selector references, use simple code generation
                    let generated =
                        vanilla_extract::collected_styles_to_code(&collected, &option.package);
                    (generated, true)
                } else {
                    // Two-pass extraction: first extract referenced styles to get their class names
                    let partial_code = vanilla_extract::collected_styles_to_code_partial(
                        &collected,
                        &option.package,
                        &referenced,
                    );

                    // Build class map by extracting the partial code
                    let class_map = if !partial_code.is_empty() {
                        extract_class_map_from_code(filename, &partial_code, &option, &referenced)?
                    } else {
                        std::collections::HashMap::new()
                    };

                    // Generate full code with class names substituted into selectors
                    let generated = vanilla_extract::collected_styles_to_code_with_classes(
                        &collected,
                        &option.package,
                        &class_map,
                    );
                    (generated, true)
                }
            }
            Err(_) => {
                // Fall back to treating as regular file if execution fails
                (transformed_code.clone(), false)
            }
        }
    } else {
        (transformed_code.clone(), false)
    };

    // For vanilla-extract files, if no styles were collected, return early
    if is_vanilla_extract && processed_code.is_empty() {
        return Ok(ExtractOutput {
            styles: HashSet::new(),
            code: code.to_string(),
            map: None,
            css_file: None,
        });
    }

    let code_to_parse = if is_vanilla_extract {
        &processed_code
    } else {
        &transformed_code
    };

    let source_type = SourceType::from_path(filename)?;
    let css_file = if option.single_css {
        format!("{}/devup-ui.css", option.css_dir)
    } else {
        format!(
            "{}/devup-ui-{}.css",
            option.css_dir,
            get_file_num_by_filename(filename)
        )
    };
    let mut css_files = vec![css_file.clone()];
    if option.import_main_css && !option.single_css {
        css_files.insert(0, format!("{}/devup-ui.css", option.css_dir));
    }
    let allocator = Allocator::default();

    let ParserReturn {
        mut program, // AST
        panicked,    // Parser encountered an error it couldn't recover from
        ..
    } = Parser::new(&allocator, code_to_parse, source_type).parse();
    if panicked {
        return Err("Parser panicked".into());
    }
    let mut visitor = DevupVisitor::new(
        &allocator,
        filename,
        &option.package,
        css_files,
        if !option.single_css {
            Some(filename.to_string())
        } else {
            None
        },
    );
    visitor.visit_program(&mut program);
    let result = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some(PathBuf::from(filename)),
            ..Default::default()
        })
        .build(&program);

    Ok(ExtractOutput {
        styles: visitor.styles,
        code: result.code,
        map: result.map.map(|m| m.to_json_string()),
        css_file: Some(css_file),
    })
}

/// Extract class names from generated code for specific style names
/// Used for two-pass vanilla-extract processing to resolve selector references
pub fn extract_class_map_from_code(
    filename: &str,
    partial_code: &str,
    option: &ExtractOption,
    style_names: &HashSet<String>,
) -> Result<std::collections::HashMap<String, String>, Box<dyn Error>> {
    let source_type = SourceType::from_path(filename)?;
    let css_file = if option.single_css {
        format!("{}/devup-ui.css", option.css_dir)
    } else {
        format!(
            "{}/devup-ui-{}.css",
            option.css_dir,
            get_file_num_by_filename(filename)
        )
    };
    let css_files = vec![css_file];
    let allocator = Allocator::default();

    let ParserReturn {
        mut program,
        panicked,
        ..
    } = Parser::new(&allocator, partial_code, source_type).parse();
    if panicked {
        Ok(std::collections::HashMap::new())
    } else {
        let mut visitor = DevupVisitor::new(
            &allocator,
            filename,
            &option.package,
            css_files,
            if !option.single_css {
                Some(filename.to_string())
            } else {
                None
            },
        );
        visitor.visit_program(&mut program);

        let result = Codegen::new().build(&program);

        // Parse the output code to extract class name assignments
        // Format: const styleName = "className" or const styleName = "className1 className2"
        let mut class_map = std::collections::HashMap::new();
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
                    // Extract value from "value" or "value";
                    let value = rest
                        .trim_start_matches('"')
                        .trim_end_matches(';')
                        .trim_end_matches('"');

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
        program, panicked, ..
    } = Parser::new(&allocator, code, source_type).parse();

    if panicked {
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
