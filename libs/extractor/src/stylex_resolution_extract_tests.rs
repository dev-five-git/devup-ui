//! `stylex.create()` read under each `styleResolution`, and `stylex.props()`
//! merging the keys it gives
use crate::{ExtractOption, extract, test_state::TestStateGuard};
use css::style_resolution::{StyleResolution, set_style_resolution};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

/// The code `create` and `use_` give under `policy`, with debug class names
/// short of their file suffix
fn code_under(policy: StyleResolution, create: &str, use_: &str) -> String {
    let _state = TestStateGuard::new();
    reset_class_map();
    reset_file_map();
    css::debug::set_debug(true);
    set_style_resolution(policy);
    let code = format!(
        "import * as stylex from '@stylexjs/stylex';
const s = stylex.create({{ {create} }});
export const result = {use_};"
    );
    extract("test.tsx", &code, ExtractOption::default())
        .map_or_else(|error| error.to_string(), |output| output.code)
        .replace("-255-a", "-255")
}

/// The classes of the first string after `marker` in `code`, sorted
fn classes(code: &str, marker: &str) -> Vec<String> {
    let (_, rest) = code.split_once(marker).unwrap_or_default();
    let (list, _) = rest.split_once('"').unwrap_or_default();
    let mut classes: Vec<String> = list.split_whitespace().map(str::to_string).collect();
    classes.sort_unstable();
    classes
}

fn sorted(expected: &[&str]) -> Vec<String> {
    let mut expected: Vec<String> = expected.iter().map(|class| (*class).to_string()).collect();
    expected.sort_unstable();
    expected
}

const SIDES: &str = "top: { paddingTop: '1px' }, pad: { padding: '2px' }";

#[rstest]
// The default leaves both: the sheet writes the longhand after its shorthand
#[case(
    StyleResolution::PropertySpecificity,
    "stylex.props(s.top, s.pad)",
    &["padding-top-0-1px--255", "padding-0-2px--255"]
)]
// A later shorthand resets the longhands it covers
#[case(
    StyleResolution::ApplicationOrder,
    "stylex.props(s.top, s.pad)",
    &["padding-0-2px--255"]
)]
// A later longhand applies after its shorthand
#[case(
    StyleResolution::ApplicationOrder,
    "stylex.props(s.pad, s.top)",
    &["padding-0-2px--255", "padding-top-0-1px--255"]
)]
// The shorthand is gone: its sides merge like any longhand
#[case(
    StyleResolution::LegacyExpandShorthands,
    "stylex.props(s.pad, s.top)",
    &[
        "padding-top-0-1px--255",
        "padding-inline-end-0-2px--255",
        "padding-bottom-0-2px--255",
        "padding-inline-start-0-2px--255",
    ]
)]
#[case(
    StyleResolution::LegacyExpandShorthands,
    "stylex.props(s.top, s.pad)",
    &[
        "padding-top-0-2px--255",
        "padding-inline-end-0-2px--255",
        "padding-bottom-0-2px--255",
        "padding-inline-start-0-2px--255",
    ]
)]
#[serial]
fn props_merge_the_keys_a_policy_gives(
    #[case] policy: StyleResolution,
    #[case] use_: &str,
    #[case] expected: &[&str],
) {
    let code = code_under(policy, SIDES, use_);
    assert_eq!(classes(&code, "className: \""), sorted(expected), "{code}");
}

