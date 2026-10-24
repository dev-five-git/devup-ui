use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::nullish("p=>(p.input.maybe??false)&&{color:'red'}", "let n=0;const input={get maybe(){trace.push('maybe');return ++n===1?undefined:true;}};", "{input}", serde_json::json!([["maybe"],["maybe","maybe"],["blue"],["red"]]))]
#[case::discarded("p=>({color:p.input.before})&&{color:p.input.after}", "const input={get before(){trace.push('before');return 'orange';},get after(){trace.push('after');return 'red';}};", "{input}", serde_json::json!([["before","after"],["before","after","before","after"],["red"],["red"]]))]
#[case::or_unreachable("p=>({color:p.input.before})||opaque()", "const input={get before(){trace.push('before');return 'red';}};", "{input}", serde_json::json!([["before"],["before","before"],["red"],["red"]]))]
#[case::and_unreachable("p=>false&&opaque()", "", "{}", serde_json::json!([[],[],["blue"],["blue"]]))]
#[case::coalesce_unreachable("p=>({color:p.input.before})??opaque()", "const input={get before(){trace.push('before');return 'red';}};", "{input}", serde_json::json!([["before"],["before","before"],["red"],["red"]]))]
#[case::normal("function rules<T>(p:T){'use strict';return p.on?{color:this===undefined&&arguments.length===1&&arguments[0]===p?p.color:'wrong'}:{};}", "", "{on:true,color:'red'}", serde_json::json!([[],[],["red"],["red"]]))]
#[case::function_leaf("({on,token=(trace.push('default'),'red')})=>on?{color:function(q){'use strict';return this===undefined&&arguments.length===1&&q.on?token:'wrong';},margin:q=>q.margin}:{}", "", "{on:true,margin:2}", serde_json::json!([["default"],["default","default"],["red"],["red"]]))]
#[case::known_mixin("p=>p.on?mixin:null", "const mixin=css({color:'red'});", "{on:true}", serde_json::json!([[],[],["red"],["red"]]))]
#[case::empty_mixin("p=>mixin||{color:'red'}", "const mixin=css({});", "{}", serde_json::json!([[],[],["red"],["red"]]))]
#[case::global_undefined("p=>undefined??{color:'red'}", "", "{}", serde_json::json!([[],[],["red"],["red"]]))]
#[serial]
fn original_callback_runs_once_when_call_rules_select_branches(
    #[case] callback: &str,
    #[case] setup: &str,
    #[case] props: &str,
    #[case] expected: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: the actual callback observes getters or its original invocation scope.
    let source = format!(
        "const trace=[];{setup}const Base=styled.div({{color:'blue'}});const Card=styled(Base)({callback});const defined=trace.slice();"
    );
    // When: two complete generated renders execute the authored short circuit.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        &format!(
            "const first=render({{target:Card,props:{props}}});const firstTrace=trace.slice();const second=render({{target:Card,props:{props}}});JSON.stringify([defined,[firstTrace,trace,values(first,'color'),values(second,'color')]]);"
        ),
        single_css,
    );
    // Then: evaluated operands run once, unreachable operands never run, and missing rules Keep.
    assert_eq!(observed, serde_json::json!([[], expected]));
}

#[rstest]
#[serial]
fn selected_function_leaf_keeps_ordinary_numbers_when_outer_rule_values_use_px(
    #[values(false, true)] single_css: bool,
) {
    // Given: a selected closure shares one default binding with a dynamic whole-rule length.
    let source = "const trace=[];const Card=styled.div(({on,token=(trace.push('default'),2)})=>on?{margin:q=>token,width:token}:{});";
    // When: both selected values execute in the complete generated render.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{ on: true }});JSON.stringify([trace,values(element,'margin'),values(element,'width')]);",
        single_css,
    );
    // Then: property callbacks retain ordinary numbers and dynamic whole-rule lengths retain px.
    assert_eq!(observed, serde_json::json!([["default"], [2], ["2px"]]));
}

