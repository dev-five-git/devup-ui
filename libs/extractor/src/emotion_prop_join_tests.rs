use crate::emotion_prop_join_support::{Case, run};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::values(Case::Values)]
#[case::width_call(Case::WidthCall)]
#[case::compiled(Case::Compiled)]
#[serial]
fn original_css_prop_values_join_their_selected_properties(
    #[case] case: Case,
    #[values(false, true)] single_css: bool,
) {
    // Given: original complete fixture source and isolated extraction state.
    let _state = crate::test_state::TestStateGuard::new();
    // When: execute the emitted module and its actual App once.
    let record = run(case, single_css);
    // Then: the helper's authored property/value oracle covers every selected token.
    assert_eq!(record.element.target, "div");
    assert_eq!(record.outputs.len(), 1);
    let expected_count = match case {
        Case::Values => 2,
        Case::WidthCall => 5,
        Case::Compiled => 3,
    };
    assert_eq!(record.joins.len(), expected_count);
}
