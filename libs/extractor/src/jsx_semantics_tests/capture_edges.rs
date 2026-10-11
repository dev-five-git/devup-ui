use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("false && load()", false)]
#[case("true && load()", true)]
#[case("null ?? load()", true)]
#[case("false || load()", true)]
#[serial]
fn literal_logical_when_the_right_side_runs_only_if_selected(
    #[case] value: &str,
    #[case] called: bool,
) {
    let source = format!("{BOX}export const a=(load)=><Box color={{{value}}}/>;");
    let actual = evaluate(&source, "a(()=>{trace.push('load');return 'blue';})");
    assert_eq!(
        actual.trace,
        if called {
            serde_json::json!(["load"])
        } else {
            serde_json::json!([])
        }
    );
}

#[test]
#[serial]
fn responsive_iterator_when_captured_runs_before_later_props_and_keeps_holes() {
    let source = format!(
        "{BOX}export const a=(items,take,later)=><Box p={{[...items,,take()]}} id={{later()}}/>;"
    );
    let actual = evaluate(
        &source,
        "a({[Symbol.iterator](){trace.push('iterator');return [1][Symbol.iterator]();}},()=>{trace.push('take');return '8px';},()=>{trace.push('later');return 'id';})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["iterator", "take", "later"])
    );
}

#[test]
#[serial]
fn child_suspension_when_empty_and_constant_children_precede_it_keeps_them() {
    let source = format!(
        "{BOX}export function* a(rest){{return <Box color='red' {{...rest}}>{{/* empty */}}{{'constant'}}{{yield 'pause'}}</Box>;}}"
    );
    let actual = evaluate(
        &source,
        "(()=>{const iterator=a({});iterator.next();return iterator.next('child').value;})()",
    );
    assert_eq!(
        actual.element["children"],
        serde_json::json!(["constant", "child"])
    );
}

#[test]
#[serial]
fn unrelated_react_call_when_it_receives_a_compile_only_component_is_not_a_factory() {
    let message = error(&format!(
        "{BOX}import * as React from 'react';export const a=React.cloneElement(Box,{{}});"
    ));
    assert!(
        message.starts_with("a.tsx:2:") && message.contains("Box"),
        "{message}"
    );
}
