use super::extract_fixture;
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn successful_globals_keep_authored_order(#[values(false, true)] single_css: bool) {
    // Given distinct siblings and successive calls whose order matters.
    let source = "import { globalCss } from '@devup-ui/react';\nglobalCss({imports: ['first.css', 'second.css'], fontFaces: [{fontFamily: 'First', src: 'first.woff2'}], body: {color: 'red'}});\nglobalCss({imports: ['third.css'], fontFaces: [{fontFamily: 'Second', src: 'second.woff2'}], body: {color: 'blue'}});";
    // When both global calls are extracted into the real collector.
    let output = extract_fixture(source, single_css).unwrap_or_else(|error| panic!("{error}"));
    // Then source ordering and file ownership survive all three structural kinds.
    let mut ordered: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Import(import) => {
                (import.order, import.url.as_str(), import.file.as_str())
            }
            ExtractStyleValue::FontFace(face) => (
                face.order,
                face.properties["font-family"].as_str(),
                face.file.as_str(),
            ),
            ExtractStyleValue::Static(style) => match style
                .selector
                .as_ref()
                .unwrap_or_else(|| panic!("missing selector"))
            {
                css::style_selector::StyleSelector::Global(selector, file) => {
                    assert_eq!(selector, "body");
                    (style.order, style.value.as_str(), file.as_str())
                }
                other => panic!("expected global selector, got {other:?}"),
            },
            other => panic!("expected ordered global style, got {other:?}"),
        })
        .collect();
    ordered.sort_unstable();
    assert_eq!(
        ordered,
        vec![
            (1, "first.css", "structural.tsx"),
            (2, "second.css", "structural.tsx"),
            (3, "First", "structural.tsx"),
            (4, "red", "structural.tsx"),
            (5, "third.css", "structural.tsx"),
            (6, "Second", "structural.tsx"),
            (7, "blue", "structural.tsx"),
        ]
    );
}

#[rstest]
#[case("export const view = <Box color={runtimeColor} />;")]
#[case("export const Card = styled('article', {color: runtimeColor});")]
#[serial]
fn ordinary_dynamic_values_still_use_element_css_variables(
    #[case] statement: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given a dynamic declaration value, not a structural input.
    let source = format!("import {{Box, styled}} from '@devup-ui/react';\n{statement}");
    // When extraction processes the ordinary element value.
    let output = extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    // Then it retains the existing dynamic declaration rather than rejecting it.
    let values: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Dynamic(style) => style.property(),
            other => panic!("expected dynamic declaration, got {other:?}"),
        })
        .collect();
    assert_eq!(values, vec!["color"]);
    assert!(output.code.contains("runtimeColor"));
    assert!(output.code.contains("--"));
}
