use super::{extract_fixture, rejects_at};
use crate::ExtractStyleValue;
use css::style_selector::StyleSelector;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn group_nth_params_require_an_argument(
    #[values("_groupNthChild", "_groupNthOfType", "_groupNthLastChild")] selector: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{Box}} from '@devup-ui/react';\nexport const view=<Box {selector}={{{{params:[],color:'red'}}}}/>;"
    );
    rejects_at(&source, "[]", single_css);
}

#[rstest]
#[case("unknownKey")]
#[case("tick()")]
#[serial]
fn font_descriptor_keys_reject_unknown_computation(
    #[case] key: &str,
    #[values(false, true)] single_css: bool,
) {
    let statement = format!("globalCss({{fontFaces:[{{[{key}]:'normal',src:'face.woff2'}}]}});");
    let column = crate::dead_properties_test_utils::position(&statement, key) + 1;
    let source = format!("import {{globalCss}} from '@devup-ui/react';\n{statement}");
    let error = crate::dead_properties_test_utils::failure(extract_fixture(&source, single_css));
    let (location, _) = error
        .split_once(": ")
        .unwrap_or_else(|| panic!("unlocated error: {error}"));
    assert_eq!(location, format!("structural.tsx:2:{column}"));
    assert!(error.contains(&format!("cannot use `[{key}]` at build time:")));
}

#[rstest]
#[case("{params:null,color:'red'}", None)]
#[case("{_nthChild:{params:['2'],color:'red'}}", Some("&:nth-child(2)"))]
#[serial]
fn params_keep_the_media_selector_context(
    #[case] rules: &str,
    #[case] expected: Option<&str>,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{Box}} from '@devup-ui/react';\nexport const view=<Box _print={{{rules}}}/>;"
    );
    let output = extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let selectors: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => match style.selector.as_ref() {
                Some(StyleSelector::At { selector, .. }) => selector.as_deref(),
                other => panic!("expected media selector, got {other:?}"),
            },
            other => panic!("expected static declaration, got {other:?}"),
        })
        .collect();
    assert_eq!(selectors, vec![expected]);
}
