use super::EmitError;
use rstest::rstest;

#[rstest]
#[case(
    EmitError::Unavailable,
    "data was not available at its original capture site"
)]
#[case(
    EmitError::ImmutableOrder,
    "nonconfigurable property cannot be reordered"
)]
fn coverage_emission_error_display_preserves_the_diagnostic_cause(
    #[case] cause: EmitError,
    #[case] expected: &str,
) {
    let formatted = cause.to_string();
    assert_eq!(formatted, expected);
    assert_eq!(
        std::error::Error::source(&cause).map(ToString::to_string),
        None
    );
}
