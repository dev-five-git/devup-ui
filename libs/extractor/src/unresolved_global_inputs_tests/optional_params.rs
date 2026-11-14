use super::extract_fixture;
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn factory_values_compile_when_selector_params_are_proven_absent(
    #[values(false, true)] single_css: bool,
) {
    // Given a factory value that must compile inside a selector with absent params.
    let source = "import {Box,css} from '@devup-ui/react';\nimport {jsx} from 'react/jsx-runtime';\nexport const view=jsx(Box,{_hover:{params:undefined,other:css({overflow:'hidden'})}});";
    // When the factory and its nested style call are extracted together.
    let output = extract_fixture(source, single_css).unwrap_or_else(|error| panic!("{error}"));
    // Then the nested call contributes its styles and the factory keeps its value slot.
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property == "overflow" && style.value == "hidden"
    )));
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
            .count(),
        1
    );
}

#[rstest]
#[case(("Box", "export const view=<Box _hover={{params:ABSENT,color:'red'}}/>;"))]
#[case(("globalCss", "globalCss({body:{_hover:{params:ABSENT,color:'red'}}});"))]
#[case(("styled", "export const Card=styled('div',{_hover:{params:ABSENT,color:'red'}});"))]
#[case(("Box", "export const view=jsx(Box,{_hover:{params:ABSENT,color:'red'}});"))]
#[case(("css", "export const view=css({_hover:{params:ABSENT,color:'red'}});"))]
#[serial]
fn optional_selector_params_keep_proven_absence(
    #[case] fixture: (&str, &str),
    #[values("undefined", "null", "void 0", "void void 0", "void null")] absent: &str,
    #[values(false, true)] single_css: bool,
) {
    let (imports, statement) = fixture;
    let source = format!(
        "import {{{imports}}} from '@devup-ui/react';\nimport {{jsx}} from 'react/jsx-runtime';\n{}",
        statement.replace("ABSENT", absent)
    );
    let output = extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let declarations: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "color" => Some((
                style.value.as_str(),
                style
                    .selector
                    .as_ref()
                    .unwrap_or_else(|| panic!("missing selector"))
                    .to_string(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(declarations.len(), 1);
    assert_eq!(declarations[0].0, "red");
    assert!(declarations[0].1.ends_with(":hover"));
}

#[rstest]
#[serial]
fn optional_selector_params_reject_shadowed_undefined(#[values(false, true)] single_css: bool) {
    let source = "import {Box} from '@devup-ui/react';\nexport function view(undefined) {return <Box _hover={{params:undefined,color:'red'}}/>;}";
    let offset = source
        .rfind("undefined")
        .unwrap_or_else(|| panic!("missing fixture reference"));
    let before = &source[..offset];
    let column = before
        .rsplit_once('\n')
        .map_or(before, |(_, tail)| tail)
        .chars()
        .count()
        + 1;
    let error = crate::dead_properties_test_utils::failure(extract_fixture(source, single_css));
    assert_eq!(
        error
            .split_once(": ")
            .unwrap_or_else(|| panic!("unlocated error: {error}"))
            .0,
        format!("structural.tsx:2:{column}")
    );
    assert!(error.contains("cannot use `undefined` at build time:"));
}

#[rstest]
#[serial]
fn absent_params_keep_authored_winners_when_known_css_is_composed_into_styled(
    #[values(false, true)] single_css: bool,
) {
    // Given competing aliases at responsive levels and in separately authored layers.
    let source = "import {css,styled} from '@devup-ui/react';\nconst base=css({_hover:{params:undefined,padding:[3,5]}});\nconst Choice=styled.div(base,{_hover:{params:void 0,p:[4,6]},'@layer':{first:{_hover:{params:undefined,padding:7,p:8}},second:{_hover:{params:null,p:9}}}});";
    // When the complete generated renderer selects its composed class names.
    let output = extract_fixture(source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let bucket = (!single_css).then(|| css::file_map::canonical("structural.tsx"));
    let active = crate::styled_source_tests::captured_render::active(
        ("structural.tsx", bucket.as_deref()),
        &output,
        "{}",
    );
    // Then only later authored aliases win, without losing selector, breakpoint or layer.
    let mut declarations: Vec<_> = active
        .into_iter()
        .map(|value| match value {
            ExtractStyleValue::Static(style) => (
                style.layer,
                style.level,
                style.property,
                style.value,
                style.selector.map(|selector| selector.to_string()),
            ),
            other => panic!("expected static composed declaration: {other:?}"),
        })
        .collect();
    declarations.sort();
    let expected = [
        (None, 0, "16px"),
        (None, 1, "24px"),
        (Some("first"), 0, "32px"),
        (Some("second"), 0, "36px"),
    ]
    .map(|(layer, level, value)| {
        (
            layer.map(str::to_string),
            level,
            "padding".to_string(),
            value.to_string(),
            Some("&:hover".to_string()),
        )
    });
    assert_eq!(declarations, expected);
    let layers: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| match value {
            ExtractStyleValue::LayerOrder(order) => Some(order.layers.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        layers,
        vec![vec!["first".to_string(), "second".to_string()]]
    );
    let repeated = extract_fixture(source, single_css).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(output.code, repeated.code);
    assert_eq!(output.styles, repeated.styles);
}

#[rstest]
#[case("css(base,{'@layer':{base:{_hover:{params:undefined,padding:3,p:4}}}})")]
#[case("styled.div(base,{'@layer':{base:{_hover:{params:undefined,padding:3,p:4}}}})")]
#[case("styled.div(base,p=>p.on?{_hover:{params:undefined,padding:3,p:4}}:{color:'blue'})")]
#[serial]
fn composed_projection_rejects_shadowed_params_at_the_authored_leaf(
    #[case] expression: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given a lexical undefined binding inside a composition, not the absent global.
    let source = format!(
        "import {{css,styled}} from '@devup-ui/react';\nconst base=css({{color:'red'}});\nexport function view(undefined) {{return {expression};}}"
    );
    let offset = source
        .rfind("undefined")
        .unwrap_or_else(|| panic!("missing params"));
    let column = source[..offset]
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count()
        + 1;
    // When authored-order extraction traverses the layer or conditional callback.
    let error = crate::dead_properties_test_utils::failure(extract_fixture(&source, single_css));
    // Then the original params leaf is rejected, never erased by composition or retry.
    assert!(
        error.starts_with(&format!("structural.tsx:3:{column}: ")),
        "{error}"
    );
    assert!(
        error.contains("cannot use `undefined` at build time:"),
        "{error}"
    );
    assert!(
        error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
        "{error}"
    );
}

#[rstest]
#[case(false, "12px")]
#[case(true, "16px")]
#[serial]
fn absent_params_keep_the_previous_atom_when_a_projected_callback_branch_omits_it(
    #[case] on: bool,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given a later callback branch that either replaces padding or keeps its earlier alias.
    let source = "import {styled} from '@devup-ui/react';\nconst Choice=styled.div({_hover:{params:undefined,padding:3}},p=>p.on?{_hover:{params:undefined,p:4}}:{_hover:{params:void 0}});";
    // When the original generated renderer chooses a branch with runtime props.
    let output = extract_fixture(source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let bucket = (!single_css).then(|| css::file_map::canonical("structural.tsx"));
    let active = crate::styled_source_tests::captured_render::active(
        ("structural.tsx", bucket.as_deref()),
        &output,
        &format!("{{on:{on}}}"),
    );
    // Then the selected class carries the authored winner, never an empty replacement.
    let declarations: Vec<_> = active
        .into_iter()
        .map(|value| match value {
            ExtractStyleValue::Static(style) => (
                style.property,
                style.value,
                style.selector.map(|selector| selector.to_string()),
            ),
            other => panic!("expected static branch winner: {other:?}"),
        })
        .collect();
    assert_eq!(
        declarations,
        vec![(
            "padding".to_string(),
            expected.to_string(),
            Some("&:hover".to_string())
        )]
    );
}
