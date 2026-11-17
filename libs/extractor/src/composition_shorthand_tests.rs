//! A shorthand written after a longhand it covers wins (#688), whatever order
//! the stylesheet gives the two (#690 writes a longhand after its shorthands):
//! the earlier longhand's atom is dropped when both are static and
//! unconditional at the same selector, breakpoint, layer and style order.
//! JSX attributes between two spreads have no written order (the lint sorts
//! them by name), so only a spread orders them
use crate::{ExtractOption, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

/// The debug class names `code` gives, sorted
fn atoms(filename: &str, code: &str) -> Vec<String> {
    reset_class_map();
    reset_file_map();
    css::debug::set_debug(true);
    let output = extract(filename, code, ExtractOption::default());
    css::debug::set_debug(false);
    let output = output.unwrap_or_else(|error| panic!("{error}"));
    let mut atoms: Vec<String> = output
        .code
        .split(|c: char| c.is_whitespace() || matches!(c, '"' | '`' | '{' | '}' | '$'))
        .filter_map(|token| token.strip_suffix("-a"))
        .map(ToString::to_string)
        .collect();
    atoms.sort_unstable();
    atoms.dedup();
    atoms
}

const CSS: &str = "import { css } from '@devup-ui/react';";
const BOX: &str = "import { Box } from '@devup-ui/react';";
const STYLED: &str = "import { styled } from '@devup-ui/react';";

#[rstest]
#[case::css_object(
    format!("{CSS} export const a = css({{ borderTopWidth: 2, borderWidth: 1 }});"),
    &["border-width-0-4px--255"]
)]
#[case::css_transitive(
    format!("{CSS} export const a = css({{ borderTopWidth: 2, border: 'solid' }});"),
    &["border-0-solid--255"]
)]
#[case::css_place(
    format!("{CSS} export const a = css({{ alignItems: 'start', placeItems: 'center' }});"),
    &["place-items-0-center--255"]
)]
#[case::css_style_order(
    format!("{CSS} export const a = css({{ borderTopWidth: 2, borderWidth: 1, styleOrder: 3 }});"),
    &["border-width-0-4px--3"]
)]
#[case::css_nested(
    format!("{CSS} export const a = css({{ _hover: {{ borderTopWidth: 2, borderWidth: 1 }} }});"),
    &["border-width-0-4px-_a__c_hover-255"]
)]
#[case::css_composed(
    format!("{CSS} const b = css({{ borderTopWidth: 2 }}); export const a = css(b, {{ borderWidth: 1 }});"),
    &["border-top-width-0-8px--255", "border-width-0-4px--255"]
)]
#[case::box_selector(
    format!("{BOX} export const a = <Box _hover={{{{ borderTopWidth: 2, borderWidth: 1 }}}} />;"),
    &["border-width-0-4px-_a__c_hover-255"]
)]
#[case::box_spread(
    format!("{BOX} export const a = <Box {{...{{ borderTopWidth: 2 }}}} borderWidth={{1}} />;"),
    &["border-width-0-4px--255"]
)]
#[case::box_attribute_before_spread(
    format!("{BOX} export const a = <Box borderTopWidth={{2}} {{...{{ borderWidth: 1 }}}} />;"),
    &["border-width-0-4px--255"]
)]
#[case::box_attribute_runs(
    format!("{BOX} export const a = (p) => <Box borderTopWidth={{2}} {{...p}} borderWidth={{1}} />;"),
    &["border-width-0-4px--255"]
)]
#[case::styled_member(
    format!("{STYLED} export const A = styled.div({{ borderTopWidth: 2, borderWidth: 1 }});"),
    &["border-width-0-4px--255"]
)]
#[case::styled_call(
    format!("{STYLED} export const A = styled('div')({{ borderTopWidth: 2, borderWidth: 1 }});"),
    &["border-width-0-4px--255"]
)]
#[serial]
fn later_shorthand_drops_the_longhand_it_covers(#[case] code: String, #[case] expected: &[&str]) {
    assert_eq!(atoms("a.tsx", &code), expected);
}

