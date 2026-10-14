use crate::{
    ExtractStyleValue,
    assignment_test_support::{compiled_jsx, evaluate, extracted},
    extract_style::numeric_conversion::{NumericConversion, NumericUnit},
};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("const render=(n:number)=><Box p={n}/>")]
#[case("type N=1|2;const render=(n:N)=><Box p={n}/>")]
#[case("interface Props { n: number };const render=(props:Props)=><Box p={props.n}/>")]
#[case("const read=():number=>external();const render=()=> <Box p={read()}/>")]
#[case("const render=(n)=><Box p={n*2}/>")]
#[case("const render=(n)=><Box p={n-2}/>")]
#[case("const render=(n)=><Box p={n/2}/>")]
#[case("const render=(n)=><Box p={n%2}/>")]
#[case("const render=(n)=><Box p={+n}/>")]
#[serial]
fn calc_matches_static_scale_when_source_proves_a_number(#[case] body: &str) {
    let output = extracted(&format!("import {{Box}}from '@devup-ui/react';{body}"));
    let styles = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Dynamic(style) => Some(style),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(styles.len(), 1);
    assert_eq!(
        styles[0].conversion(),
        NumericConversion::Number(NumericUnit::Length)
    );
    assert_eq!(
        styles[0].effective_value(),
        format!("calc(var({}) * 4px)", styles[0].variable_name())
    );
}

#[rstest]
#[case("<Box p={state.value} id={state.id}/>", "4", "[\"16px\"]")]
#[case("<Box p={state.value} id={state.id}/>", "'4'", "[\"16px\"]")]
#[case("<Box p={state.value} id={state.id}/>", "'-2'", "[\"-8px\"]")]
#[case("<Box p={state.value} id={state.id}/>", "'0.5'", "[\"2px\"]")]
#[case("<Box p={state.value} id={state.id}/>", "' 4 '", "[\" 4 \"]")]
#[case("<Box p={state.value} id={state.id}/>", "'auto'", "[\"auto\"]")]
#[case("<Box p={state.value} id={state.id}/>", "'10px'", "[\"10px\"]")]
#[case("<Box p={state.value*1} id={state.id}/>", "4", "[4]")]
#[case("<Box px={state.value} id={state.id}/>", "4", "[\"16px\"]")]
#[case("<Box _hover={{p:state.value}} id={state.id}/>", "4", "[\"16px\"]")]
#[case(
    "<Box selectors={{'& > span':{w:state.value}}} id={state.id}/>",
    "100",
    "[\"400px\"]"
)]
#[case("jsx(Box,{p:state.value,id:state.id})", "4", "[\"16px\"]")]
#[serial]
fn values_follow_static_conversion_when_dynamic_reads_are_captured(
    #[case] expression: &str,
    #[case] input: &str,
    #[case] expected: &str,
) {
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{get value(){{trace.push('value');return {input}}},get id(){{trace.push('id');return 'target'}}}};const jsx=(tag,props)=>props;const node=render(state);JSON.stringify([trace,Object.values(node.style)]);"
    );
    let actual = evaluate(&compiled_jsx(&source));
    assert_eq!(actual, format!("[[\"value\",\"id\"],{expected}]"));
}

#[test]
#[serial]
fn numeric_template_keeps_interpolation_when_quasis_look_like_a_number() {
    let source = "import {Box}from '@devup-ui/react';function render(state){return <Box p={`1${state.value}2`} id={state.id}/>}let trace=[];const state={get value(){trace.push('value');return ''},get id(){trace.push('id');return 'target'}};const node=render(state);JSON.stringify([trace,Object.values(node.style)]);";
    let actual = evaluate(&compiled_jsx(source));
    assert_eq!(actual, "[[\"value\",\"id\"],[\"48px\"]]");
}

#[test]
#[serial]
fn mixed_responsive_values_keep_their_conversion_when_branches_differ() {
    let source = "import {Box}from '@devup-ui/react';function render(state){return <Box p={[state.first,state.flag?state.second*1:state.third,'8px']} id={state.id}/>}let trace=[];const state={};for(const[key,value]of Object.entries({first:4,flag:false,second:2,third:'3',id:'target'}))Object.defineProperty(state,key,{get(){trace.push(key);return value}});const node=render(state);JSON.stringify([trace,Object.values(node.style)]);";
    let actual = evaluate(&compiled_jsx(source));
    assert_eq!(
        actual,
        "[[\"first\",\"flag\",\"third\",\"id\"],[\"16px\",\"12px\"]]"
    );
}

#[test]
#[serial]
fn unit_template_passes_through_when_its_string_cannot_be_numeric() {
    let source = "import {Box}from '@devup-ui/react';function render(state){return <Box p={`${state.value}px`}/>}const node=render({value:10});JSON.stringify(Object.values(node.style));";
    let actual = evaluate(&compiled_jsx(source));
    assert_eq!(actual, "[\"10px\"]");
    let output = extracted(source);
    assert!(output.styles.iter().all(|style| !matches!(style, ExtractStyleValue::Dynamic(style) if style.conversion()!=NumericConversion::Keep)));
}

#[test]
#[serial]
fn shorthand_consumers_share_a_converted_value_when_one_source_site_is_reused() {
    let source = "import {Box}from '@devup-ui/react';function render(state){return <Box px={state.value}/>}let reads=0;const state={get value(){reads++;return 4}};const node=render(state);";
    let actual = crate::assignment_blocker_support::selected(
        source,
        "JSON.stringify([reads,assigned(node),Object.keys(node.style).length]);",
    );
    assert_eq!(
        actual,
        "[1,[\"padding-left:16px:0\",\"padding-right:16px:0\"],1]"
    );
}

#[test]
#[serial]
fn escaped_numeric_template_matches_literal_when_its_cooked_value_is_numeric() {
    let output = extracted(r"import {Box}from '@devup-ui/react';const node=<Box p={`\x34`}/>;");
    let values = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "padding" => {
                Some(style.value())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(values, vec!["16px"]);
}

#[test]
#[serial]
fn different_static_scales_keep_distinct_variables_when_one_shorthand_expands() {
    css::set_custom_shorthands(std::collections::BTreeMap::from([(
        "mixedScale".to_string(),
        vec![
            "padding".to_string(),
            "opacity".to_string(),
            "animation-duration".to_string(),
        ],
    )]));
    let actual = crate::assignment_blocker_support::selected(
        "import {Box}from '@devup-ui/react';function render(state){return <Box mixedScale={state.value}/>}let reads=0;const state={get value(){reads++;return 0.5}};const node=render(state);",
        "JSON.stringify([reads,assigned(node)]);",
    );
    css::set_custom_shorthands(std::collections::BTreeMap::new());
    assert_eq!(
        actual,
        "[1,[\"animation-duration:0.5ms:0\",\"opacity:0.5:0\",\"padding:2px:0\"]]"
    );
}
