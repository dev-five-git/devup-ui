use boa_engine::{Context, Source};
use rstest::rstest;

use super::Operations;

#[rstest]
#[case("TypeError: iterator failed\n    at eval (devup-ui-stylesheet:unknown)")]
#[case("TypeError: iterator failed\n    at eval (devup-ui-stylesheet:x:y)")]
fn unlocated_engine_frames_survive_operation_rebasing(#[case] error: &str) {
    // Given
    let operations = Operations::new("[...[]]");
    // When / Then
    assert_eq!(operations.explain(error, &Context::default()), error);
}

#[test]
fn failed_spread_uses_operation_site_when_boa_has_no_script_frame() -> Result<(), String> {
    // Given
    let source = "[...null]";
    let operations = Operations::new(source);
    let mut context = Context::default();
    operations
        .prepare(&mut context)
        .map_err(|error| error.to_string())?;
    // When
    let error = context
        .eval(Source::from_bytes(&operations.code))
        .err()
        .ok_or("invalid spread succeeded")?;
    let explained = operations.explain(&error.to_string(), &context);
    // Then
    assert!(
        explained.contains("at <operation> (devup-ui-stylesheet:1:2)"),
        "{explained}"
    );
    Ok(())
}
