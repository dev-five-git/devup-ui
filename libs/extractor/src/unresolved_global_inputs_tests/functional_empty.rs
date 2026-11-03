use super::{extract_fixture, rejects_at};
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn functional_only_selectors_reject_explicit_empty_lists(
    #[values("_is", "_where", "_not", "_has", "_lang", "_part", "_slotted")] selector: &str,
    #[values("[]", "[\"\"]", "[\"\",\"\"]")] params: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{Box}} from '@devup-ui/react';\nexport const view=<Box {selector}={{{{params:{params},color:'red'}}}}/>;"
    );
    rejects_at(
        &source,
        if params == "[]" { "[]" } else { "\"\"" },
        single_css,
    );
}

#[rstest]
#[case(("_hover", "&:hover"))]
#[case(("_host", "&:host"))]
#[case(("_cue", "&::cue"))]
#[case(("_cueRegion", "&::cue-region"))]
#[serial]
fn optional_selectors_keep_the_bare_form_without_arguments(
    #[case] fixture: (&str, &str),
    #[values("[]", "[\"\"]", "null", "undefined")] params: &str,
    #[values(false, true)] single_css: bool,
) {
    let (selector, expected) = fixture;
    let source = format!(
        "import {{Box}} from '@devup-ui/react';\nexport const view=<Box {selector}={{{{params:{params},color:'red'}}}}/>;"
    );
    let output = extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let selectors: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => style
                .selector
                .as_ref()
                .unwrap_or_else(|| panic!("missing selector"))
                .to_string(),
            other => panic!("expected static selector, got {other:?}"),
        })
        .collect();
    assert_eq!(selectors, vec![expected]);
}

#[rstest]
#[case(("_is", "p", "&:is(p)"))]
#[case(("_lang", "en", "&:lang(en)"))]
#[serial]
fn readable_nonempty_lists_keep_existing_empty_text_skips(
    #[case] fixture: (&str, &str, &str),
    #[values(false, true)] single_css: bool,
) {
    let (selector, argument, expected) = fixture;
    let source = format!(
        "import {{Box}} from '@devup-ui/react';\nexport const view=<Box {selector}={{{{params:['','{argument}',''],color:'red'}}}}/>;"
    );
    let output = extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let selectors: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => style
                .selector
                .as_ref()
                .unwrap_or_else(|| panic!("missing selector"))
                .to_string(),
            other => panic!("expected static selector, got {other:?}"),
        })
        .collect();
    assert_eq!(selectors, vec![expected]);
}
