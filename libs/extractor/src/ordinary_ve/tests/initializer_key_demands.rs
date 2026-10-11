use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, reset, run};
use super::initializer_key_support::{
    ALIAS_CONSUMER, ALIAS_DATA, CONSTANT_DATA, CONSUMER, DATA_PATH, LITERAL_DATA, NUMERIC_CONSUMER,
    NUMERIC_DATA, UNKNOWN_DATA, assert_static_margin_and_edge,
};

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn margin_is_static_when_the_initializer_key_is_a_preceding_const(
    #[case] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let path = format!("/initializer-key-constant.{suffix}");
    // When
    let output = run(&path, CONSUMER, &[("./data", DATA_PATH, CONSTANT_DATA)])?;
    // Then
    assert_static_margin_and_edge(&output);
    Ok(())
}

#[rstest]
#[case("tsx")]
#[case("css.ts")]
#[serial]
fn margin_is_static_when_the_initializer_key_is_a_literal(#[case] suffix: &str) -> TestResult {
    // Given
    reset();
    let path = format!("/initializer-key-literal.{suffix}");
    // When
    let output = run(&path, CONSUMER, &[("./data", DATA_PATH, LITERAL_DATA)])?;
    // Then
    assert_static_margin_and_edge(&output);
    Ok(())
}

#[test]
#[serial]
fn margin_is_static_when_a_nested_initializer_key_uses_a_wrapped_const_alias() -> TestResult {
    // Given
    reset();
    // When
    let output = run(
        "/initializer-key-nested-alias.css.ts",
        ALIAS_CONSUMER,
        &[("./data", DATA_PATH, ALIAS_DATA)],
    )?;
    // Then
    assert_static_margin_and_edge(&output);
    Ok(())
}

#[test]
#[serial]
fn margin_is_static_when_the_initializer_key_is_a_numeric_const() -> TestResult {
    // Given
    reset();
    // When
    let output = run(
        "/initializer-key-numeric.css.js",
        NUMERIC_CONSUMER,
        &[("./data", DATA_PATH, NUMERIC_DATA)],
    )?;
    // Then
    assert_static_margin_and_edge(&output);
    Ok(())
}

#[test]
#[serial]
fn original_producer_error_when_the_initializer_key_is_genuinely_unknown() -> TestResult {
    // Given
    reset();
    let path = "/initializer-key-runtime.tsx";
    let place = crate::locate(
        DATA_PATH,
        UNKNOWN_DATA,
        UNKNOWN_DATA
            .find("window.name")
            .ok_or("missing original computed key")?,
    );
    // When
    let result = run(path, CONSUMER, &[("./data", DATA_PATH, UNKNOWN_DATA)]);
    // Then
    let error = match result {
        Ok(output) => panic!("unknown initializer key silently pruned: {}", output.code),
        Err(error) => error.to_string(),
    };
    assert!(
        error.contains(&format!(
            "{place}: JS execution error: ReferenceError: `window` cannot be read at build time:"
        )),
        "{error}"
    );
    assert!(
        error.contains("Fix: use a literal or a CSS variable for `window`"),
        "{error}"
    );
    assert!(
        error.contains("cannot use `window.name` at build time"),
        "{error}"
    );
    assert!(
        error.contains("this required expression needs an exact, static input"),
        "{error}"
    );
    assert!(
        error.contains("Fix: provide an exact static value or CSS variable for this expression"),
        "{error}"
    );
    assert!(!error.contains(&format!("{path}:")), "{error}");
    Ok(())
}
