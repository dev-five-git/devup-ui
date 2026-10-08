use boa_engine::{Context, Source, js_string};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_codegen::{Codegen, Context as CodegenContext, Gen, GenExpr};
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};
use oxc_syntax::precedence::Precedence;

use crate::{ExtractOption, ExtractOutput, ExtractStyleValue};

mod effects;
mod hygiene;
mod oracle;
mod preservation;
mod references;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn reset() {
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
}

fn option() -> ExtractOption {
    ExtractOption {
        single_css: true,
        import_aliases: std::collections::HashMap::from_iter([(
            "@vanilla-extract/css".into(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    }
}

fn stylesheet(package: &str, body: &str) -> String {
    format!("import {{style}} from '{package}';\n{body}\nexport const box=style({{color:'blue'}});")
}

/// Execute actual emitted declarations and export targets, removing only CSS imports.
fn observe(output: &ExtractOutput, predicate: &str) -> TestResult {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let mut script = String::new();
    let mut exports = Vec::new();
    for statement in &parsed.program.body {
        let mut codegen = Codegen::new();
        match statement {
            Statement::ImportDeclaration(import) => {
                assert!(import.specifiers.is_none(), "{}", output.code);
                assert!(
                    std::path::Path::new(import.source.value.as_str()).extension()
                        == Some(std::ffi::OsStr::new("css")),
                    "{}",
                    output.code
                );
                continue;
            }
            Statement::ExportDeclaration(export) => {
                for name in crate::module_loader::declared_names(&export.declaration) {
                    exports.push(format!("{}:{name}", serde_json::to_string(&name)?));
                }
                script.push_str(export.declaration.span().source_text(&output.code));
            }
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    exports.push(format!(
                        "{}:{}",
                        serde_json::to_string(specifier.exported.name().as_str())?,
                        specifier.local.name()
                    ));
                }
                continue;
            }
            Statement::ExportDefaultDeclaration(export) => {
                script.push_str("const __fallback_test_default__ = ");
                export.declaration.to_expression().print_expr(
                    &mut codegen,
                    Precedence::Assign,
                    CodegenContext::default(),
                );
                exports.push("\"default\":__fallback_test_default__".into());
            }
            Statement::ExportFromDeclaration(_) | Statement::ExportAllDeclaration(_) => {
                return Err("fixture output unexpectedly requires a runtime module".into());
            }
            statement => statement.print(&mut codegen, CodegenContext::default()),
        }
        script.push_str(&codegen.into_source_text());
        script.push_str(";\n");
    }
    let script = format!(
        "(()=>{{{script}const exports={{{}}};return {{valid:({predicate}),className:exports.box}};}})()",
        exports.join(",")
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(script.as_bytes()))
        .map_err(|error| format!("emitted export execution failed: {error}\n{script}"))?;
    let result = result.as_object().ok_or("observation was not an object")?;
    let valid = result
        .get(js_string!("valid"), &mut context)
        .map_err(|error| error.to_string())?;
    assert_eq!(valid.as_boolean(), Some(true), "{script}");
    let class = result
        .get(js_string!("className"), &mut context)
        .map_err(|error| error.to_string())?;
    let class = class
        .as_string()
        .ok_or("compiled box export was not a string")?
        .to_std_string_escaped();
    let mut values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| {
            let ExtractStyleValue::Static(style) = value else {
                return None;
            };
            let Some(crate::extract_style::style_property::StyleProperty::ClassName(token)) =
                value.extract(None)
            else {
                return None;
            };
            class
                .split_whitespace()
                .any(|class| class == token)
                .then(|| (style.property().to_string(), style.value().to_string()))
        })
        .collect();
    values.sort();
    assert_eq!(values, vec![("color".into(), "blue".into())]);
    Ok(())
}
