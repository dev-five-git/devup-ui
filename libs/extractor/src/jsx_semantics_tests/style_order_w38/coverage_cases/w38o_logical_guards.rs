use super::w38o_logical_oracle::{Expected, verify};
use super::w38o_logical_source::{NESTED_GUARDS, observe};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::both_false((false, false), "black", &["outer"])]
#[case::outer_false((false, true), "black", &["outer"])]
#[case::inner_false((true, false), "green", &["outer", "inner"])]
#[case::both_true((true, true), "red", &["outer", "inner"])]
#[serial]
fn nested_guard_when_boolean_inputs_select_a_branch_preserves_lazy_effects(
    #[case] flags: (bool, bool),
    #[case] color: &str,
    #[case] trace: &[&str],
) {
    // Given: a black base and two distinct ordered alternatives under nested guards.
    let (outer, inner) = flags;
    let invocation = format!("render(false,{outer},{inner})");
    // When: public extraction and exactly one generated render execute.
    let actual = observe(&NESTED_GUARDS, &invocation);
    // Then: only the native winner applies and the inner effect remains lazy.
    verify(
        &actual,
        Expected {
            inventory: &["black", "red", "green"],
            selected: &[color],
            trace,
            order: Some(2),
        },
    );
}
