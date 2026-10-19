use super::{extract_fixture, rejects_at};
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("fontFaces: unknownFaces", "unknownFaces")]
#[case("fontFaces: tick()", "tick()")]
#[case("fontFaces: [unknownFace]", "unknownFace")]
#[case("fontFaces: [...unknownFaces]", "unknownFaces")]
#[serial]
fn structural_boundaries_reject_effects_and_unknown_font_collections(
    #[case] properties: &str,
    #[case] marker: &str,
    #[values(false, true)] single_css: bool,
) {
    let source =
        format!("import {{globalCss}} from '@devup-ui/react';\nglobalCss({{{properties}}});");
    rejects_at(&source, marker, single_css);
}

#[rstest]
#[case("imports: [{url: 'base.css', query: (tick(), undefined)}]")]
#[case("fontFaces: [{fontFamily: 'Face', fontStyle: (tick(), undefined)}]")]
#[case("body: {_nthChild: {params: [(tick(), undefined)], color: 'red'}}")]
#[serial]
fn optional_looking_sequences_keep_effects_and_original_locations(
    #[case] properties: &str,
    #[values(false, true)] single_css: bool,
) {
    let statement = format!("globalCss({{{properties}}});");
    let column = crate::dead_properties_test_utils::position(&statement, "(tick(), undefined)") + 1;
    let source = format!("import {{globalCss}} from '@devup-ui/react';\n{statement}");
    let error = crate::dead_properties_test_utils::failure(extract_fixture(&source, single_css));
    let (diagnostic, requirement) = error
        .split_once(" at build time: ")
        .unwrap_or_else(|| panic!("unexpected diagnostic: {error}"));
    assert_eq!(
        diagnostic,
        format!("structural.tsx:2:{column}: `globalCss()` cannot use `tick(), undefined`")
    );
    assert_ne!(requirement, "");
}

#[rstest]
#[case("imports: [1, , null, undefined]", 0)]
#[case("fontFaces: [1, , null, undefined]", 0)]
#[case("fontFaces: null", 0)]
#[case("fontFaces: 17", 0)]
#[case("fontFaces: undefined", 0)]
#[case("imports: [{...{url: 'face.css', query: 'print'}}]", 1)]
#[case("fontFaces: [{...{fontFamily: 'Face', src: 'face.woff2'}}]", 1)]
#[serial]
fn structural_boundaries_keep_absence_and_literal_spreads(
    #[case] properties: &str,
    #[case] expected: usize,
    #[values(false, true)] single_css: bool,
) {
    let source =
        format!("import {{globalCss}} from '@devup-ui/react';\nglobalCss({{{properties}}});");
    let output = extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(output.styles.len(), expected);
    for style in &output.styles {
        match style {
            ExtractStyleValue::Import(import) => assert_eq!(import.url, "\"face.css\" print"),
            ExtractStyleValue::FontFace(face) => assert_eq!(face.properties["font-family"], "Face"),
            other => panic!("expected literal structural style, got {other:?}"),
        }
    }
}

#[rstest]
#[serial]
fn params_preserve_joining_and_nonrequired_absence(#[values(false, true)] single_css: bool) {
    let source = "import {Box} from '@devup-ui/react';\nexport const view = <Box _is={{params: ['p', , 'a'], color: 'red'}} _hover={{params: null, color: 'blue'}} />;";
    let output = extract_fixture(source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let mut selectors: Vec<_> = output
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
    selectors.sort_unstable();
    assert_eq!(selectors, vec!["&:hover", "&:is(p,a)"]);
}
