use super::*;
use serial_test::serial;

#[test]
#[serial]
fn literal_callbacks_when_values_surround_order_run_in_render_source_order() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const flag=(name)=>(trace.push(name),true); const Card=styled.div`color:${p=>flag('first')?'red':'blue'};style-order:${p=>flag('order')?2:3};background:${p=>flag('last')?'black':'white'};`; const a=Card({},null);";
    let result = whole::evaluate(source, "a.props.className");
    assert_eq!(result.trace, serde_json::json!(["first", "order", "last"]));
    assert!(
        result
            .element
            .as_str()
            .required("literal callbacks must emit classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn literal_callbacks_when_nested_order_has_early_return_keeps_closure() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const limit=2; const Card=styled.div`&:hover{style-order:${function(p){trace.push(p.active);if(p.active)return limit;return 3}};color:red;}`; const a=Card({active:true},null); const b=Card({active:false},null);";
    let result = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(result.trace, serde_json::json!([true, false]));
    assert!(
        result.element[0]
            .as_str()
            .required("active nested callback must emit classes")
            .contains("-2-")
    );
    assert!(
        result.element[1]
            .as_str()
            .required("inactive nested callback must emit classes")
            .contains("-3-")
    );
}

#[test]
#[serial]
fn literal_callbacks_when_mixin_is_function_keeps_root_order() {
    let source = "import {styled,css} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const flag=(value)=>(trace.push(value),value); const Card=styled.div`color:red;${p=>css({backgroundColor:flag(p.active)?'blue':'green'})};style-order:2;`; const a=Card({active:true},null); const b=Card({active:false},null);";
    let result = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(result.trace, serde_json::json!([true, false]));
    assert!(
        result.element[0]
            .as_str()
            .required("active mixin must emit classes")
            .contains("blue--2-"),
        "{}\n{}",
        result.element,
        code(source)
    );
    assert!(
        result.element[1]
            .as_str()
            .required("inactive mixin must emit classes")
            .contains("green--2-")
    );
}

#[test]
#[serial]
fn literal_order_when_nested_text_is_lazy_captures_only_selected_branch() {
    let source = "import {css} from '@devup-ui/react'; const flag=(name,value)=>(trace.push(name),value); const a=css({_hover:flag('outer',false)?`style-order:${flag('dead',true)?2:3};color:red`:`style-order:${flag('live',true)?4:5};color:blue`});";
    let result = whole::evaluate(source, "a");
    assert_eq!(result.trace, serde_json::json!(["outer", "live"]));
    assert!(
        result
            .element
            .as_str()
            .required("selected lazy text must emit classes")
            .contains("blue-_a__c_hover-4-")
    );
}

#[test]
#[serial]
fn literal_order_when_raw_jsx_factory_takes_css_captures_at_source_once() {
    let source = "/** @jsxImportSource @emotion/react */\nimport {jsx} from '@emotion/react'; const config={get active(){trace.push('css');return true}}; const a=jsx('div',{id:(trace.push('id'),'a'),css:`style-order:${config.active?2:3};color:red`,title:(trace.push('title'),'b')});";
    let compiled = compile_emotion(source)
        .required("Emotion literal css factory must compile")
        .code;
    let result = whole::evaluate_code(&compiled, "a.props.className");
    assert_eq!(
        result.trace,
        serde_json::json!(["id", "css", "title"]),
        "{compiled}"
    );
    assert!(
        result
            .element
            .as_str()
            .required("Emotion factory must emit classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn literal_support_when_plain_object_order_is_function_stays_invalid() {
    let source = "import {styled} from '@devup-ui/react'; const Card=styled.div({styleOrder:p=>p.active?2:3,color:'red'});";
    assert!(
        compile(source)
            .required_err("plain object order function must stay invalid")
            .contains("styleOrder")
    );
}
