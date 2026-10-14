use super::support::{compiled, selected};
use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[path = "styled_creation_array_tests.rs"]
mod typography_arrays;

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn selected_argument_reads_once_in_source_order_when_rendered_twice(#[case] choice: bool) {
    // Given: distinct reads in opposite branch orders, with a mutable controller.
    let rules = "choice ? {width:values.first,height:values.second} : {height:values.second,width:values.first}";
    let setup = format!(
        r"
const trace=[]; const state={{choice:{choice},size:0}};
Object.defineProperty(globalThis,'choice',{{get(){{trace.push('choice');return state.choice}}}});
const values={{get first(){{trace.push('first');return `${{++state.size}}px`}},
get second(){{trace.push('second');return `${{++state.size}}px`}}}};
"
    );
    let expected = evaluate(&format!(
        r"{setup}
const rules={rules}; const created=trace.slice();state.choice=!state.choice;
JSON.stringify([created,trace.slice(),trace.slice(),Object.values(rules).sort()]);"
    ));
    // When: the generated declaration and two renders run in the real JS evaluator.
    let actual = evaluate(&format!(
        r"{setup}
{} const created=trace.slice();state.choice=!state.choice;
const first=A({{}});const rendered=trace.slice();A({{}});
JSON.stringify([created,rendered,trace.slice(),Object.values(first.style).sort()]);",
        compiled(rules)
    ));
    // Then: creation order and distinct read values match the authored argument.
    assert_eq!(actual, expected);
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn unselected_argument_stays_lazy_when_other_branch_throws(#[case] choice: bool) {
    // Given: precisely the unselected getter throws.
    let setup = format!(
        r"const trace=[];const state={{choice:{choice}}};
Object.defineProperty(globalThis,'choice',{{get(){{trace.push('choice');return state.choice}}}});
const values={{get yes(){{if(!state.choice)throw Error('unselected');trace.push('yes');return '13px'}},
get no(){{if(state.choice)throw Error('unselected');trace.push('no');return '17px'}}}};"
    );
    let rules = "choice?{width:values.yes}:{width:values.no}";
    let expected = evaluate(&format!(
        r"{setup} const rules={rules};JSON.stringify([trace,trace,trace]);"
    ));
    // When: the selected component renders repeatedly.
    let actual = evaluate(&format!(
        r"{setup} {} const created=trace.slice();A({{}});const first=trace.slice();A({{}});
JSON.stringify([created,first,trace]);",
        compiled(rules)
    ));
    // Then: only the selected creation branch is evaluated.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn selected_argument_throws_at_creation_when_first_field_fails() {
    // Given: the chosen branch fails before its later field or the other branch.
    let rules = "choice?{width:values.fail,height:values.later}:{width:values.other}";
    let setup = r"const trace=[];Object.defineProperty(globalThis,'choice',{get(){trace.push('choice');return true}});
const values={get fail(){trace.push('fail');throw Error('chosen')},get later(){trace.push('later');return '1px'},
get other(){trace.push('other');return '2px'}};";
    let expected = evaluate(&format!(
        r"{setup}let error;try{{const rules={rules}}}catch(caught){{error=caught.message}}
JSON.stringify([trace,error]);"
    ));
    // When: the declaration executes without a render.
    let actual = evaluate(&format!(
        r"{setup}let error;try{{{}}}catch(caught){{error=caught.message}}
JSON.stringify([trace,error]);",
        compiled(rules)
    ));
    // Then: the same error interrupts the same source prefix.
    assert_eq!(actual, expected);
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn packet_keeps_dynamic_and_typography_associated_when_state_changes(#[case] choice: bool) {
    // Given: two branches with different static, dynamic and typography selections.
    let rules = "state.cond?{color:'red',width:state.w,typography:state.typo}:{color:'blue',width:state.other,typography:state.body}";
    let setup =
        format!("const state={{cond:{choice},w:'13px',other:'17px',typo:'heading',body:'body'}};");
    let (color, typography, width) = if choice {
        ("red", "heading", "13px")
    } else {
        ("blue", "body", "17px")
    };
    let expected = serde_json::json!([
        ["color", color, 0, null],
        ["typography", typography, 0, null],
        ["width", width, 0, null]
    ]);
    // When: actual class records and inline values are projected after repeated renders.
    let actual: serde_json::Value = serde_json::from_str(&selected(rules, &setup))
        .unwrap_or_else(|error| panic!("projected selections must be JSON: {error}"));
    // Then: creation selection, not later state, owns both class and variable consumers.
    assert_eq!(actual, serde_json::json!([expected.clone(), expected]));
}

#[rstest]
#[case("flag && {width:values.width}", true)]
#[case("flag && {width:values.width}", false)]
#[case("flag || {width:values.width}", false)]
#[case("flag ?? {width:values.width}", false)]
#[serial]
fn logical_argument_keeps_creation_reads_when_rendered_twice(
    #[case] rules: &str,
    #[case] flag: bool,
) {
    // Given: a supported logical rule composition and mutable controller.
    let setup = format!(
        r"const trace=[];const state={{flag:{flag}}};
Object.defineProperty(globalThis,'flag',{{get(){{trace.push('flag');return state.flag}}}});
const values={{get width(){{trace.push('width');return '13px'}}}};"
    );
    let expected = evaluate(&format!(
        r"{setup}const rules={rules};JSON.stringify([trace,trace,trace]);"
    ));
    // When: creation is followed by a changed controller and two renders.
    let actual = evaluate(&format!(
        r"{setup}{}const created=trace.slice();state.flag=!state.flag;
A({{}});const first=trace.slice();A({{}});JSON.stringify([created,first,trace]);",
        compiled(rules)
    ));
    // Then: source short-circuiting and read counts remain creation-owned.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn supported_member_class_is_bound_at_creation_when_getter_changes() {
    // Given: a computed member selects an authored class composition.
    let source = r"import {styled,css}from '@devup-ui/react';let reads=0;
const classes={a:css({color:'red'}),b:css({color:'blue'})};
const state={get key(){reads++;return reads===1?'a':'b'}};
const A=styled.div(classes[state.key]);const created=reads;
const first=A({}),second=A({});JSON.stringify([created,reads,first.className===classes.a,second.className===classes.a]);";
    // When: the generated class-composing component renders twice.
    let actual = evaluate(&compiled_jsx(source));
    // Then: the member lookup happens once and its class remains selected.
    assert_eq!(actual, "[1,1,true,true]");
}

#[test]
#[serial]
fn attrs_wrapper_keeps_rules_created_once_when_callback_runs_per_render() {
    // Given: a rule getter at creation, and an authored attrs callback at render time.
    let source = r"import {styled}from '@devup-ui/react';const trace=[];
const values={get width(){trace.push('width');return '13px'}};
const A=styled.div.attrs(props=>{trace.push(props.id);return {title:props.id}})({width:values.width});
const created=trace.slice();const first=A({id:'first'}),second=A({id:'second'});
JSON.stringify([created,trace,first.title,second.title,Object.values(first.style),Object.values(second.style)]);";
    // When: the real generated attrs component is created and rendered twice.
    let actual = evaluate(&compiled_jsx(source));
    // Then: attrs callbacks stay per-render without pulling rules into rendering.
    assert_eq!(
        actual,
        r#"[["width"],["width","first","second"],"first","second",["13px"],["13px"]]"#
    );
}

#[test]
#[serial]
fn css_control_evaluates_each_call_when_function_is_called_twice() {
    // Given: css is authored inside a function, unlike a creation-owned styled rule.
    let source = r"import {css}from '@devup-ui/react';let reads=0;
Object.defineProperty(globalThis,'flag',{get(){reads++;return reads===1}});
const render=()=>css(flag?{color:'red'}:{color:'blue'});const created=reads;
const first=render(),second=render();JSON.stringify([created,reads,first!==second]);";
    // When: two authored calls execute the actual extracted class expressions.
    let actual = evaluate(&compiled_jsx(source));
    // Then: ordinary css call ownership is not hoisted out of its function.
    assert_eq!(actual, "[0,2,true]");
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn static_typography_remains_in_selected_packet_when_branch_changes(#[case] choice: bool) {
    // Given: literal typography is selected alongside a static CSS declaration.
    let rules = "state.cond?{typography:'heading',color:'red'}:{typography:'body',color:'blue'}";
    let setup = format!("const state={{cond:{choice}}};");
    let (color, typography) = if choice {
        ("red", "heading")
    } else {
        ("blue", "body")
    };
    let expected = serde_json::json!([
        ["color", color, 0, null],
        ["typography", typography, 0, null]
    ]);
    // When: selection is observed on the emitted component before and after mutation.
    let actual: serde_json::Value = serde_json::from_str(&selected(rules, &setup))
        .unwrap_or_else(|error| panic!("projected selections must be JSON: {error}"));
    // Then: both selected declarations stay attached to the creation-owned packet.
    assert_eq!(actual, serde_json::json!([expected.clone(), expected]));
}

#[test]
#[serial]
fn base_and_selected_rules_keep_source_order_when_attrs_wrap_rendering() {
    // Given: a runtime base precedes the conditional argument; attrs execute on rendering.
    let source = r"import {styled}from '@devup-ui/react';const trace=[];
const pick=()=>{trace.push('base');return 'section'};
Object.defineProperty(globalThis,'flag',{get(){trace.push('flag');return false}});
const values={get width(){trace.push('width');return '13px'},get other(){throw Error('unselected')}};
const A=styled(pick()).attrs(props=>{trace.push(props.id);return {title:props.id}})(flag?{width:values.other}:{width:values.width});
const created=trace.slice();A({id:'first'});A({id:'second'});JSON.stringify([created,trace]);";
    // When: the actual base/attrs/creation wrappers are executed together.
    let actual = evaluate(&compiled_jsx(source));
    // Then: base and chosen rule reads happen once, before render-time callbacks.
    assert_eq!(
        actual,
        r#"[["base","flag","width"],["base","flag","width","first","second"]]"#
    );
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn selected_packet_retains_nested_static_rules_when_argument_is_conditional(#[case] choice: bool) {
    // Given: selector and layer declarations share a branch with a dynamic width.
    let rules = "state.cond?{_hover:{color:'red'},'@layer':{base:{p:1}},width:state.w}:{_hover:{color:'blue'},'@layer':{base:{p:2}},width:state.other}";
    let setup = format!("const state={{cond:{choice},w:'13px',other:'17px'}};");
    let (color, padding, width) = if choice {
        ("red", "4px", "13px")
    } else {
        ("blue", "8px", "17px")
    };
    let expected = serde_json::json!([
        ["color", color, 0, null],
        ["padding", padding, 0, "base"],
        ["width", width, 0, null]
    ]);
    // When: emitted declarations are observed through two actual renders.
    let actual: serde_json::Value = serde_json::from_str(&selected(rules, &setup))
        .unwrap_or_else(|error| panic!("projected selections must be JSON: {error}"));
    // Then: packet ownership preserves nested declarations as well as inline values.
    assert_eq!(actual, serde_json::json!([expected.clone(), expected]));
}
