use super::w38q_leaf_oracle::{Expected, verify};
use super::w38q_leaf_source::{Fixture, observe};
use super::*;
use rstest::rstest;

const EMPTY: Expected<'static> = Expected {
    saved: false,
    classes: "color-0-red--2-a",
    external: "",
    trace: &[],
};
const LITERAL: Expected<'static> = Expected {
    classes: "ext-a color-0-red--2-a",
    external: "ext-a",
    ..EMPTY
};
const SAVED_EMPTY: Expected<'static> = Expected {
    saved: true,
    classes: " color-0-red--2-a background-0-black--1-a",
    trace: &["construct"],
    ..EMPTY
};
const SAVED_NONEMPTY: Expected<'static> = Expected {
    classes: "ext-a color-0-red--2-a background-0-black--1-a",
    external: "ext-a",
    ..SAVED_EMPTY
};
const DYNAMIC_EMPTY: Expected<'static> = Expected {
    classes: "color-0-red--2-a ",
    trace: &["template"],
    ..EMPTY
};
const DYNAMIC_NONEMPTY: Expected<'static> = Expected {
    classes: "color-0-red--2-a ext-a",
    external: "ext-a",
    ..DYNAMIC_EMPTY
};

#[rstest]
#[case::literal_empty("''", EMPTY)]
#[case::literal_nonempty("'ext-a'", LITERAL)]
#[case::constant_template_empty("``", SAVED_EMPTY)]
#[case::constant_template_nonempty("`ext-a`", SAVED_NONEMPTY)]
#[case::interpolated_template_empty("`${mark('template',external)}`", DYNAMIC_EMPTY)]
#[case::interpolated_template_nonempty("`${mark('template',external)}`", DYNAMIC_NONEMPTY)]
#[case::static_getter("object.value", Expected { trace: &["getter"], ..DYNAMIC_NONEMPTY })]
#[case::computed_getter("object[mark('key','value')]", Expected { trace: &["key", "getter"], ..DYNAMIC_NONEMPTY })]
#[serial]
fn w38q_leaf_when_public_external_is_composed_preserves_selection_and_reads(
    #[case] leaf: &str,
    #[case] expected: Expected<'_>,
) {
    // Given: independent authored leaf, class order, declarations and effect trace.
    let input = Fixture {
        leaf,
        saved: expected.saved,
    };
    // When: compile the real imported ClassNames child and render once.
    let actual = observe(&input, expected.external);
    // Then: compare full typed inventory before exact selection and once-only effects.
    verify(&actual, expected);
}
