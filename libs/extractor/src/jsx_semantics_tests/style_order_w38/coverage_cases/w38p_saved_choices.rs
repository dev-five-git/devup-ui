use super::w38p_saved_oracle::{
    B3, CONSTRUCTION, Declaration, Expected, INVENTORY, K1, R2, W1, verify,
};
use super::w38p_saved_source::{
    AND, COALESCE, CONDITIONAL, EMPTY_AND, LAZY_OR, NONEMPTY_OR, OR, observe,
};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::empty("render(false,false,false,false)", &[K1])]
#[case::right("render(false,true,false,false)", &[K1, B3])]
#[case::left("render(true,false,false,false)", &[K1, R2])]
#[case::both("render(true,true,false,false)", &[K1, R2])]
#[serial]
fn saved_or_when_flags_change_preserves_the_constructed_choice(
    #[case] invocation: &str,
    #[case] selected: &[Declaration],
) {
    // Given: both producers run before the saved OR and the state changes afterward.
    // When: one public extraction and one generated render consume the saved array alias.
    let actual = observe(&OR, invocation);
    // Then: the full inventory, selected declarations and once-only construction agree.
    verify(
        &actual,
        Expected {
            inventory: INVENTORY,
            selected,
            trace: CONSTRUCTION,
        },
    );
}

#[rstest]
#[case::empty("render(false,false,false,false)", &[K1])]
#[case::left_empty("render(false,true,false,false)", &[K1])]
#[case::right_empty("render(true,false,false,false)", &[K1])]
#[case::both("render(true,true,false,false)", &[K1, B3])]
#[serial]
fn saved_and_when_flags_change_selects_right_only_if_left_was_nonempty(
    #[case] invocation: &str,
    #[case] selected: &[Declaration],
) {
    // Given: empty and nonempty left results differ from the eagerly saved right.
    // When: one public extraction and one generated render consume the saved AND.
    let actual = observe(&AND, invocation);
    // Then: false selection stays empty while both producer declarations remain available.
    verify(
        &actual,
        Expected {
            inventory: INVENTORY,
            selected,
            trace: CONSTRUCTION,
        },
    );
}

#[rstest]
#[case::empty("render(false,false,false,false)", &[K1])]
#[case::empty_left("render(false,true,false,false)", &[K1])]
#[case::left("render(true,false,false,false)", &[K1, R2])]
#[case::both("render(true,true,false,false)", &[K1, R2])]
#[serial]
fn saved_coalesce_when_left_is_an_empty_string_keeps_left_not_right(
    #[case] invocation: &str,
    #[case] selected: &[Declaration],
) {
    // Given: css supplies a primitive string even when its saved result is empty.
    // When: one public extraction and one generated render consume the saved coalesce.
    let actual = observe(&COALESCE, invocation);
    // Then: blue remains in the whole inventory but is never selected by coalesce.
    verify(
        &actual,
        Expected {
            inventory: INVENTORY,
            selected,
            trace: CONSTRUCTION,
        },
    );
}

#[rstest]
#[case::empty_right("render(false,false,false,false)", &[K1])]
#[case::empty_left("render(false,false,true,false)", &[K1])]
#[case::right("render(false,true,false,false)", &[K1, B3])]
#[case::choose_empty_left("render(false,true,true,false)", &[K1])]
#[case::choose_empty_right("render(true,false,false,false)", &[K1])]
#[case::left("render(true,false,true,false)", &[K1, R2])]
#[case::both_right("render(true,true,false,false)", &[K1, B3])]
#[case::both_left("render(true,true,true,false)", &[K1, R2])]
#[serial]
fn saved_conditional_when_choice_state_changes_does_not_recompute_the_choice(
    #[case] invocation: &str,
    #[case] selected: &[Declaration],
) {
    // Given: both saved producer strings precede an effectful conditional choice.
    // When: one public extraction and one generated render consume the saved conditional.
    let actual = observe(&CONDITIONAL, invocation);
    // Then: changing state cannot replay either producer or the choice effect.
    verify(
        &actual,
        Expected {
            inventory: INVENTORY,
            selected,
            trace: &["left", "right", "choose"],
        },
    );
}

#[rstest]
#[case::empty_right("render(false,false,false,false)")]
#[case::nonempty_right("render(false,true,false,false)")]
#[serial]
fn saved_or_when_every_left_result_is_nonempty_preserves_left(#[case] invocation: &str) {
    // Given: a real literal red producer and a separately constructed blue-or-empty right.
    // When: one public extraction and one generated render consume the saved OR.
    let actual = observe(&NONEMPTY_OR, invocation);
    // Then: right still constructs once but cannot replace the saved red result.
    verify(
        &actual,
        Expected {
            inventory: INVENTORY,
            selected: &[K1, R2],
            trace: &["right"],
        },
    );
}

#[rstest]
#[case::empty_right("render(false,false,false,false)")]
#[case::nonempty_right("render(false,true,false,false)")]
#[serial]
fn saved_and_when_every_left_result_is_empty_preserves_absence(#[case] invocation: &str) {
    // Given: an ordinary empty css call, not a manually remembered finite result.
    // When: one public extraction and one generated render consume the saved AND.
    let actual = observe(&EMPTY_AND, invocation);
    // Then: blue constructs once and remains inventoried without becoming selected.
    verify(
        &actual,
        Expected {
            inventory: &[B3, K1],
            selected: &[K1],
            trace: &["right"],
        },
    );
}

#[rstest]
#[case::empty_white("render(false,false,false,false)", &[W1], &["left", "right", "fallback"])]
#[case::empty_black("render(false,false,false,true)", &[K1], &["left", "right", "fallback"])]
#[case::blue_false("render(false,true,false,false)", &[B3], CONSTRUCTION)]
#[case::blue_true("render(false,true,false,true)", &[B3], CONSTRUCTION)]
#[case::red_false("render(true,false,false,false)", &[R2], CONSTRUCTION)]
#[case::red_true("render(true,false,false,true)", &[R2], CONSTRUCTION)]
#[case::both_false("render(true,true,false,false)", &[R2], CONSTRUCTION)]
#[case::both_true("render(true,true,false,true)", &[R2], CONSTRUCTION)]
#[serial]
fn saved_consumer_when_selected_result_is_empty_evaluates_only_its_lazy_fallback(
    #[case] invocation: &str,
    #[case] selected: &[Declaration],
    #[case] trace: &[&str],
) {
    // Given: a saved OR choice and distinct black/white fallback declarations.
    // When: one public extraction and one generated render consume the lazy fallback.
    let actual = observe(&LAZY_OR, invocation);
    // Then: the fallback runs once only for empty results, never during eager construction.
    verify(
        &actual,
        Expected {
            inventory: &[R2, B3, K1, W1],
            selected,
            trace,
        },
    );
}
