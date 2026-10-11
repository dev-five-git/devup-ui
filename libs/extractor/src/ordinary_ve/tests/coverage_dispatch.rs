use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, reset, run};
use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, has_static};

#[rstest]
#[case("export const view=<div/>;")]
#[case("export const view=<></>;")]
#[case("throw new Error('runtime only');")]
#[serial]
fn emits_css_without_running_unrelated_jsx_or_throw(#[case] retained: &str) -> TestResult {
    reset();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';export const box=style({{padding:8}});{retained}"
    );
    let output = run("/dispatch.tsx", &source, &[])?;
    assert_consumed(&output);
    assert!(has_static(&output, "padding", "8px"));
    assert_preserved(&output.code, retained);
    Ok(())
}

#[rstest]
#[case("export type Marker={ value:string };")]
#[case("export interface Marker { value:string }")]
#[case("const rules={ padding:8 };rules.padding=12;export const box=style(rules);")]
#[case(
    "const rules={ padding:8 };Object.assign(rules,{ padding:12 });export const box=style(rules);"
)]
fn keeps_types_and_selected_data_mutations_on_the_nonmixed_path(#[case] body: &str) {
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';{body}export const other=style({{color:'blue'}});"
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = super::super::selection::select(&parsed.program, &built.semantic);
    let mixed = super::super::dispatch::mixed(&parsed.program, &built.semantic, &selection);
    assert!(!mixed, "{source}");
    assert_eq!(selection.escapes.len(), 0);
    assert_eq!(
        selection.roots.len(),
        if body.contains("rules") { 2 } else { 1 }
    );
}
