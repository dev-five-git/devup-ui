use std::path::Path;

use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Statement};
use oxc_codegen::{Codegen, Gen, GenExpr};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use rstest::rstest;
use serial_test::serial;

use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, extract_without_source_map};

mod components;
mod extended;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    &'static str,
    &'static str,
    Option<(&'static str, &'static str)>,
    &'static str,
);

const LOWER_COMPILED: &str = "import {css,style} from '@devup-ui/react';
const base=css({p:2});
const loop={};
(function(){loop.self=loop;})();
export {loop};
export const box=style([base,{color:'sienna'}]);";

const LOWER_EARLIER_VALUES: &str = "import {css,style} from '@devup-ui/react';
function make(n:number){
  return n+1;
}
const loop={};
loop.self=loop;
export {loop};
const base=css({w:(make(2))});
export const box=style([base,{color:'sienna'}]);";

const COMPUTED_LABELS: &str = "import {css,style} from '@devup-ui/react';
export const labels=['compact','wide'].map(label=>label.toUpperCase());
const base=css({p:2});
export const box=style([base,{color:'sienna'}]);";

const PADDING_RULES: &str = "{p:2}";
const WIDTH_RULES: &str = "{w:3}";
const EMPTY_RULES: &str = "{}";
const EMPTY_SOURCE: &str = "import {css,style} from '@devup-ui/react';const base=css({});const loop={};loop.self=loop;export {loop};export const box=style([base,{color:'sienna'}]);";
const ALIAS_SOURCE: &str = "import {css as rules,style as nativeStyle} from '@devup-ui/react';const base=rules({p:2});const loop={};loop.self=loop;export {loop};export const box=nativeStyle([base,{color:'sienna'}]);";
const NAMESPACE_SOURCE: &str = "import * as ui from '@devup-ui/react';const base=ui.css({p:2});const loop={};loop.self=loop;export {loop};export const box=ui.style([base,{color:'sienna'}]);";

fn reset() {
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
}

fn observe(output: &ExtractOutput, predicate: &str) -> Result<String, Box<dyn std::error::Error>> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let mut program = parsed.program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let options = TransformOptions::default();
    let transformed = Transformer::new(&allocator, Path::new("sheet.css.ts"), &options)
        .build_with_scoping(scoping, &mut program);
    assert_eq!(transformed.diagnostics.len(), 0, "{}", output.code);
    let mut script = String::new();
    let mut exports = Vec::new();
    for statement in &program.body {
        let mut codegen = Codegen::new();
        match statement {
            Statement::ImportDeclaration(import) => {
                assert!(
                    import.specifiers.is_none(),
                    "runtime import: {}",
                    output.code
                );
                assert_eq!(
                    Path::new(import.source.value.as_str()).extension(),
                    Some(std::ffi::OsStr::new("css")),
                    "{}",
                    output.code
                );
                continue;
            }
            Statement::ExportDeclaration(export) => {
                for name in crate::module_loader::declared_names(&export.declaration) {
                    exports.push(format!("{}:{name}", serde_json::to_string(&name)?));
                }
                match &export.declaration {
                    Declaration::VariableDeclaration(declaration) => {
                        declaration.print(&mut codegen, oxc_codegen::Context::default());
                    }
                    _ => panic!("unexpected exported declaration: {}", output.code),
                }
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
                let expression = export
                    .declaration
                    .as_expression()
                    .ok_or("expression default export")?;
                script.push_str("const __macro_default__ = ");
                expression.print_expr(
                    &mut codegen,
                    oxc_syntax::precedence::Precedence::Assign,
                    oxc_codegen::Context::default(),
                );
                exports.push("default:__macro_default__".to_string());
                script.push_str(&codegen.into_source_text());
                script.push_str(";\n");
                continue;
            }
            Statement::ExportFromDeclaration(_) | Statement::ExportAllDeclaration(_) => {
                panic!("unexpected export form: {}", output.code);
            }
            statement => statement.print(&mut codegen, oxc_codegen::Context::default()),
        }
        script.push_str(&codegen.into_source_text());
        script.push('\n');
    }
    let script = format!(
        "(()=>{{{script}const exports={{{}}};return JSON.stringify([({predicate}),exports.box]);}})()",
        exports.join(",")
    );
    let value = Context::default()
        .eval(Source::from_bytes(script.as_bytes()))
        .map_err(|error| format!("emitted program failed: {error}\n{script}"))?;
    let json = value
        .as_string()
        .ok_or("observation was not JSON text")?
        .to_std_string_escaped();
    let (valid, class): (bool, String) = serde_json::from_str(&json)?;
    assert!(valid, "{script}");
    Ok(class)
}

fn declarations(output: &ExtractOutput, class: &str) -> Vec<(String, String)> {
    let mut declarations: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| match value {
            ExtractStyleValue::Static(style) => {
                value.extract(None).and_then(|property| match property {
                    crate::extract_style::style_property::StyleProperty::ClassName(token)
                        if class.split_whitespace().any(|class| class == token) =>
                    {
                        Some((style.property().to_string(), style.value().to_string()))
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .collect();
    declarations.sort();
    declarations
}

#[rstest]
#[case::lower_compiled((LOWER_COMPILED, PADDING_RULES, Some(("padding", "8px")), "exports.loop.self===exports.loop"))]
#[case::lower_earlier_values((LOWER_EARLIER_VALUES, WIDTH_RULES, Some(("width", "12px")), "exports.loop.self===exports.loop"))]
#[case::computed_labels((COMPUTED_LABELS, PADDING_RULES, Some(("padding", "8px")), "exports.labels.length===2 && exports.labels[0]==='COMPACT' && exports.labels[1]==='WIDE'"))]
#[case::empty((EMPTY_SOURCE, EMPTY_RULES, None, "exports.loop.self===exports.loop"))]
#[case::alias((ALIAS_SOURCE, PADDING_RULES, Some(("padding", "8px")), "exports.loop.self===exports.loop"))]
#[case::namespace((NAMESPACE_SOURCE, PADDING_RULES, Some(("padding", "8px")), "exports.loop.self===exports.loop"))]
#[serial]
fn own_css_keeps_class_identity_and_exports_when_stylesheet_fallback_runs(
    #[case] fixture: Fixture,
) -> TestResult {
    // Given
    let (source, rules, expected_base, predicate) = fixture;
    let filename = "/own-css-macro.css.ts";
    let option = ExtractOption {
        single_css: true,
        ..ExtractOption::default()
    };
    let reference_source =
        format!("import {{css}} from '@devup-ui/react';export const box=css({rules});");
    reset();
    let reference = extract_without_source_map(filename, &reference_source, option.clone())?;
    let base_class = observe(&reference, "true")?;
    let base_declarations: Vec<_> = expected_base
        .into_iter()
        .map(|(property, value)| (property.to_string(), value.to_string()))
        .collect();
    assert_eq!(declarations(&reference, &base_class), base_declarations);
    assert_eq!(
        base_class.split_whitespace().count(),
        usize::from(expected_base.is_some())
    );
    reset();

    // When
    let output = extract_without_source_map(filename, source, option)?;

    // Then
    let box_class = observe(&output, predicate)?;
    for token in base_class.split_whitespace() {
        assert!(
            box_class.split_whitespace().any(|class| class == token),
            "{box_class}"
        );
    }
    let mut expected = vec![("color".to_string(), "sienna".to_string())];
    expected.extend(base_declarations);
    expected.sort();
    assert_eq!(declarations(&output, &box_class), expected);
    Ok(())
}
