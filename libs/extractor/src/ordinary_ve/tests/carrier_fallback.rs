use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, assert_import, global_selector, reset, run};
use super::mixed_support::{assert_consumed, has_static};

#[rstest]
#[case(("import {css,globalCss} from '@devup-ui/react';", "css", "globalCss"))]
#[case(("import * as du from '@devup-ui/react';", "du.css", "du.globalCss"))]
#[serial]
fn devup_stylesheet_fallback_keeps_classes_and_global_rules_when_native_imports_prove_nothing(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(
        "",
        "import {style as unused} from '@vanilla-extract/css';",
        "import * as unused from '@vanilla-extract/css';"
    )]
    unused_native: &str,
    #[case] api: (&str, &str, &str),
) -> TestResult {
    // Given: normal standalone Devup extraction supplies the class/rule oracle.
    reset();
    let (import, css, global) = api;
    let path = format!("/devup-fallback.{suffix}");
    let control_source = format!(
        "{import}export const base={css}({{color:'orange',p:2}});{global}({{body:{{borderWidth:'12px'}}}});"
    );
    let control = run(&path, &control_source, &[])?;
    assert!(has_static(&control, "padding", "8px"));
    assert!(has_static(&control, "color", "orange"));
    assert!(has_static(&control, "border-width", "12px"));
    assert_eq!(global_selector(&control, "border-width"), "body");
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &control.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", control.code);
    let classes = parsed
        .program
        .body
        .iter()
        .find_map(|statement| {
            let Statement::ExportDeclaration(export) = statement else {
                return None;
            };
            let Declaration::VariableDeclaration(declaration) = &export.declaration else {
                return None;
            };
            declaration.declarations.iter().find_map(|declarator| {
                let identifier = declarator.id.get_binding_identifier()?;
                let Expression::StringLiteral(value) = declarator.init.as_ref()? else {
                    return None;
                };
                (identifier.name == "base").then(|| value.value.to_string())
            })
        })
        .ok_or("control class export missing")?;
    let producer = format!("{unused_native}{control_source}");
    let graph = [("./data", path.as_str(), producer.as_str())];
    let source = "import {style} from '@vanilla-extract/css';import {base} from './data';export const box=style([base,{margin:3,content:JSON.stringify(base)}]);const browser=window.document;";
    // When
    let output = run("/fallback-entry.tsx", source, &graph)?;
    // Then: the producer class is retained, while its declarations remain producer-owned.
    assert_consumed(&output);
    assert!(has_static(
        &output,
        "content",
        &serde_json::to_string(&classes)?
    ));
    assert!(has_static(&output, "margin", "3px"));
    assert!(!output.code.contains("style("));
    assert!(output.code.contains("window.document"));
    assert_import(&output, "./data");
    assert!(
        output
            .dependencies
            .iter()
            .any(|dependency| dependency == &path)
    );
    Ok(())
}
