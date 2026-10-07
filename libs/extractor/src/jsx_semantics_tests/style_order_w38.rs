use super::*;
use serial_test::serial;

mod captured;
mod contexts;
mod coverage;

fn orders(source: &str) -> Vec<(String, u8, Option<u8>)> {
    let mut result: Vec<_> = output(source)
        .styles
        .into_iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((
                style.property().to_string(),
                style.level(),
                style.style_order(),
            )),
            _ => None,
        })
        .collect();
    result.sort();
    result
}

#[test]
#[serial]
fn strict_table_when_used_in_jsx_css_and_styled_has_the_same_result() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../test-fixtures/style-order-w38.json"
    ))
    .required("strict-order fixture must contain valid JSON");
    for case in cases
        .as_array()
        .required("strict-order fixture must be an array")
    {
        let expression = case["expression"]
            .as_str()
            .required("strict-order case must contain an expression string");
        let valid = case["valid"]
            .as_bool()
            .required("strict-order case must contain a validity boolean");
        for usage in [
            format!("<Box styleOrder={{{expression}}} color='red' />"),
            format!("jsx(Box,{{ styleOrder: {expression}, color: 'red' }})"),
            format!("css({{ styleOrder: {expression}, color: 'red' }})"),
            format!("styled('div',{{ styleOrder: {expression}, color: 'red' }})"),
        ] {
            let source = format!(
                "{BOX}{JSX_RUNTIME}import {{css,styled}} from '@devup-ui/react'; export const a=(on)=>{usage};"
            );
            assert_eq!(compile(&source).is_ok(), valid, "{expression}: {usage}");
        }
    }
}

#[test]
#[serial]
fn css_when_order_conditions_have_effects_evaluates_in_key_order_once() {
    let source = "import {css} from '@devup-ui/react'; const flag=(name,value)=>(trace.push(name),value); const config={get active(){trace.push('get');return true}}; const a=css({_hover:flag('first',true)?{color:'blue'}:{color:'red'},styleOrder:config.active?2:3,_focus:flag('last',true)?{color:'green'}:{color:'black'}});";
    let actual = whole::evaluate(source, "a");
    assert_eq!(actual.trace, serde_json::json!(["first", "get", "last"]));
    let mut classes: Vec<_> = actual
        .element
        .as_str()
        .required("CSS order selection must emit a class string")
        .split_whitespace()
        .collect();
    classes.sort_unstable();
    assert_eq!(
        classes,
        vec![
            "color-0-blue-_a__c_hover-2-a",
            "color-0-green-_a__c_focus-2-a"
        ]
    );
}

#[test]
#[serial]
fn css_when_an_order_only_or_equal_branch_collapses_keeps_the_test() {
    let source = "import {css} from '@devup-ui/react'; const flag=(name)=>(trace.push(name),true); const a=css({styleOrder:flag('empty')?2:3}); const b=css({styleOrder:flag('same')?2:2,color:'red'});";
    let actual = whole::evaluate(source, "[a,b]");
    assert_eq!(actual.trace, serde_json::json!(["empty", "same"]));
    assert_eq!(actual.element[0], "");
}

#[test]
#[serial]
fn css_when_whole_objects_are_lazy_does_not_evaluate_the_other_branch() {
    let source = "import {css} from '@devup-ui/react'; const flag=(name,value)=>(trace.push(name),value); const a=css(flag('outer',false)?{styleOrder:flag('dead',true)?2:3,color:'red'}:{styleOrder:flag('live',true)&&4,color:'blue'},{styleOrder:flag('next',true)?5:6,backgroundColor:'black'});";
    let actual = whole::evaluate(source, "a");
    assert_eq!(actual.trace, serde_json::json!(["outer", "live", "next"]));
    let classes = actual
        .element
        .as_str()
        .required("lazy CSS objects must emit a class string");
    assert!(
        classes.contains("--4-") && classes.contains("--5-"),
        "{classes}"
    );
}

#[test]
#[serial]
fn styled_when_object_order_is_dynamic_captures_at_construction_not_render() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; let active=true; const config={get active(){trace.push('construct');return active}}; const Card=styled('div',{styleOrder:config.active?2:3,color:'red'}); active=false; const a=Card({},null); const b=Card({},null);";
    let actual = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(actual.trace, serde_json::json!(["construct"]));
    assert_eq!(actual.element[0], actual.element[1]);
    assert!(
        actual.element[0]
            .as_str()
            .required("constructed styled component must emit classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn styled_when_callback_order_reads_props_selects_at_each_render() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const flag=(value)=>(trace.push(value),value); const Card=styled('div',props=>({styleOrder:flag(props.active)?2:3,color:'red'})); const a=Card({active:true},null); const b=Card({active:false},null);";
    let actual = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(actual.trace, serde_json::json!([true, false]));
    assert!(
        actual.element[0]
            .as_str()
            .required("active styled callback must emit classes")
            .contains("--2-")
    );
    assert!(
        actual.element[1]
            .as_str()
            .required("inactive styled callback must emit classes")
            .contains("--3-")
    );
}

#[test]
#[serial]
fn nested_orders_when_responsive_objects_override_the_parent_remain_metadata() {
    for api in ["css", "styled"] {
        let rules = "{styleOrder:2,color:['red','blue'],_hover:[{styleOrder:3,color:'green'},{styleOrder:4,color:'black'}],selectors:{'& > p':{styleOrder:5,color:'purple'}}}";
        let call = if api == "styled" {
            format!("styled('div',{rules})")
        } else {
            format!("css({rules})")
        };
        let actual = orders(&format!(
            "import {{css,styled}} from '@devup-ui/react'; const a={call};"
        ));
        assert_eq!(
            actual,
            vec![
                ("color".to_string(), 0, Some(2)),
                ("color".to_string(), 0, Some(3)),
                ("color".to_string(), 0, Some(5)),
                ("color".to_string(), 1, Some(2)),
                ("color".to_string(), 1, Some(4))
            ]
        );
    }
}
