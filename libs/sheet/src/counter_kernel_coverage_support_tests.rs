use super::{KernelError, authentic_support::*};
use crate::counter_evidence::ReplayError;

struct Keys(Vec<String>);
impl Drop for Keys {
    fn drop(&mut self) {
        css::theme_tokens::set_typography_keys(std::mem::take(&mut self.0));
    }
}

#[rstest::rstest]
#[case(ReplayError::Allocation, KernelError::Authority)]
#[case(ReplayError::Expansion, KernelError::Coverage)]
#[case(ReplayError::Cleanup, KernelError::Cleanup)]
#[case(ReplayError::MissingPreset, KernelError::Preset)]
#[case(ReplayError::Preset, KernelError::Preset)]
fn replay_failure_keeps_its_boundary_category_when_converted(
    #[case] replay: ReplayError,
    #[case] expected: KernelError,
) {
    // Given: an independent replay rejection.
    // When
    let error = KernelError::from(replay);
    // Then
    assert_eq!(error, expected);
}

#[test]
fn missing_fixture_panics_with_context_when_option_is_none() {
    // Given
    let fixture: Option<u8> = None;
    // When
    let failure = std::panic::catch_unwind(|| fixture.required("absent fixture"));
    // Then
    let payload = failure.err().required("missing fixture must panic");
    assert_eq!(
        payload.downcast_ref::<String>().required("panic text"),
        "missing test fixture: absent fixture"
    );
}

#[test]
fn failed_fixture_panics_with_context_and_error_when_result_is_err() {
    // Given
    let fixture: Result<u8, KernelError> = Err(KernelError::Coverage);
    // When
    let failure = std::panic::catch_unwind(|| fixture.required("rejected fixture"));
    // Then
    let payload = failure.err().required("failed fixture must panic");
    assert_eq!(
        payload.downcast_ref::<String>().required("panic text"),
        "rejected fixture: Coverage"
    );
}

#[test]
#[serial_test::serial]
fn saved_presets_restore_populated_sparse_registry_when_guard_drops() {
    // Given
    let _state = state();
    let _keys = Keys(css::theme_tokens::get_typography_keys());
    let _original = Presets::save();
    let original = BTreeMap::from([(
        "coverageHeading".into(),
        vec![
            (0, "font-size".into(), "16px".into()),
            (2, "font-weight".into(), "700".into()),
        ],
    )]);
    css::content_typography::set(original.clone());
    css::theme_tokens::set_typography_keys(vec!["coverageHeading".into()]);
    let saved = Presets::save();
    css::content_typography::set(BTreeMap::from([(
        "other".into(),
        vec![(0, "font-size".into(), "24px".into())],
    )]));
    // When
    drop(saved);
    // Then
    assert_eq!(
        css::theme_tokens::get_typography_keys(),
        vec!["coverageHeading".to_string()]
    );
    assert_eq!(
        css::content_typography::declarations("coverageHeading", 0),
        original["coverageHeading"]
    );
}
