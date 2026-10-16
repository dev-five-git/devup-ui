use crate::dead_properties_test_utils::{failure, position};
use crate::{ExtractOption, ExtractStyleValue, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("box-align", "boxAlign")]
#[case("box-pack", "boxPack")]
#[case("box-flex", "boxFlex")]
#[case("box-flex-group", "boxFlexGroup")]
#[case("box-orient", "boxOrient")]
#[case("box-ordinal-group", "boxOrdinalGroup")]
#[case("box-direction", "boxDirection")]
#[case("box-lines", "boxLines")]
#[case("flex-order", "flexOrder")]
#[case("flex-positive", "flexPositive")]
#[case("flex-negative", "flexNegative")]
#[case("flex-preferred-size", "flexPreferredSize")]
#[case("scroll-snap-points-x", "scrollSnapPointsX")]
#[case("scroll-snap-points-y", "scrollSnapPointsY")]
#[case("scroll-snap-coordinate", "scrollSnapCoordinate")]
#[case("scroll-snap-destination", "scrollSnapDestination")]
#[case("scroll-snap-type-x", "scrollSnapTypeX")]
#[case("scroll-snap-type-y", "scrollSnapTypeY")]
#[serial]
fn rejects_closed_property_set_at_exact_keys(
    #[case] canonical: &str,
    #[case] camel: &str,
    #[values(
        "jsx",
        "css",
        "nested",
        "styled",
        "keyframes",
        "global",
        "fontFaces",
        "text",
        "template",
        "styledText",
        "keyframesText",
        "globalText",
        "upperText"
    )]
    api: &str,
) {
    reset_class_map();
    reset_file_map();
    let statement = match api {
        "jsx" => format!("const e = <Box {camel}='1' />;"),
        "css" => format!("const c = css({{ {camel}: '1' }});"),
        "nested" => format!("const c = css({{ selectors: {{ '&:hover': {{ {camel}: '1' }} }} }});"),
        "styled" => format!("const S = styled.div({{ {camel}: '1' }});"),
        "keyframes" => format!("const k = keyframes({{ from: {{ {camel}: '1' }} }});"),
        "global" => format!("globalCss({{ body: {{ {camel}: '1' }} }});"),
        "fontFaces" => format!("globalCss({{ fontFaces: [{{ {camel}: '1' }}] }});"),
        "text" => format!("const c = css('color: red; {canonical}: 1');"),
        "template" => format!("const c = css`color: red; {canonical}: 1`;"),
        "styledText" => format!("const S = styled.div`color: red; {canonical}: 1`;"),
        "keyframesText" => format!("const k = keyframes`from {{ {canonical}: 1 }}`;"),
        "globalText" => format!("globalCss`body {{ {canonical}: 1 }}`;"),
        "upperText" => format!("const c = css`{}: 1`;", canonical.to_ascii_uppercase()),
        _ => unreachable!(),
    };
    let uppercase = canonical.to_ascii_uppercase();
    let name = if api == "upperText" {
        uppercase.as_str()
    } else if statement.contains(camel) {
        camel
    } else {
        canonical
    };
    let column = position(&statement, name) + 1;
    let source = format!(
        "import {{ Box, css, styled, keyframes, globalCss }} from '@devup-ui/react';\n{statement}"
    );
    let error = failure(extract("dead.tsx", &source, ExtractOption::default()));
    assert!(
        error.starts_with(&format!("dead.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains(canonical), "{error}");
    assert!(error.contains("use "), "{error}");
}

#[rstest]
#[case(
    "const c = css({ strokeColor: 'red', imeMode: 'active', futureDraft: 'on', WebkitBoxOrient: 'vertical', p: 2 });"
)]
#[case(
    "const c = css`stroke-color:red;ime-mode:active;future-draft:on;-webkit-box-orient:vertical;padding:8px`; "
)]
#[serial]
fn supported_and_open_ended_declarations_still_emit(#[case] statement: &str) {
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ css }} from '@devup-ui/react';\n{statement}");
    let output = match extract("controls.tsx", &source, ExtractOption::default()) {
        Ok(output) => output,
        Err(error) => panic!("control failed: {error}"),
    };
    let declarations: std::collections::BTreeSet<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property.as_str(), style.value.as_str()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        declarations,
        std::collections::BTreeSet::from([
            ("stroke-color", "red"),
            ("ime-mode", "active"),
            ("future-draft", "on"),
            ("-webkit-box-orient", "vertical"),
            ("padding", "8px"),
        ])
    );
}

#[rstest]
#[case(
    "const c = css`/* box-align: center */ content:'box-align: center'; background:url(box-align:foo); --data: box-align: center; &:is([box-align='center']) { color:red }`; "
)]
#[case("const c = css`box-align:hover { color:red }`; ")]
#[serial]
fn scanner_ignores_names_outside_declaration_keys(#[case] statement: &str) {
    reset_class_map();
    reset_file_map();
    let source = format!("import {{ css }} from '@devup-ui/react';\n{statement}");
    assert!(extract("controls.tsx", &source, ExtractOption::default()).is_ok());
}