#[rstest]
// Within a namespace, a later key replaces an earlier one where it stands
#[case(
    StyleResolution::ApplicationOrder,
    "a: { paddingTop: '1px', padding: '2px' }",
    &["padding-0-2px--255"]
)]
#[case(
    StyleResolution::ApplicationOrder,
    "a: { padding: '2px', paddingTop: '1px' }",
    &["padding-0-2px--255", "padding-top-0-1px--255"]
)]
#[case(
    StyleResolution::PropertySpecificity,
    "a: { paddingTop: '1px', padding: '2px' }",
    &["padding-top-0-1px--255", "padding-0-2px--255"]
)]
#[case(
    StyleResolution::LegacyExpandShorthands,
    "a: { margin: '1px 2px', marginTop: '3px' }",
    &[
        "margin-top-0-3px--255",
        "margin-inline-end-0-2px--255",
        "margin-bottom-0-1px--255",
        "margin-inline-start-0-2px--255",
    ]
)]
// An alias names the property it stands for
#[case(
    StyleResolution::ApplicationOrder,
    "a: { marginStart: 4 }",
    &["margin-inline-start-0-4px--255"]
)]
#[case(
    StyleResolution::LegacyExpandShorthands,
    "a: { paddingBlock: '1px 2px' }",
    &["padding-top-0-1px--255", "padding-bottom-0-2px--255"]
)]
// Each condition of a value expands on its own
#[case(
    StyleResolution::LegacyExpandShorthands,
    "a: { gap: { default: '1px 2px', ':hover': null, '@media (min-width: 1px)': '3px' } }",
    &[
        "row-gap-0-1px--255",
        "row-gap-0-3px-_at_media_lp_min-width_c_1px_rp_-255",
        "column-gap-0-2px--255",
        "column-gap-0-3px-_at_media_lp_min-width_c_1px_rp_-255",
    ]
)]
// A pseudo-class object expands its own keys
#[case(
    StyleResolution::ApplicationOrder,
    "a: { ':hover': { paddingTop: '1px', padding: '2px' } }",
    &["padding-0-2px-_a__c_hover-255"]
)]
// Only the keys StyleX's tables name expand
#[case(
    StyleResolution::LegacyExpandShorthands,
    "a: { 'padding': '1px 2px', 'padding-block': '3px' }",
    &[
        "padding-top-0-1px--255",
        "padding-inline-end-0-2px--255",
        "padding-bottom-0-1px--255",
        "padding-inline-start-0-2px--255",
        "padding-block-0-3px--255",
    ]
)]
#[serial]
fn namespaces_expand_their_keys(
    #[case] policy: StyleResolution,
    #[case] create: &str,
    #[case] expected: &[&str],
) {
    let code = code_under(policy, create, "s");
    assert_eq!(classes(&code, "\"a\": \""), sorted(expected), "{code}");
}

#[rstest]
// A null shorthand resets every key it expands to
#[case(StyleResolution::LegacyExpandShorthands)]
#[case(StyleResolution::ApplicationOrder)]
#[serial]
fn null_shorthands_reset_what_they_cover(#[case] policy: StyleResolution) {
    let code = code_under(
        policy,
        "top: { paddingTop: '1px' }, none: { padding: null }",
        "stylex.props(s.top, s.none)",
    );
    assert!(code.contains("{ className: \"\" }"), "{code}");
}

#[rstest]
#[case(
    StyleResolution::ApplicationOrder,
    "a: { all: 'unset' }",
    "`stylex.create()` cannot use `all` at build time: all is not supported (styleResolution: 'application-order')"
)]
#[case(
    StyleResolution::ApplicationOrder,
    "a: { padding: stylex.firstThatWorks('1px', '2px') }",
    "`stylex.create()` cannot use `padding` at build time: Cannot use fallbacks for shorthands. Use the expansion instead. (styleResolution: 'application-order')"
)]
#[case(
    StyleResolution::LegacyExpandShorthands,
    "a: { listStyle: 'inside outside' }",
    "`stylex.create()` cannot use `listStyle` at build time: invalid \"listStyle\" value of \"inside outside\" (styleResolution: 'legacy-expand-shorthands')"
)]
#[serial]
fn rejected_keys_are_build_errors(
    #[case] policy: StyleResolution,
    #[case] create: &str,
    #[case] expected: &str,
) {
    let code = code_under(policy, create, "s");
    assert!(code.contains(expected), "{expected}\n{code}");
}

// Fallbacks of a longhand stay, as the default reads them
#[test]
#[serial]
fn longhand_fallbacks_stay() {
    let code = code_under(
        StyleResolution::LegacyExpandShorthands,
        "a: { position: stylex.firstThatWorks('sticky', 'fixed') }",
        "s",
    );
    assert_eq!(
        classes(&code, "\"a\": \""),
        sorted(&["position-0-fixed--255", "position-0-sticky--255"]),
        "{code}"
    );
}
