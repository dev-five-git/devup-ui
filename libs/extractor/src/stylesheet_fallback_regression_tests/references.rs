use rstest::rstest;
use serial_test::serial;

use super::{TestResult, option, reset};
use crate::{ExtractOutput, ResolvedModule, extract_with_modules, extract_without_source_map};

fn assert_failure(
    result: Result<ExtractOutput, Box<dyn std::error::Error>>,
    place: &str,
    cause: &str,
) -> TestResult {
    let error = result
        .err()
        .ok_or("unsafe authored fallback returned successful output")?
        .to_string();
    assert!(error.starts_with(place), "{error}");
    assert!(error.contains("`seed`"), "{error}");
    assert!(error.contains(cause), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    assert!(!error.contains("Invalid exports"), "{error}");
    Ok(())
}

#[rstest]
#[case("const seed=7;", "removed")]
#[case("export const seed=7;", "rewritten")]
#[serial]
fn authored_fallback_fails_at_its_read_when_compilation_changes_the_input_binding(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] seed: &str,
    #[case] cause: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@devup-ui/react';\n{seed}\nexport const help=()=>seed;\nexport const box=style({{color:'blue'}});"
    );
    let path = format!("/unsafe.{suffix}");
    // When
    let result = extract_without_source_map(&path, &source, option());
    // Then
    assert_failure(result, &format!("{path}:3:23:"), cause)
}

#[rstest]
#[case("export const help=()=>({seed});", 25)]
#[case("export const help=()=>({[seed]:1});", 26)]
#[case("const helper=()=>seed; export {helper as invoke};", 18)]
#[case("export const helper=(value=seed)=>value;", 28)]
#[serial]
fn deferred_value_references_fail_when_their_module_binding_is_removed(
    #[values("css.ts", "css.js")] suffix: &str,
    #[case] fallback: &str,
    #[case] column: usize,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@devup-ui/react';\nconst seed=7;\n{fallback}\nexport const box=style({{color:'blue'}});"
    );
    let path = format!("/deferred.{suffix}");
    // When
    let result = extract_without_source_map(&path, &source, option());
    // Then
    assert_failure(result, &format!("{path}:3:{column}:"), "removed")
}

#[rstest]
#[serial]
fn fallback_reference_location_keeps_unicode_columns_and_crlf_lines(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let source = "const 한글='😀';\r\nimport {style as makeStyle} from '@devup-ui/react';\r\nconst seed=7;\r\nexport const help=()=>('😀',seed);\r\nexport const box=makeStyle({color:'blue'});";
    let path = format!("/unicode-fallback.{suffix}");
    // When
    let result = extract_without_source_map(&path, source, option());
    // Then
    assert_failure(result, &format!("{path}:4:28:"), "removed")
}

#[rstest]
#[serial]
fn dependency_fallback_failure_reports_the_producers_original_read(
    #[values("css.ts", "css.js")] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let path = format!("/unsafe-producer.{suffix}");
    let producer_path = path.clone();
    let producer = "import {style} from '@devup-ui/react';\nconst seed=7;\nexport const help=()=>seed;\nexport const box=style({color:'blue'});";
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./producer.css").then(|| ResolvedModule {
            path: producer_path.clone(),
            code: producer.into(),
        })
    };
    let consumer = "import {style} from '@devup-ui/react';\nimport {help} from './producer.css';\nexport const box=style({color:help()});";
    // When
    let result = extract_with_modules(
        &format!("/fallback-consumer.{suffix}"),
        consumer,
        crate::ExtractOption {
            single_css: true,
            ..crate::ExtractOption::default()
        },
        false,
        &resolver,
    );
    // Then
    assert_failure(result, &format!("{path}:3:23:"), "removed")
}
