use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("(Box)")]
#[case("Box as typeof Box")]
#[case("Box satisfies typeof Box")]
#[serial]
fn wrapped_component_bindings_when_create_element_reads_them_compile(#[case] component: &str) {
    let source = format!(
        "{BOX}import {{createElement}} from 'react'; export const a=()=>createElement({component},{{color:'red'}},'child');"
    );
    let actual = evaluate(&source, "a()");
    assert_eq!(actual.element["type"], "div");
    assert_eq!(actual.element["children"], serde_json::json!(["child"]));
}

#[test]
#[serial]
fn shadowed_undefined_when_used_as_props_is_forwarded() {
    let source = format!(
        "{BOX}import {{createElement}} from 'react';export const a=(undefined)=>createElement(Box,undefined,'child');"
    );
    let actual = evaluate(&source, "a({id:'kept'})");
    assert_eq!(actual.element["props"]["id"], "kept");
}

#[test]
#[serial]
fn void_effect_when_used_as_props_runs_before_children() {
    let source = format!(
        "{BOX}import {{createElement}} from 'react';export const a=()=>createElement(Box,void trace.push('props'),(trace.push('children'),'child'));"
    );
    let actual = evaluate(&source, "a()");
    assert_eq!(actual.trace, serde_json::json!(["props", "children"]));
}

#[rstest]
#[case("undefined")]
#[case("void effect()")]
#[case("cond ? 1 : undefined")]
#[case("cond ? void effect() : 2")]
#[serial]
fn uncertain_empty_order_when_shadowed_or_effectful_is_located(#[case] order: &str) {
    for element in [
        format!("<Box styleOrder={{{order}}} color=\"red\" />"),
        format!("jsx(Box,{{styleOrder:{order},color:'red'}})"),
    ] {
        let source =
            format!("{BOX}{JSX_RUNTIME}export const a=(undefined,effect,cond)=>{element};");
        let message = error(&source);
        assert!(message.starts_with("a.tsx:3:"), "{message}");
        assert!(message.contains(order), "{message}");
    }
}

#[test]
#[serial]
fn shadowed_globals_when_spreads_copy_props_do_not_capture_generated_names() {
    let source = format!(
        "{BOX}export const a=(Object,undefined,__devupSpread0,__devupValue0)=> <Box color={{__devupValue0}} {{...__devupSpread0}} />;"
    );
    let actual = evaluate(&source, "a(null,'not-undefined',{color:void 0},'red')");
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .is_some_and(serde_json::Map::is_empty)
    );
}

#[rstest]
#[case("jsx(Box,{id:first(),as:type(),color:'red',...rest},child())")]
#[case("createElement(Box,{id:first(),as:type(),color:'red',...rest},child())")]
#[case("React.createElement(Box,{id:first(),as:type(),color:'red',...rest},child())")]
#[serial]
fn dynamic_as_when_props_are_captured_keeps_type_at_its_source_position(#[case] element: &str) {
    let source = format!(
        "{BOX}{JSX_RUNTIME}import React,{{createElement}} from 'react';export const a=(first,type,rest,child)=>{element};"
    );
    let actual = evaluate(
        &source,
        "a(()=>{trace.push('id');return 'id';},()=>{trace.push('type');return 'section';},{get color(){trace.push('spread');return 'blue';}},()=>{trace.push('child');return 'child';})",
    );
    assert_eq!(actual.element["type"], "section");
    assert_eq!(
        actual.trace,
        serde_json::json!(["id", "type", "spread", "child"])
    );
}

#[test]
#[serial]
fn dynamic_as_when_member_factory_looks_up_a_receiver_keeps_it() {
    let source = format!(
        "{BOX}import React from 'react';export const a=()=>React.createElement(Box,{{color:(trace.push('color'),'red'),as:(trace.push('type'),'a')}},(trace.push('child'),'child'));"
    );
    let actual = evaluate(
        &source,
        "(()=>{React.marker='receiver';Object.defineProperty(React,'createElement',{get(){trace.push('callee');return function(type,props,...children){return {type,props,children,receiver:this.marker};};}});return a();})()",
    );
    assert_eq!(actual.element["receiver"], "receiver");
    assert_eq!(
        actual.trace,
        serde_json::json!(["callee", "color", "type", "child"])
    );
}

#[test]
#[serial]
fn shadowed_undefined_when_a_conditional_style_is_absent_stays_absent() {
    let source =
        format!("{BOX}export const a=(undefined,active,value)=><Box color={{active && value}}/>;");
    let actual = evaluate(&source, "a('incorrect',false,'red')");
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .is_none_or(serde_json::Map::is_empty),
        "{}",
        actual.element
    );
}

#[test]
#[serial]
fn locally_compiled_keyframes_when_capture_precedes_props_keep_static_rules() {
    let source = "import {Box,keyframes} from '@devup-ui/react';export function a(props){const spin=keyframes({from:{opacity:0},to:{opacity:1}});return <Box animationName={spin} props={{...props}}/>;}";
    let actual = output(source);
    assert!(
        static_styles(&actual)
            .iter()
            .any(|(property, _)| property == "animation-name"),
        "{}",
        actual.code
    );
    assert!(!actual.styles.iter().any(|style|matches!(style,ExtractStyleValue::Dynamic(style) if style.property()=="animation-name")),"{}",actual.code);
}

#[rstest]
#[case("<Box id={take('id')} as={type} {...rest} title={take('title')}>{take('child')}</Box>")]
#[case("createElement(Box,{id:take('id'),as:type,...rest,title:take('title')},take('child'))")]
#[serial]
fn absent_dynamic_as_when_captured_defaults_without_reordering_later_props(#[case] element: &str) {
    let source = format!(
        "{BOX}import {{createElement}} from 'react';export const a=(type,rest,take)=>{element};"
    );
    let actual = evaluate(
        &source,
        "a(null,{get x(){trace.push('spread');return 1;}},key=>{trace.push(key);return key;})",
    );
    assert_eq!(actual.element["type"], "div");
    assert_eq!(
        actual.trace,
        serde_json::json!(["id", "spread", "title", "child"])
    );
}

#[rstest]
#[case("({lg:'large',sm:'small'})[size]")]
#[case("({lg:'large',sm:'small'} as const)[size]")]
#[serial]
fn wrapped_written_typography_when_captured_keeps_its_known_tokens(#[case] value: &str) {
    let source = format!(
        "{BOX}export const a=(size,rest)=><Box typography={{{value}}} title={{rest.id}}/>;"
    );
    let actual = output(&source);
    assert!(
        actual
            .styles
            .iter()
            .any(|style| matches!(style,ExtractStyleValue::Typography(name) if name=="large")),
        "{}",
        actual.code
    );
    assert!(
        actual
            .styles
            .iter()
            .any(|style| matches!(style,ExtractStyleValue::Typography(name) if name=="small")),
        "{}",
        actual.code
    );
}
