use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::number("4", serde_json::json!(["16px"]))]
#[case::string("'4'", serde_json::json!(["4"]))]
#[case::pixels("'4px'", serde_json::json!(["4px"]))]
#[case::keyword("'auto'", serde_json::json!(["auto"]))]
#[case::zero("0", serde_json::json!([0]))]
#[case::negative("-2", serde_json::json!(["-8px"]))]
#[case::null("null", serde_json::json!(["8px"]))]
#[case::undefined("undefined", serde_json::json!(["8px"]))]
#[case::false_value("false", serde_json::json!(["8px"]))]
#[serial]
fn captured_shorthand_keeps_units_and_absence_when_input_varies(
    #[case] value: &str,
    #[case] expected: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: a distinguishable inherited native shorthand.
    let source = "const Base=styled.div({ p: 2 });const Card=styled(Base)(p=>({ p: p.value }));";
    // When: the complete original callback captures the supplied application value.
    let actual = execute_in_mode(
        "value-transport.ts",
        source,
        &format!(
            "JSON.stringify(values(render({{target:Card,props:{{value:{value}}}}}),'padding'));"
        ),
        single_css,
    );
    // Then: numbers use native scale, strings pass through, and missing values Keep.
    assert_eq!(actual, expected);
}

#[rstest]
#[case::undefined("p.on?4:undefined")]
#[case::void("p.on?4:void 0")]
#[case::logical("p.on&&4")]
#[case::false_value("p.on?4:false")]
#[serial]
fn earlier_value_is_kept_when_captured_expression_is_absent(
    #[case] expression: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: a missing branch must not erase the inherited declaration.
    let source =
        format!("const Base=styled.div({{p:2}});const Card=styled(Base)(p=>({{p:{expression}}}));");
    // When: complete renderers independently exercise the two authored states.
    let actual = execute_in_mode(
        "value-omission.ts",
        &source,
        "JSON.stringify([false,true].map(on=>values(render({target:Card,props:{on}}),'padding')));",
        single_css,
    );
    // Then: the omitted side Keeps8 and the selected number scales to16.
    assert_eq!(actual, serde_json::json!([["8px"], ["16px"]]));
}

#[rstest]
#[case::logical("p.input.on&&p.input.value", "0", serde_json::json!([0]), vec!["default", "on", "color"])]
#[case::void("p.input.on?p.input.value:void p.input.effect", "false", serde_json::json!(["8px"]), vec!["default", "on", "effect", "color"])]
#[case::number("p.input.on?p.input.value:undefined", "true", serde_json::json!(["16px"]), vec!["default", "on", "value", "color"])]
#[serial]
fn original_effects_run_once_when_later_part_covers_a_leaf(
    #[case] expression: &str,
    #[case] on: &str,
    #[case] expected: serde_json::Value,
    #[case] trace: Vec<&str>,
    #[values(false, true)] single_css: bool,
) {
    // Given: a default initializer, changing getters, and a covered color leaf.
    let source = format!(
        "const trace=[];const Base=styled.div({{p:2}});const Card=styled(Base)((p,unused=(trace.push('default'),0))=>({{p:{expression},color:p.input.color}}),{{color:'green'}});const input={{get on(){{trace.push('on');return {on};}},get value(){{trace.push('value');return 4;}},get effect(){{trace.push('effect');return 9;}},get color(){{trace.push('color');return 'red';}}}};"
    );
    // When: the complete callback runs in its original scope.
    let actual = execute_in_mode(
        "value-effects.ts",
        &source,
        "const element=render({target:Card,props:{input}});JSON.stringify([trace,values(element,'padding'),values(element,'color')]);",
        single_css,
    );
    // Then: no controller/default/getter is replayed, including the covered leaf.
    assert_eq!(actual, serde_json::json!([trace, expected, ["green"]]));
}

#[rstest]
#[serial]
fn bound_undefined_is_application_data_when_callback_shadows_global(
    #[values(false, true)] single_css: bool,
) {
    // Given: the parameter named undefined is actually the props object.
    let source = "const Base=styled.div({ p: 2 });const Card=styled(Base)(undefined=>({ p: undefined.value }));";
    // When: the application supplies a number through that binding.
    let actual = execute_in_mode(
        "value-shadow.ts",
        source,
        "JSON.stringify(values(render({target:Card,props:{ value: 4 }}),'padding'));",
        single_css,
    );
    // Then: identifier spelling cannot turn the actual value into omission.
    assert_eq!(actual, serde_json::json!(["16px"]));
}
