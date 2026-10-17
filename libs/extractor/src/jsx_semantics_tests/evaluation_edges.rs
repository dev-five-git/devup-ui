use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("active || [2,load()]", "false", true)]
#[case("active || [2,load()]", "'12px'", false)]
#[case("active ?? [2,load()]", "null", true)]
#[case("active ?? [2,load()]", "0", false)]
#[serial]
fn lazy_logical_arrays_when_captured_evaluate_only_the_selected_branch(
    #[case] value: &str,
    #[case] active: &str,
    #[case] loads: bool,
) {
    let source =
        format!("{BOX}export const a=(active,load,rest)=><Box p={{{value}}} {{...rest}}/>;");
    let actual = evaluate(
        &source,
        &format!(
            "a({active},()=>{{trace.push('branch');return '4px';}},{{get id(){{trace.push('spread');return 'id';}}}})"
        ),
    );
    assert_eq!(
        actual.trace,
        if loads {
            serde_json::json!(["branch", "spread"])
        } else {
            serde_json::json!(["spread"])
        }
    );
}

#[test]
#[serial]
fn literal_spread_with_effectful_keys_when_followed_by_an_unknown_spread_keeps_effects() {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=(load,rest)=>jsx(Box,{{...{{color:load(),id:load()}},...rest}});"
    );
    let actual = evaluate(
        &source,
        "a(()=>{trace.push('load');return 'red';},{color:'blue'})",
    );
    assert_eq!(actual.trace, serde_json::json!(["load", "load"]));
    assert_eq!(actual.element["props"]["id"], "red");
}

#[test]
#[serial]
fn suspending_children_when_wrapped_leave_await_and_yield_in_the_callers_scope() {
    for declaration in [
        "async function a(load){return <Box color={load()} id={load()}>{await load()}<span>{load()}</span></Box>;}",
        "function* a(load){return <Box color={load()} id={load()}>{yield load()}<span>{load()}</span></Box>;}",
    ] {
        let rendered = code(&format!("{BOX}export {declaration}"));
        assert!(rendered.contains("load()"));
        assert!(rendered.contains("__devupValue"));
    }
}

#[test]
#[serial]
fn trailing_children_spread_when_dynamic_as_is_captured_evaluates_the_iterable_once() {
    let source = format!(
        "{BOX}import {{createElement}} from 'react';export const a=(type,rest,kids)=>createElement(Box,{{as:type(),color:'red',...rest}},...kids());"
    );
    let actual = evaluate(
        &source,
        "a(()=>{trace.push('type');return 'section';},{get id(){trace.push('spread');return 'id';}},()=>{trace.push('children');return ['a','b'];})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["type", "spread", "children"])
    );
    assert_eq!(actual.element["children"], serde_json::json!(["a", "b"]));
}

#[test]
#[serial]
fn duplicate_styles_when_a_throwing_earlier_read_is_discarded_still_throw() {
    let source = format!("{BOX}export const a=(bad)=><Box color={{bad}} color=\"red\"/>;");
    assert!(code(&source).contains("bad"));
    let actual = evaluate(
        &source,
        "(()=>{try{return a(missing);}catch(error){return error.name;}})()",
    );
    assert_eq!(actual.element, "ReferenceError");
}
