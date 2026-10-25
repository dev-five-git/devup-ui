use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

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
fn native_scale_and_keep_survive_when_ordinary_callback_returns_application_value(
    #[case] value: &str,
    #[case] expected: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: ordinary and accepted whole-rule callbacks read the same value.
    let source = "const Base=styled.div({ p: 2 });const Card=styled(Base)({ p: p=>p.value });const Whole=styled(Base)(p=>({ p: p.value }));";
    // When: both complete generated renderers receive one application input.
    let actual = execute_in_mode(
        "ordinary-value.ts",
        source,
        &format!(
            "JSON.stringify([Card,Whole].map(target=>values(render({{target,props:{{value:{value}}}}}),'padding')));"
        ),
        single_css,
    );
    // Then: the ordinary result preserves the accepted units, representation and Keep.
    assert_eq!(actual, serde_json::json!([expected, expected]));
}

#[rstest]
#[case::known(
    "const mixin=css({ p: 2 });const Card=styled.div(mixin,{ p: p=>p.value });",
    0,
    ""
)]
#[case::earlier("const Card=styled.div({ p: 2 },{ p: p=>p.value });", 0, "")]
#[case::layer(
    "const Base=styled.div({'@layer':{base:{p:2}}});const Card=styled(Base)({'@layer':{base:{p:p=>p.value}}});",
    0,
    ""
)]
#[case::hover(
    "const Base=styled.div({_hover:{p:2}});const Card=styled(Base)({_hover:{p:p=>p.value}});",
    0,
    "&:hover"
)]
#[serial]
fn same_key_keeps_predecessor_when_ordinary_result_is_missing(
    #[case] source: &str,
    #[case] level: u8,
    #[case] selector: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: a distinguishable predecessor in the same composed key.
    // When: the real renderer exercises present and missing callback results.
    let actual = execute_in_mode(
        "ordinary-context.ts",
        source,
        &format!(
            "JSON.stringify([4,null,undefined,false].map(value=>values(render({{target:Card,props:{{value}}}}),'padding',{level},'{selector}')));"
        ),
        single_css,
    );
    // Then: only a present result replaces that key.
    assert_eq!(
        actual,
        serde_json::json!([["16px"], ["8px"], ["8px"], ["8px"]])
    );
}

#[rstest]
#[serial]
fn authored_effects_stay_ordered_when_mixed_arguments_cover_ordinary_leaf(
    #[values(false, true)] single_css: bool,
) {
    // Given: changing getters around normal/default callbacks and a later covering part.
    let source = "const trace=[];let count=0;const input={get value(){trace.push('value'+(++count));return count===1?4:null;},get before(){trace.push('before');return 'red';},get after(){trace.push('after');return 'blue';}};const Base=styled.div({p:2});const Card=styled(Base)({color:input.before,p:function(p,unused=(trace.push('default'),0)){trace.push(this===globalThis?'global':'receiver',arguments.length);return p.input.value;},m:p=>p.input.value,color:input.after},(p,unused=(trace.push('whole'),0))=>({opacity:p.opacity}),{color:'green'});";
    // When: a complete render evaluates all authored operands, including covered leaves.
    let actual = execute_in_mode(
        "ordinary-order.ts",
        source,
        "const e=render({target:Card,props:{input,opacity:0.5}});JSON.stringify([trace,values(e,'padding'),values(e,'margin'),values(e,'opacity'),values(e,'color')]);",
        single_css,
    );
    // Then: one direct invocation preserves params, this, arguments and original read order.
    assert_eq!(
        actual,
        serde_json::json!([
            [
                "before", "default", "global", 1, "value1", "value2", "after", "whole"
            ],
            ["16px"],
            [],
            [0.5],
            ["green"]
        ])
    );
}

#[rstest]
#[serial]
fn longhand_and_raw_values_keep_representation_when_ordinary_callbacks_run(
    #[values(false, true)] single_css: bool,
) {
    // Given: longhand, unitless and raw custom-property callback boundaries.
    let source = "const Card=styled.div({padding:p=>p.value,opacity:p=>p.value,'--raw':p=>p.value});const Whole=styled.div(p=>({padding:p.value}));";
    // When: a complete render supplies numeric4.
    let actual = execute_in_mode(
        "ordinary-units.ts",
        source,
        "const e=render({target:Card,props:{ value: 4 }});JSON.stringify([values(e,'padding'),values(e,'opacity'),values(e,'--raw'),values(render({target:Whole,props:{ value: 4 }}),'padding')]);",
        single_css,
    );
    // Then: ordinary numeric domains stay raw, while the whole-rule longhand retains px.
    assert_eq!(actual, serde_json::json!([[4], [4], [4], ["4px"]]));
}
