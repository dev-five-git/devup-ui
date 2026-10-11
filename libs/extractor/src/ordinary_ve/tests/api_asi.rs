use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, reset, run};
use super::mixed_review::emitted_predicate;
use super::mixed_support::has_static;

#[rstest]
#[case("const size=2\nstyle({color:'red'})\nreturn size")]
#[case("const size=2\nif(false)style({color:'blue'})\nstyle({color:'red'})\nreturn size")]
#[serial]
fn native_callee_normalization_preserves_asi_and_controlled_statements(
    #[case] body: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';export const result=(()=>{{{body}}})();const browser=window.document;"
    );
    // When
    let output = run("/native-asi.ts", &source, &[])?;
    // Then
    assert!(has_static(&output, "color", "red"));
    assert!(!has_static(&output, "color", "blue"));
    assert!(
        emitted_predicate(&output.code, "result===2"),
        "{}",
        output.code
    );
    Ok(())
}
