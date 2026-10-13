use super::w38o_logical_oracle::{Expected, verify};
use super::w38o_logical_source::{EMPTY_COALESCE, EMPTY_OR, NONEMPTY_OR, observe};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::present(true, "red")]
#[case::absent(false, "blue")]
#[serial]
fn finite_or_when_saved_result_may_be_empty_selects_the_native_winner(
    #[case] active: bool,
    #[case] color: &str,
) {
    // Given: a saved closed CSS result and a distinct blue fallback.
    let invocation = format!("render({active},false,false)");
    // When: public extraction and exactly one generated render execute.
    let actual = observe(&EMPTY_OR, &invocation);
    // Then: the chosen declaration, complete stylesheet and construction trace agree.
    verify(
        &actual,
        Expected {
            inventory: &["red", "blue"],
            selected: &[color],
            trace: &["construct"],
            order: Some(2),
        },
    );
}

#[rstest]
#[case::present(true, &["red"])]
#[case::empty(false, &[])]
#[serial]
fn finite_coalesce_when_saved_result_is_a_string_never_selects_the_fallback(
    #[case] active: bool,
    #[case] selected: &[&str],
) {
    // Given: even the inactive CSS result is the non-nullish empty string.
    let invocation = format!("render({active},false,false)");
    // When: public extraction and exactly one generated render execute.
    let actual = observe(&EMPTY_COALESCE, &invocation);
    // Then: blue is absent from the complete inventory as well as selection.
    verify(
        &actual,
        Expected {
            inventory: &["red"],
            selected,
            trace: &["construct"],
            order: Some(2),
        },
    );
}

#[rstest]
#[case::red(true, "red")]
#[case::green(false, "green")]
#[serial]
fn finite_or_when_every_saved_result_is_nonempty_preserves_the_saved_choice(
    #[case] active: bool,
    #[case] color: &str,
) {
    // Given: both constructed alternatives differ from the unreachable blue fallback.
    let invocation = format!("render({active},false,false)");
    // When: public extraction and exactly one generated render execute.
    let actual = observe(&NONEMPTY_OR, &invocation);
    // Then: no blue rule exists and the saved choice is not recomputed.
    verify(
        &actual,
        Expected {
            inventory: &["red", "green"],
            selected: &[color],
            trace: &["construct"],
            order: Some(2),
        },
    );
}