/// The longhand stays where the build cannot tell the shorthand replaces it:
/// written later, under another selector or breakpoint, under a condition, or
/// at another style order
#[rstest]
#[case::longhand_later(
    format!("{CSS} export const a = css({{ borderWidth: 1, borderTopWidth: 2 }});"),
    &["border-top-width-0-8px--255", "border-width-0-4px--255"]
)]
#[case::other_selector(
    format!("{CSS} export const a = css({{ borderTopWidth: 2, _hover: {{ borderWidth: 1 }} }});"),
    &["border-top-width-0-8px--255", "border-width-0-4px-_a__c_hover-255"]
)]
#[case::other_breakpoint(
    format!("{CSS} export const a = css({{ borderTopWidth: [2, 3], borderWidth: 1 }});"),
    &["border-top-width-1-12px--255", "border-width-0-4px--255"]
)]
#[case::box_props(
    format!("{BOX} export const a = <Box borderTopWidth={{2}} borderWidth={{1}} />;"),
    &["border-top-width-0-8px--255", "border-width-0-4px--255"]
)]
#[case::box_gradient_text(
    format!("{BOX} export const a = <Box backgroundClip=\"text\" bg=\"red\" />;"),
    &["background-0-red--255", "background-clip-0-text--255"]
)]
#[case::box_spread_attribute_runs(
    format!("{BOX} export const a = (p) => <Box {{...p}} borderTopWidth={{2}} borderWidth={{1}} />;"),
    &["border-top-width-0-8px--255", "border-width-0-4px--255"]
)]
#[case::unrelated(
    format!("{CSS} export const a = css({{ paddingTop: 2, margin: 1 }});"),
    &["margin-0-4px--255", "padding-top-0-8px--255"]
)]
#[case::conditional_shorthand(
    format!("{BOX} export const a = (on) => <Box borderTopWidth={{2}} borderWidth={{on ? 1 : 3}} />;"),
    &["border-top-width-0-8px--255", "border-width-0-12px--255", "border-width-0-4px--255"]
)]
#[case::conditional_longhand(
    format!("{BOX} export const a = (on) => <Box borderTopWidth={{on ? 2 : 3}} borderWidth={{1}} />;"),
    &["border-top-width-0-12px--255", "border-top-width-0-8px--255", "border-width-0-4px--255"]
)]
#[case::other_style_order(
    format!("{CSS} const b = css({{ borderTopWidth: 2, styleOrder: 3 }}); export const a = css(b, {{ borderWidth: 1 }});"),
    &["border-top-width-0-8px--3", "border-width-0-4px--255"]
)]
#[serial]
fn longhand_stays_unless_a_known_later_shorthand_replaces_it(
    #[case] code: String,
    #[case] expected: &[&str],
) {
    assert_eq!(atoms("a.tsx", &code), expected);
}

/// A global rule writes its declarations in the order written, so it keeps
/// both: the later shorthand already wins there
#[test]
#[serial]
fn global_rule_keeps_both_declarations() {
    reset_class_map();
    reset_file_map();
    let code = "import { globalCss } from '@devup-ui/react'; globalCss({ body: { marginTop: 10, margin: 0 } });";
    let output =
        extract("a.tsx", code, ExtractOption::default()).unwrap_or_else(|error| panic!("{error}"));
    let mut properties: Vec<String> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(style) => Some(style.property.clone()),
            _ => None,
        })
        .collect();
    properties.sort_unstable();
    assert_eq!(properties, ["margin", "margin-top"]);
}

/// A shorthand held by a variable does not replace a longhand, nor is a
/// longhand held by one replaced
#[rstest]
#[case(
    "<Box borderTopWidth={2} borderWidth={width} />",
    "border-top-width-0-8px--255"
)]
#[case(
    "<Box borderTopWidth={width} borderWidth={1} />",
    "border-width-0-4px--255"
)]
#[serial]
fn dynamic_atoms_stay(#[case] element: &str, #[case] static_atom: &str) {
    let code = format!("{BOX} export const a = (width) => {element};");
    let atoms = atoms("a.tsx", &code);
    assert!(atoms.iter().any(|atom| atom == static_atom), "{atoms:?}");
    assert_eq!(atoms.len(), 2, "{atoms:?}");
}
