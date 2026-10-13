use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, assert_import, reset, run};
use super::mixed_support::has_static;
use crate::ExtractOutput;

const DATA: &str = "export const palette={space:'8px',browser:window.document};";
const NUMERIC_DATA: &str = "export const palette={0:'8px',browser:window.document};";
const DATA_PATH: &str = "/loader-key-data.ts";

fn source(declaration: &str, member: &str) -> String {
    format!(
        "import {{style}} from '@vanilla-extract/css';import {{palette}} from './data';{declaration}export const box=style({{margin:palette[{member}]}});"
    )
}

fn assert_margin_and_edge(output: &ExtractOutput) {
    assert!(has_static(output, "margin", "8px"), "{:?}", output.styles);
    assert_import(output, "./data");
    assert!(
        output.dependencies.iter().any(|path| path == DATA_PATH),
        "{:?}",
        output.dependencies
    );
}

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn static_margin_when_a_preceding_const_selects_the_required_property(
    #[case] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let source = source("const key='space';", "key");
    // When
    let output = run(
        &format!("/loader-key-constant.{suffix}"),
        &source,
        &[("./data", DATA_PATH, DATA)],
    )?;
    // Then
    assert_margin_and_edge(&output);
    Ok(())
}

#[rstest]
#[case("css.ts")]
#[case("tsx")]
#[serial]
fn static_margin_when_a_literal_selects_the_required_property(#[case] suffix: &str) -> TestResult {
    // Given
    reset();
    let source = source("", "'space'");
    // When
    let output = run(
        &format!("/loader-key-literal.{suffix}"),
        &source,
        &[("./data", DATA_PATH, DATA)],
    )?;
    // Then
    assert_margin_and_edge(&output);
    Ok(())
}

#[rstest]
#[case("css.ts", "const first='space';const key=first;", "key")]
#[case("tsx", "const key=('space' as const);", "(key as string)")]
#[serial]
fn static_margin_when_a_known_key_uses_aliases_or_syntax_wrappers(
    #[case] suffix: &str,
    #[case] declaration: &str,
    #[case] member: &str,
) -> TestResult {
    // Given
    reset();
    let source = source(declaration, member);
    // When
    let output = run(
        &format!("/loader-key-wrapped.{suffix}"),
        &source,
        &[("./data", DATA_PATH, DATA)],
    )?;
    // Then
    assert_margin_and_edge(&output);
    Ok(())
}

#[rstest]
#[case("css.ts", "", "0")]
#[case("css.js", "const key=0;", "key")]
#[serial]
fn static_margin_when_a_numeric_key_selects_the_required_property(
    #[case] suffix: &str,
    #[case] declaration: &str,
    #[case] member: &str,
) -> TestResult {
    // Given
    reset();
    let source = source(declaration, member);
    // When
    let output = run(
        &format!("/loader-key-numeric.{suffix}"),
        &source,
        &[("./data", DATA_PATH, NUMERIC_DATA)],
    )?;
    // Then
    assert_margin_and_edge(&output);
    Ok(())
}

#[rstest]
#[case("css.ts")]
#[case("tsx")]
#[serial]
fn original_entry_error_when_the_required_key_is_runtime_only(#[case] suffix: &str) -> TestResult {
    // Given
    reset();
    let path = format!("/loader-key-runtime.{suffix}");
    let source = source("\nconst key=window.name;\n", "key");
    let expected = crate::locate(
        &path,
        &source,
        source
            .find("window.name")
            .ok_or("authored key read missing")?,
    );
    let producer = "export const palette={space:'8px'};";
    // When
    let result = run(&path, &source, &[("./data", DATA_PATH, producer)]);
    // Then
    let error = result
        .err()
        .ok_or("runtime-only key unexpectedly compiled")?
        .to_string();
    assert!(
        error.contains(&format!(
            "{expected}: JS execution error: ReferenceError: `window` cannot be read at build time:"
        )),
        "{error}"
    );
    assert!(
        error.contains("Fix: use a literal or a CSS variable for `window`"),
        "{error}"
    );
    assert!(!error.contains(&format!("{DATA_PATH}:")), "{error}");
    Ok(())
}
