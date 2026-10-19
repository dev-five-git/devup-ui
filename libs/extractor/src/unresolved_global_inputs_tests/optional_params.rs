use super::extract_fixture;
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

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
