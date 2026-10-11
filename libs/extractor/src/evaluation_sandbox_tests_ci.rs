use rstest::rstest;

use super::{reads, text};

#[test]
fn text_rejects_javascript_failure_instead_of_reporting_a_value() {
    // Given / When
    let result = text("throw new TypeError('fixture failure')");
    // Then
    assert!(result.is_err());
}

#[test]
fn text_rejects_forbidden_read_even_when_script_catches_it() {
    // Given / When
    let result = text("try { Date.now() } catch {} 'caught'");
    // Then
    assert!(result.is_err());
}

#[rstest]
#[case("throw new TypeError('fixture failure')")]
#[case("Math.max(1, 2)")]
fn reads_rejects_runs_without_forbidden_evidence(#[case] script: &str) {
    // Given / When
    let result = reads(script);
    // Then
    assert!(result.is_err());
}
