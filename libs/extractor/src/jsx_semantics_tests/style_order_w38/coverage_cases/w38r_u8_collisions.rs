use super::*;
use crate::extract_style::extract_static_style::ThemeTokenResolution;
use rstest::rstest;
use std::collections::BTreeMap;

/// Registers `$containerX` as a two-breakpoint length token, then clears it.
struct Tokens;

impl Tokens {
    fn registered() -> Self {
        css::theme_tokens::set_theme_token_levels(
            BTreeMap::from([("containerX".to_string(), vec![0, 2])]),
            BTreeMap::new(),
        );
        Self
    }
}

impl Drop for Tokens {
    fn drop(&mut self) {
        css::theme_tokens::set_theme_token_levels(BTreeMap::new(), BTreeMap::new());
    }
}

type Declaration = (&'static str, &'static str, u8, ThemeTokenResolution);

fn declarations(actual: &ExtractOutput) -> Vec<(String, String, u8, ThemeTokenResolution)> {
    let mut found: Vec<_> = actual
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((
                style.property().to_string(),
                style.value().to_string(),
                style.level(),
                style.theme_token_resolution(),
            )),
            _ => None,
        })
        .collect();
    found.sort_unstable();
    found
}

const ARRAY_FIRST: &str = "const a=css({width:['$containerX',null,'$containerX']});";
const WIDTH: &str = "width-0-_dl_containerX--255-a width-2-_dl_containerX--255-a";

#[rstest]
#[case::same_classes_other_resolution(
    "const b=css({width:'$containerX'});",
    "a.tsx:1:171: `css()` cannot use `c` at build time: its styles must be an object literal or a constant object, or be computed from constants"
)]
#[serial]
fn w38r_u8_choice_when_one_class_text_has_two_declaration_sets_is_not_a_known_style(
    #[case] other: &str,
    #[case] expected: &str,
) {
    // Given: array elements keep the first value of a responsive token, while the
    // plain value expands it, so both emit the same two classes with unequal values.
    let _tokens = Tokens::registered();
    let source = format!(
        "import {{css}} from '@devup-ui/react';let flag=true;{ARRAY_FIRST}{other}const c=flag?a:b;const d=css(c,{{color:'red'}});"
    );
    // When: public extraction composes the conditional choice of those two classes.
    let actual = compile(&source);
    // Then: the choice is no finite style, so the composition reports it at the call.
    assert_eq!(actual.err().as_deref(), Some(expected));
}

#[rstest]
#[case::distinct_classes("const b=css({height:'1px'});", &[
    ("color", "red", 0, ThemeTokenResolution::CssVariable),
    ("height", "1px", 0, ThemeTokenResolution::CssVariable),
    ("width", "$containerX", 0, ThemeTokenResolution::FirstValue),
    ("width", "$containerX", 2, ThemeTokenResolution::FirstValue),
])]
#[serial]
fn w38r_u8_choice_when_class_texts_differ_stays_a_known_style(
    #[case] other: &str,
    #[case] expected: &[Declaration],
) {
    // Given: the same shape of choice whose two classes never share a text.
    let _tokens = Tokens::registered();
    let source = format!(
        "import {{css}} from '@devup-ui/react';let flag=true;{ARRAY_FIRST}{other}const c=flag?a:b;const d=css(c,{{color:'red'}});"
    );
    // When: public extraction composes it and the result runs.
    let actual = output(&source);
    let evaluated = whole::evaluate_code(&actual.code, "[a,d]");
    // Then: every declaration is kept with its token resolution and the choice renders.
    assert_eq!(
        evaluated.element,
        serde_json::json!([WIDTH, format!("color-0-red--255-a {WIDTH} ")])
    );
    let expected: Vec<_> = expected
        .iter()
        .map(|(property, value, level, resolution)| {
            (
                (*property).to_string(),
                (*value).to_string(),
                *level,
                *resolution,
            )
        })
        .collect();
    assert_eq!(declarations(&actual), expected);
}

#[rstest]
#[case::spread_beside_marker(
    "css({__devupLiteralMixin:base,...{backgroundColor:'blue'},styleOrder:255})",
    "a.tsx:1:153: `styleOrder()` cannot use `255` at build time: an explicit styleOrder must be an ECMAScript Number integer from 1 to 254 or canonical decimal string without signs, spaces or leading zeros"
)]
#[serial]
fn w38r_u8_local_rules_when_source_spells_the_marker_beside_a_spread_report_the_order(
    #[case] rules: &str,
    #[case] expected: &str,
) {
    // Given: authored source can spell the generated mixin key and add a spread.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';const render=base=><ClassNames>{{({{css}})=>{rules}}}</ClassNames>;"
    );
    // When: the local emotion path reads the object literal as a mixin scope.
    let actual = compile_emotion(&source);
    // Then: the invalid order is reported at its authored value.
    assert_eq!(actual.err().as_deref(), Some(expected));
}

#[rstest]
#[case::getter("get styleOrder(){return 2}", 64, "(function() { return 2; })")]
#[case::method("styleOrder(){return 2}", 60, "(function() { return 2; })")]
#[case::setter("set styleOrder(value){}", 64, "(function(value) {})")]
#[serial]
fn w38r_u8_css_when_style_order_is_an_accessor_or_method_reports_the_function_value(
    #[case] member: &str,
    #[case] column: u32,
    #[case] code: &str,
) {
    // Given: an accessor or method is the only way source writes a non-plain order member.
    let source =
        format!("import {{css}} from '@devup-ui/react';const a=css({{{member},color:'red'}});");
    // When: public extraction checks the declaration object.
    let actual = error(&source);
    // Then: its function value is the invalid order, so no accessor-specific rejection exists.
    assert_eq!(
        actual,
        format!(
            "a.tsx:1:{column}: `styleOrder()` cannot use `{code}` at build time: an explicit styleOrder must be an ECMAScript Number integer from 1 to 254 or canonical decimal string without signs, spaces or leading zeros"
        )
    );
}
