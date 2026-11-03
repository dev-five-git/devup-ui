use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::ordinary("const trace=[];const callback=function(p,unused=(trace.push('default'),0)){trace.push(this===globalThis?'global':'receiver',arguments.length);return p.value;};const Base=styled.div({p:2});const Card=styled(Base)({p:callback});", serde_json::json!(["default", "global", 1]))]
#[case::whole("const trace=[];const callback=(p,unused=(trace.push('default'),0))=>({p:p.value});const Base=styled.div({p:2});const Card=styled(Base)(callback);", serde_json::json!(["default"]))]
#[serial]
fn original_invocation_survives_when_callback_is_bound(
    #[case] source: &str,
    #[case] trace: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: the exact bound witnesses, including their original extra default.
    // When: one complete generated renderer receives exactly one props object.
    let actual = execute_in_mode(
        "binding.ts",
        &format!("{source}const observe=()=>trace;"),
        "const e=render({target:Card,props:{ value: 4 }});JSON.stringify([observe(),values(e,'padding')]);",
        single_css,
    );
    // Then: original defaults/receiver/arguments execute once and the native value scales.
    assert_eq!(actual, serde_json::json!([trace, ["16px"]]));
}

#[rstest]
#[case::number("4", serde_json::json!(["16px"]))]
#[case::negative("-2", serde_json::json!(["-8px"]))]
#[case::zero("0", serde_json::json!([0]))]
#[case::string("'4'", serde_json::json!(["4"]))]
#[case::pixels("'4px'", serde_json::json!(["4px"]))]
#[case::keyword("'auto'", serde_json::json!(["auto"]))]
#[case::null("null", serde_json::json!(["8px"]))]
#[case::undefined("undefined", serde_json::json!(["8px"]))]
#[case::false_value("false", serde_json::json!(["8px"]))]
#[serial]
fn scale_and_keep_survive_when_bound_callbacks_return_application_values(
    #[case] value: &str,
    #[case] expected: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = "const ordinary=p=>p.value;const whole=p=>({p:p.value});const Base=styled.div({p:2});const Card=styled(Base)({p:ordinary});const Whole=styled(Base)(whole);";
    // When
    let actual = execute_in_mode(
        "binding-values.ts",
        source,
        &format!(
            "JSON.stringify([Card,Whole].map(target=>values(render({{target,props:{{value:{value}}}}}),'padding')));"
        ),
        single_css,
    );
    // Then
    assert_eq!(actual, serde_json::json!([expected, expected]));
}

#[rstest]
#[serial]
fn authored_schedule_survives_when_bound_ordinary_value_is_fully_covered(
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = "const trace=[];let count=0;const input={get before(){trace.push('before');return 'red';},get on(){trace.push('on');return true;},get value(){trace.push('value'+(++count));return 4;},get after(){trace.push('after');return 'blue';}};const callback=function(p,unused=(trace.push('default'),0)){trace.push(this===globalThis?'global':'receiver',arguments.length);return p.input.on&&p.input.value;};const Card=styled.div({color:input.before,p:callback,color:input.after},p=>({opacity:(trace.push('whole'),p.opacity)}),{p:3});const observe=()=>trace;";
    // When
    let actual = execute_in_mode(
        "binding-order.ts",
        source,
        "const e=render({target:Card,props:{input,opacity:0.5}});JSON.stringify([observe(),values(e,'padding'),values(e,'opacity')]);",
        single_css,
    );
    // Then
    assert_eq!(
        actual,
        serde_json::json!([
            [
                "before", "default", "global", 1, "on", "value1", "after", "whole"
            ],
            ["12px"],
            [0.5]
        ])
    );
}

#[rstest]
#[serial]
fn lexical_binding_survives_when_same_spelling_is_shadowed(
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = "const callback=p=>'wrong';function make(){const callback=(p,unused='kept')=>p.value;return styled.div({ p: callback });}const Card=make();";
    // When
    let actual = execute_in_mode(
        "binding-shadow.ts",
        source,
        "JSON.stringify(values(render({target:Card,props:{ value: 4 }}),'padding'));",
        single_css,
    );
    // Then
    assert_eq!(actual, serde_json::json!(["16px"]));
}

#[rstest]
#[case::unknown(
    "function make(callback){return styled.div({ p: callback });}const Card=make(function(){trace.push('called');return 4;});"
)]
#[case::alias(
    "const callback=p=>{trace.push('called');return p.value;};const alias=callback;const Card=styled.div({p:alias});"
)]
#[serial]
fn unproven_callable_stays_a_value_when_existing_recovery_cannot_prove_source(
    #[case] source: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = format!("const trace=[];{source}const observe=()=>trace;");
    // When
    let actual = execute_in_mode(
        "binding-unknown.ts",
        &source,
        "const e=render({target:Card,props:{ value: 4 }});JSON.stringify([typeof values(e,'padding')[0],observe()]);",
        single_css,
    );
    // Then
    assert_eq!(actual, serde_json::json!(["function", []]));
}

#[rstest]
#[serial]
fn numeric_origins_survive_when_bound_callbacks_use_longhand_properties(
    #[values(false, true)] single_css: bool,
) {
    // Given
    let source = "const ordinary=p=>p.value;const whole=p=>({padding:p.value});const Card=styled.div({padding:ordinary});const Whole=styled.div(whole);";
    // When
    let actual = execute_in_mode(
        "binding-units.ts",
        source,
        "JSON.stringify([Card,Whole].map(target=>values(render({target,props:{ value: 4 }}),'padding')));",
        single_css,
    );
    // Then
    assert_eq!(actual, serde_json::json!([[4], ["4px"]]));
}
