use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::property("p=>({p:[null,null,p.on?4:null]})")]
#[case::whole_rule("p=>p.on?{p:[null,null,4]}:{}")]
#[serial]
fn responsive_indices_keep_base_when_callback_branch_is_absent(
    #[case] callback: &str,
    #[values(false, true)] on: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: distinct inherited declarations at levels0/2/4.
    let source = format!(
        "const Base=styled.div({{p:[1,null,2,null,3]}});const Card=styled(Base)({callback});"
    );
    // When: the complete renderer invokes the authored callback.
    let actual = execute_in_mode(
        "callback-indices.ts",
        &source,
        &format!(
            "const element=render({{target:Card,props:{{on:{on}}}}});JSON.stringify([values(element,'padding',0),values(element,'padding',2),values(element,'padding',4)]);"
        ),
        single_css,
    );
    // Then: omission Keeps each level and replacement affects only index2.
    assert_eq!(
        actual,
        serde_json::json!([["4px"], [if on { "16px" } else { "8px" }], ["12px"]])
    );
}

#[rstest]
#[case::standalone("const Card=styled.div(p=>({padding:3,p:p.on?4:null}));", serde_json::json!([["12px"],["12px","16px"]]))]
#[case::inherited("const Base=styled.div({ p: 2 });const Card=styled(Base)(p=>({ padding: 3,p:p.on?4:null }));", serde_json::json!([["12px"],["16px"]]))]
#[case::known("const mixin=css({ p: 2 });const Card=styled.div(mixin,p=>({ padding: 3,p:p.on?4:null }));", serde_json::json!([["12px"],["16px"]]))]
#[case::previous("const Card=styled.div({ p: 2 },p=>({ p: p.on?4:null }));", serde_json::json!([["8px"],["16px"]]))]
#[case::layer("const Card=styled.div(p=>({'@layer':{base:{padding:3,p:p.on?4:null}}}));", serde_json::json!([["12px"],["12px","16px"]]))]
#[case::layer_inherited("const Base=styled.div({'@layer':{base:{p:2}}});const Card=styled(Base)(p=>({'@layer':{base:{padding:3,p:p.on?4:null}}}));", serde_json::json!([["12px"],["16px"]]))]
#[case::layer_known("const mixin=css({'@layer':{base:{p:2}}});const Card=styled.div(mixin,p=>({'@layer':{base:{p:p.on?4:null}}}));", serde_json::json!([["8px"],["16px"]]))]
#[serial]
fn authored_scalar_keeps_previous_when_callback_selects_null(
    #[case] source: &str,
    #[case] expected: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: a written later native shorthand and a distinguishable earlier alias.
    // When: both states execute through the original renderer independently.
    let actual = execute_in_mode(
        "callback-scalar.ts",
        source,
        "JSON.stringify([false,true].map(on=>values(render({target:Card,props:{on}}),'padding')));",
        single_css,
    );
    // Then: null Keeps the previous declaration and the static native4 is16px.
    assert_eq!(actual, expected);
}

#[rstest]
#[case::array("p:[null,null,p.input.on?4:null]")]
#[case::scalar("padding:p.input.on?p.input.value:null")]
#[serial]
fn callback_leaves_execute_once_when_later_part_covers_color(
    #[case] property: &str,
    #[values(false, true)] on: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: the default, controller and covered color getter are observable.
    let array = property.starts_with("p:[");
    let initial = if array { 1 } else { 2 };
    let source = format!(
        "const trace=[];const Base=styled.div({{p:[{initial},null,2,null,3]}});const Card=styled(Base)((p,unused=(trace.push('default'),0))=>({{{property},color:p.input.color}}),{{color:'green'}});const input={{get on(){{trace.push('on');return {on};}},get value(){{trace.push('value');return 4;}},get color(){{trace.push('color');return 'red';}}}};"
    );
    // When: a complete render evaluates only the authored selected scalar branch.
    let actual = execute_in_mode(
        "callback-trace.ts",
        &source,
        "const element=render({target:Card,props:{input}});JSON.stringify([trace,values(element,'padding',0),values(element,'padding',2),values(element,'color')]);",
        single_css,
    );
    // Then: covered leaves still run in order and ordinary dynamic padding stays px.
    let trace = if on && !array {
        vec!["default", "on", "value", "color"]
    } else {
        vec!["default", "on", "color"]
    };
    assert_eq!(
        actual,
        serde_json::json!([
            trace,
            [if !array && !on { "8px" } else { "4px" }],
            [if array && on { "16px" } else { "8px" }],
            ["green"]
        ])
    );
}
