use serial_test::serial;

use super::consumer_support::{edges, located_failure};
use super::demand_support::{TestResult, reset, run};
use super::mixed_support::has_static;

const PRODUCER: &str = "import {createTheme} from '@vanilla-extract/css';import {tokens} from './tokens';export const [theme,vars]=createTheme({space:tokens.space});";
const CONSUMER: &str = "import {css} from '@devup-ui/react';import {vars} from './producer';export const box=css({margin:vars.space});";

#[test]
#[serial]
fn required_commonjs_inner_path_error_is_located_in_original_helper() {
    // Given
    reset();
    let helper = "// 한글\r\n\r\n  module.exports[key].space='8px';";
    // When
    let result = run(
        "/path-cjs-consumer.ts",
        CONSUMER,
        &[
            ("./producer", "/path-cjs-producer.ts", PRODUCER),
            ("./tokens", "/path-tokens.js", helper),
        ],
    );
    // Then
    located_failure(
        result,
        "/path-tokens.js:3:3",
        "required CommonJS export needs an exact static path",
    );
}

#[test]
#[serial]
fn exact_computed_commonjs_path_publishes_native_css_and_dependencies() -> TestResult {
    // Given
    reset();
    let helper = "module['exports']['tokens']={space:'8px',browser:window.document};";
    // When
    let graph = crate::graph::extract_graph(
        "/exact-path-cjs-consumer.ts",
        CONSUMER,
        super::option(),
        false,
        Some(&|specifier, _| match specifier {
            "./producer" => Some(crate::ResolvedModule {
                path: "/exact-path-cjs-producer.ts".to_string(),
                code: PRODUCER.to_string(),
            }),
            "./tokens" => Some(crate::ResolvedModule {
                path: "/exact-path-tokens.js".to_string(),
                code: helper.to_string(),
            }),
            _ => None,
        }),
    )?;
    let output = graph.entry;
    // Then
    assert!(has_static(&output, "margin", "var(--space-0-1)"));
    assert!(!output.styles.iter().any(|style| matches!(style,
        crate::ExtractStyleValue::Static(style) if style.property == "--space-0-1")));
    let owners: Vec<_> = graph
        .artifacts
        .iter()
        .filter(|artifact| artifact.filename == "/exact-path-cjs-producer.ts")
        .collect();
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].output.styles.iter().filter(|style| matches!(style,
        crate::ExtractStyleValue::Static(style)
            if style.property == "--space-0-1" && style.value == "8px"
                && matches!(&style.selector, Some(css::style_selector::StyleSelector::Global(_, owner))
                    if owner == "/exact-path-cjs-producer.ts"))).count(), 1);
    assert!(!output.code.contains("vars.space"), "{}", output.code);
    edges(
        &output,
        &["/exact-path-cjs-producer.ts", "/exact-path-tokens.js"],
    );
    assert_eq!(output.dependencies.len(), 2);
    Ok(())
}