#[rstest]
#[serial]
fn selected_callback_keeps_order_when_arrays_objects_and_opaque_classes_interleave(
    #[values(false, true)] single_css: bool,
) {
    // Given: covered leaves and inactive branches share one authored argument schedule.
    let source = "const trace=[];const data={get before(){trace.push('before');return 'orange';},get opaque(){trace.push('class');return 'opaque';},get on(){trace.push('on');return true;},get after(){trace.push('after');return 'green';}};const make=input=>styled.div({color:input.before},(p,unused=(trace.push('default'),0))=>p.on?{color:(trace.push('leaf'),p.color)}:{color:(trace.push('inactive'),'wrong')},input.opaque,input.on?[{margin:2}]:[],{color:input.after});const Card=make(data);const defined=trace.slice();";
    // When: a selected callback is covered by the later ordinary object.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{on:true,color:'red'}});JSON.stringify([defined,trace,values(element,'color'),values(element,'margin'),element.props.className.split(/\\s+/).includes('opaque')]);",
        single_css,
    );
    // Then: D1 composition changes declarations, never the execution schedule.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["before", "default", "leaf", "class", "on", "after"],
            ["green"],
            ["8px"],
            true
        ])
    );
}

#[rstest]
#[serial]
fn later_shorthand_wins_when_callback_object_has_competing_longhand(
    #[values("p", "props")] parameter: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: the later shorthand targets the same property only in the plain selector group.
    let source = format!(
        "const Card=styled.div({parameter}=>({{padding:3,p:4,_hover:{{padding:3}},'@layer':{{base:{{padding:3}}}}}}));"
    );
    // When: the generated renderer selects the authored callback object's declarations.
    let observed = execute_in_mode(
        "callback-alias-order.ts",
        &source,
        "const element=render({target:Card,props:{}});JSON.stringify([atoms(element).filter(r=>r[0]==='padding'&&r[3]===''&&r[4]===null).map(r=>r[1]),values(element,'padding',0,'&:hover'),atoms(element).filter(r=>r[0]==='padding'&&r[4]==='base').map(r=>r[1])]);",
        single_css,
    );
    // Then: the sheet orders the retained plain 12px before 16px; hover and layer stay distinct.
    assert_eq!(
        observed,
        serde_json::json!([["12px", "16px"], ["12px"], ["12px"]])
    );
}

#[rstest]
#[case::inherited(
    "const Base=styled.div({ padding: 2 });const Card=styled(Base)(p=>({ padding: 3,p:4 }));",
    &["16px"]
)]
#[case::parts("const Card=styled.div({ padding: 2 },p=>({ padding: 3,p:4 }));", &["16px"])]
#[case::reverse_parts("const Card=styled.div(p=>({padding:3,p:4}),{padding:2});", &["8px"])]
#[case::known(
    "const mixin=css({ padding: 2 });const Card=styled.div(mixin,p=>({ padding: 3,p:4 }));",
    &["16px"]
)]
#[case::known_choice(
    "const mixin=css({ padding: 2 });const Card=styled.div(p=>p.on?mixin:{ padding: 3,p:4 });",
    &["16px"]
)]
#[serial]
fn later_alias_keeps_d1_when_callback_rules_compose(
    #[case] source: &str,
    #[case] expected: &[&str],
    #[values(false, true)] single_css: bool,
) {
    // Given: competing aliases enter through inheritance, written parts or semantic mixins.
    // When: the complete renderer selects the callback's object branch once.
    let observed = execute_in_mode(
        "callback-alias-composition.ts",
        source,
        "const element=render({target:Card,props:{ on: false }});JSON.stringify(values(element,'padding'));",
        single_css,
    );
    // Then: later parts win by canonical property, including the object's later shorthand.
    assert_eq!(observed, serde_json::json!(expected));
}
