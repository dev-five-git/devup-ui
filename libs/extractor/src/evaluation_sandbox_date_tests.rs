use crate::{ExtractOption, ExtractStyleValue, ResolvedModule, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[path = "evaluation_sandbox_date_negative_tests.rs"]
mod date_negative_tests;

#[path = "evaluation_sandbox_date_semantics_tests.rs"]
mod date_semantics_tests;

fn extract_value(body: &str, imported: bool) -> Result<crate::ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    let helper = format!("export function read() {{\n{body}\n}}");
    let source = if imported {
        "import {style} from '@devup-ui/react';\nimport {read} from './date-helper';\nexport const card = style({opacity: String(read())});".to_string()
    } else {
        format!(
            "import {{style}} from '@devup-ui/react';\nfunction read() {{\n{body}\n}}\nexport const card = style({{opacity: String(read())}});"
        )
    };
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./date-helper").then(|| ResolvedModule {
            path: "date-helper.ts".to_string(),
            code: helper.clone(),
        })
    };
    extract_with_modules(
        "date-entry.css.ts",
        &source,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .map_err(|error| error.to_string())
}

#[rstest]
#[case("new Date(0).getTime()", "0")]
#[case("Date.UTC(2020, 0, 1)", "1577836800000")]
#[case("new Date(Date.UTC(2020, 0, 1)).valueOf()", "1577836800000")]
#[case("new Date(-1.9).getTime()", "-1")]
#[case("new Date(0).getUTCFullYear()", "1970")]
#[case("new Date(0).getUTCMonth()", "0")]
#[case("new Date(0).getUTCDate()", "1")]
#[case("new Date(0).getUTCDay()", "4")]
#[case("new Date(0).getUTCHours()", "0")]
#[case("new Date(0).getUTCMinutes()", "0")]
#[case("new Date(0).getUTCSeconds()", "0")]
#[case("new Date(123).getUTCMilliseconds()", "123")]
#[case("Number.isNaN(new Date(NaN).getTime())", "true")]
#[case("Number.isNaN(new Date(Infinity).getTime())", "true")]
#[case("Number.isNaN(new Date(8640000000000001).getTime())", "true")]
#[case("new Date(8640000000000000).getTime()", "8640000000000000")]
#[case("new Date(-8640000000000000).getTime()", "-8640000000000000")]
#[case("Date.UTC(99, 12, 1)", "946684800000")]
#[case("Date.UTC('2020', '0', '1')", "1577836800000")]
#[case("Number.isNaN(Date.UTC())", "true")]
#[case("Date.UTC(2020)", "1577836800000")]
#[case("+new Date(7)", "7")]
#[case("Reflect.construct(Date, [9]).getTime()", "9")]
#[case("new (new Date(0).constructor)(12).getTime()", "12")]
#[case("new Date(0).toISOString() === '1970-01-01T00:00:00.000Z'", "true")]
#[case(
    "new Date(0).toUTCString() === 'Thu, 01 Jan 1970 00:00:00 GMT'",
    "true"
)]
#[case("new Date(0).toGMTString() === new Date(0).toUTCString()", "true")]
#[case(
    "Date.prototype.toISOString.call(new Date(0)) === '1970-01-01T00:00:00.000Z'",
    "true"
)]
#[serial]
fn deterministic_date_is_emitted_when_full_stylesheet_runs(
    #[case] expression: &str,
    #[case] expected: &str,
    #[values(false, true)] imported: bool,
) -> Result<(), String> {
    // Given / When
    let output = extract_value(&format!("return {expression};"), imported)?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property == "opacity" && style.value == expected
    )), "{:?}", output.styles);
    assert!(!output.code.contains("Date"), "{}", output.code);
    assert!(!output.code.contains("var(--"), "{}", output.code);
    assert!(!output.code.contains("style("), "{}", output.code);
    Ok(())
}

#[rstest]
#[case("new Date(0).getTime()")]
#[case("Date.UTC(2020, 0, 1)")]
fn default_sandbox_stays_strict_when_date_is_deterministic(#[case] expression: &str) {
    // Given / When
    let result = super::reflection_tests::run(expression);
    // Then
    assert!(matches!(result, Err(super::Failure::Forbidden(reads)) if reads[0].name() == "Date"));
}

#[rstest]
#[case("Date.UTC(2020, 0, 1)")]
#[case("new Date(0).getTime()")]
fn actual_value_evaluation_stays_strict_when_date_is_reached_indirectly(
    #[case] expression: &str,
) -> Result<(), String> {
    // Given
    let code = format!(
        "import {{css}} from '@devup-ui/react'; const read = Object.constructor('return {expression}'); export const card = css({{opacity: read()}});"
    );
    // When
    let errors = crate::build_time_values::evaluate_located(
        &code,
        "date-value.tsx",
        &ExtractOption::default(),
        None,
        &crate::imported_constants::Unknown::default(),
    )
    .err()
    .ok_or("Date must fail in actual value evaluation")?;
    // Then
    assert!(
        errors.iter().any(|(_, error)| error.contains("`Date`")),
        "{errors:?}"
    );
    Ok(())
}
