use super::rejects_at;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("[unknownParam]", "unknownParam")]
#[case("[tick()]", "tick()")]
#[case("unknownParams", "unknownParams")]
#[case("tick()", "tick()")]
#[case("['2', unknownParam]", "unknownParam")]
#[case("['2', tick()]", "tick()")]
#[case("[...unknownParams]", "unknownParams")]
#[case("[...tick()]", "tick()")]
#[case("[]", "[]")]
#[case("[\"\"]", "\"\"")]
#[case("[\"\", \"\"]", "\"\"")]
#[case("[void tick()]", "void tick()")]
#[serial]
fn rejects_nth_child_params_without_partial_or_empty_arguments(
    #[case] params: &str,
    #[case] marker: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ Box }} from '@devup-ui/react';\nexport const view = <Box _nthChild={{{{params: {params}, color: 'red'}}}} />;"
    );
    rejects_at(&source, marker, single_css);
}

#[rstest]
#[case("export const c = css({_nthChild: {params: ['2', unknownParam], color: 'red'}});")]
#[case("globalCss({body: {_nthChild: {params: ['2', unknownParam], color: 'red'}}});")]
#[case("export const S = styled.div({_nthChild: {params: ['2', unknownParam], color: 'red'}});")]
#[case("export const view = jsx(Box, {_nthChild: {params: ['2', unknownParam], color: 'red'}});")]
#[case(
    "export const view = jsxs(Box, {_nthChild: {params: ['2', unknownParam], color: 'red'}, children: ['a', 'b']});"
)]
#[case("globalCss('body', {_nthChild: {params: ['2', unknownParam], color: 'red'}});")]
#[serial]
fn every_params_caller_retains_the_original_entry_location(
    #[case] statement: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ Box, css, globalCss, styled }} from '@devup-ui/react';\nimport {{ jsx, jsxs }} from 'react/jsx-runtime';\n{statement}"
    );
    rejects_at(&source, "unknownParam", single_css);
}

#[rstest]
#[serial]
fn advanced_selectors_reject_unresolved_params(
    #[values("_is", "_where", "_not", "_has", "_nthOfType")] selector: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ Box }} from '@devup-ui/react';\nexport const view = <Box {selector}={{{{params: ['p', unknownParam], color: 'red'}}}} />;"
    );
    rejects_at(&source, "unknownParam", single_css);
}

#[rstest]
#[case("['2']", "&:nth-child(2)")]
#[case("[`2${''}`]", "&:nth-child(2)")]
#[serial]
fn literal_and_pure_params_keep_the_complete_selector(
    #[case] params: &str,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ Box }} from '@devup-ui/react';\nexport const view = <Box _nthChild={{{{params: {params}, color: 'red'}}}} />;"
    );
    let output =
        super::extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let styles: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::ExtractStyleValue::Static(style) => (
                style.property.as_str(),
                style.value.as_str(),
                style
                    .selector
                    .as_ref()
                    .unwrap_or_else(|| panic!("missing selector"))
                    .to_string(),
            ),
            other => panic!("expected static selector style, got {other:?}"),
        })
        .collect();
    assert_eq!(styles, vec![("color", "red", expected.to_string())]);
}

#[rstest]
#[serial]
fn optional_params_on_simple_selectors_remain_optional(#[values(false, true)] single_css: bool) {
    let source = "import {Box} from '@devup-ui/react';\nexport const view = <Box _hover={{color: ['red', null, 'blue']}} />;";
    let output =
        super::extract_fixture(source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let mut styles: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::ExtractStyleValue::Static(style) => (
                style.level,
                style.value.as_str(),
                style
                    .selector
                    .as_ref()
                    .unwrap_or_else(|| panic!("missing selector"))
                    .to_string(),
            ),
            other => panic!("expected responsive hover style, got {other:?}"),
        })
        .collect();
    styles.sort_unstable();
    assert_eq!(
        styles,
        vec![
            (0, "red", "&:hover".to_string()),
            (2, "blue", "&:hover".to_string()),
        ]
    );
}
