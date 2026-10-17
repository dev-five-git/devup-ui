use crate::{ExtractOption, ExtractOutput, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use serial_test::serial;

fn compile(source: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    extract_without_source_map("/bridge.css.ts", source, ExtractOption::default())
        .map_err(|e| e.to_string())
}

#[rstest::rstest]
#[case("css({p:[1,2],_hover:{p:[1,2],color:'red'}},{p:[3],_hover:{p:[3],w:2}})")]
#[case(
    "css({selectors:{'&:hover, & > a':{_focus:{p:[2,null,3]}}},_media:{'(min-width:1px)':{color:'red'}}})"
)]
#[case(concat!("css({", "'@layer':{base:{_supports:{'(display:grid)':{p:2}}}},_print:{color:'blue'}})"))]
#[case(concat!("css({", "_nthChild:{params:['2n'],p:2,_hover:{w:3}}})"))]
#[case("css({_hover:[{p:1,color:'red'},null,{p:2}],opacity:0.5,styleOrder:2})")]
#[case("css({p:'$space',color:'$brand',_hover:{w:'$space'}})")]
#[case("css(base,{p:[3],selectors:{[`${base} &`]:{animation:`${fade} 1s`,color:'blue'}}})")]
#[case(concat!("css({", "selectors:{'&:hover':{p:2}}},{selectors:{'&:hover':{color:'red'}}})"))]
#[case(concat!("css({", "'@media print':{'@media screen':{p:2}},p:1})"))]
#[case(concat!("css({", "'@media (min-width:1px)':{'@media (max-width:2px)':{p:2}}})"))]
#[case("css({'@layer':{outer:{'@layer':{inner:{p:2}}}}})")]
#[case("css({styleOrder:3,_hover:{styleOrder:7,p:2},p:1})")]
#[case("css('external',base,{p:[3]})")]
#[serial]
fn deep_object_bridge_preserves_shallow_contracts_when_discarded_data_activates_it(
    #[case] expression: &str,
) -> Result<(), String> {
    // Given: an independently compiled shallow control and a semantically ignored deep payload.
    let prefix = "import{css,keyframes}from'@devup-ui/react';const base=css({p:[1,2]});const fade=keyframes({from:{opacity:0},to:{opacity:1}});";
    let expected = compile(&format!("{prefix}export const card={expression};"))?;
    let expression = expression.replacen("css(", "planned(", 1);
    let source = format!(
        "{prefix}function planned(...parts){{let data=1;for(let i=0;i<128;i++)data={{payload:data}};return css(...parts,{{props:data}})}}export const card={expression};"
    );
    // When
    let output = compile(&source)?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[test]
#[serial]
fn deep_object_tag_uses_static_rules_when_pure_interpolation_is_deep() -> Result<(), String> {
    // Given
    let selector = format!("&{}", ":focus:hover".repeat(2048));
    let expected = compile(&format!(
        "import{{css}}from'@devup-ui/react';export const card=css({{selectors:{{'{selector}':{{p:2}}}}}});"
    ))?;
    let source = concat!(
        "import{",
        "css}from'@devup-ui/react';let rules={p:2};for(let i=0;i<2048;i++)rules={_focus:{_hover:rules}};export const card=css`${rules}`;"
    );
    // When
    let output = compile(source)?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[test]
#[serial]
fn deep_object_template_value_rejects_when_interpolation_is_not_a_css_scalar() -> Result<(), String>
{
    // Given: the scalar slot has the same invalid type at shallow and deep shape.
    let shallow = compile(concat!(
        "import{",
        "css}from'@devup-ui/react';export const card=css`opacity:${{p:2}}`;"
    ));
    assert!(shallow.is_err());
    let source = concat!(
        "import{",
        "css}from'@devup-ui/react';let rules={p:2};for(let i=0;i<2048;i++)rules={_hover:rules};export const card=css`opacity:${rules}`;"
    );
    // When
    let result = compile(source);
    // Then
    assert!(
        result.is_err(),
        "non-scalar template value must not disappear"
    );
    Ok(())
}
