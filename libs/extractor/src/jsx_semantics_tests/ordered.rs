use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box color={first()} {...read()} />")]
#[case("jsx(Box, {color:first(), ...read()})")]
#[case("createElement(Box, {color:first(), ...read()})")]
#[case("<Box {...{color:first()}} {...read()} />")]
#[serial]
fn explicit_effects_when_overwritten_still_run_before_spreads(#[case] element: &str) {
    let source = format!(
        "{BOX}{JSX_RUNTIME}import {{createElement}} from 'react'; export const a = (first, read) => {element};"
    );
    let actual = evaluate(
        &source,
        "a(() => {trace.push('first'); return 'red';}, () => {trace.push('spread'); return {color:'blue'};})",
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert_eq!(actual.element["props"]["color"], "blue");
}

#[rstest]
#[case("<Box color=\"red\" bg=\"gray\" {...rest} />")]
#[case("jsx(Box, {color:'red', bg:'gray', ...rest})")]
#[case("createElement(Box, {color:'red', bg:'gray', ...rest})")]
#[serial]
fn getters_when_props_are_forwarded_and_selected_run_once(#[case] element: &str) {
    let source = format!(
        "{BOX}{JSX_RUNTIME}import {{createElement}} from 'react'; export const a = (rest) => {element};"
    );
    let actual = evaluate(
        &source,
        "a({get color(){trace.push('color');return 'blue';},get className(){trace.push('class');return 'user';},get style(){trace.push('style');return {opacity:0.5};}})",
    );
    assert_eq!(actual.trace, serde_json::json!(["color", "class", "style"]));
    assert_eq!(actual.element["props"]["style"]["opacity"], 0.5);
}

#[rstest]
#[case("<Box color={x} id={change()} />")]
#[case("jsx(Box, {color:x, id:change()})")]
#[case("createElement(Box, {color:x, id:change()})")]
#[serial]
fn earlier_identifier_when_later_props_mutate_it_keeps_its_value(#[case] element: &str) {
    let source = format!(
        "{BOX}{JSX_RUNTIME}import {{createElement}} from 'react'; export function a() {{let x='red';function change(){{x='blue';return 'id';}} return {element};}}"
    );
    let actual = evaluate(&source, "a()");
    let style = actual.element["props"]["style"]
        .as_object()
        .unwrap_or_else(|| panic!("{}", actual.element));
    assert!(
        style.values().any(|value| value == "red"),
        "{}",
        actual.element
    );
}

#[test]
#[serial]
fn mixed_branches_when_captured_stay_lazy_and_keep_static_spacing() {
    let source = format!(
        "{BOX}export const a=(active, value, rest) => <Box p={{active ? 2 : value()}} {{...rest}} />;"
    );
    let actual = evaluate(
        &source,
        "a(true,()=>{trace.push('inactive');return '9px';},{get id(){trace.push('spread');return 'id';}})",
    );
    assert_eq!(actual.trace, serde_json::json!(["spread"]));
    assert!(
        slots(&output(&source))
            .iter()
            .any(|slot| slot.fallback == "8px")
    );
}

#[test]
#[serial]
fn a_throwing_spread_when_props_precede_it_keeps_the_original_trace() {
    let source = format!(
        "{BOX}export const a=() => <Box id={{(trace.push('id'),'id')}} color=\"red\" {{...{{get color(){{trace.push('getter');throw 'boom';}}}}}} title={{(trace.push('title'),'later')}} />;"
    );
    let actual = evaluate(
        &source,
        "(()=>{try{return a();}catch(error){return error;}})()",
    );
    assert_eq!(actual.element, "boom");
    assert_eq!(actual.trace, serde_json::json!(["id", "getter"]));
}

#[test]
#[serial]
fn receiver_lookup_when_props_are_captured_precedes_props_and_children() {
    let source = format!(
        "{BOX}import React from 'react'; export const a=()=>React.createElement(Box,{{color:(trace.push('color'),'red'),...{{get id(){{trace.push('spread');return 'id';}}}}}},(trace.push('child'),'child'));"
    );
    let actual = evaluate(
        &source,
        "(()=>{Object.defineProperty(React,'createElement',{get(){trace.push('callee');return h;}});return a();})()",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["callee", "color", "spread", "child"])
    );
}

#[rstest]
#[case("<Box p={choose() ? 2 : value()} bg={last()} />")]
#[case("jsx(Box,{p:choose() ? 2 : value(),bg:last()})")]
#[case("createElement(Box,{p:choose() ? 2 : value(),bg:last()})")]
#[serial]
fn conditions_when_classes_and_variables_read_them_run_once(#[case] element: &str) {
    let source = format!(
        "{BOX}{JSX_RUNTIME}import {{createElement}} from 'react';export const a=(choose,value,last)=>{element};"
    );
    let actual = evaluate(
        &source,
        "a(()=>{trace.push('test');return false;},()=>{trace.push('branch');return '9px';},()=>{trace.push('last');return 'red';})",
    );
    assert_eq!(actual.trace, serde_json::json!(["test", "branch", "last"]));
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn logical_branches_when_captured_keep_short_circuit_evaluation(#[case] active: bool) {
    let source = format!(
        "{BOX}export const a=(active,load,rest)=><Box p={{active && [2,load()]}} {{...rest}}/>;"
    );
    let actual = evaluate(
        &source,
        &format!(
            "a({active},()=>{{trace.push('branch');return '9px';}},{{get id(){{trace.push('spread');return 'id';}}}})"
        ),
    );
    assert_eq!(
        actual.trace,
        if active {
            serde_json::json!(["branch", "spread"])
        } else {
            serde_json::json!(["spread"])
        }
    );
    assert!(
        slots(&output(&source))
            .iter()
            .any(|slot| slot.fallback == "8px")
    );
}
