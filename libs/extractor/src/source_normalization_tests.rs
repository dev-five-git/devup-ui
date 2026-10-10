use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "(`${state.value}`+';')||state.fallback",
    "''",
    "[[\"value\"],0,[\"\"]]"
)]
#[case(
    "(`${state.value}`+';')??state.fallback",
    "''",
    "[[\"value\"],0,[\"\"]]"
)]
#[case(
    "(`${state.value}`+';')&&state.fallback",
    "''",
    "[[\"value\",\"fallback\"],1,[\"blue\"]]"
)]
#[case(
    "state.controller&&(`${state.value}`+';')",
    "''",
    "[[\"controller\",\"value\"],0,[\"\"]]"
)]
#[case(
    "state.missing||(`${state.value}`+';')",
    "''",
    "[[\"missing\",\"value\"],0,[\"\"]]"
)]
#[case(
    "state.missing??(`${state.value}`+';')",
    "''",
    "[[\"missing\",\"value\"],0,[\"\"]]"
)]
#[case(
    "(state.controller?(`${state.value}`+';'):state.fallback)||state.fallback",
    "''",
    "[[\"controller\",\"value\"],0,[\"\"]]"
)]
#[case(
    "state.controller&&((`${state.value}`+';')||state.fallback)",
    "''",
    "[[\"controller\",\"value\"],0,[\"\"]]"
)]
#[case(
    "(state.missing??(`${state.value}`+';'))&&state.fallback",
    "''",
    "[[\"missing\",\"value\",\"fallback\"],1,[\"blue\"]]"
)]
#[case("state.value&&state.fallback", "0", "[[\"value\"],1,[]]")]
#[case(
    "state.value&&state.fallback",
    "'0'",
    "[[\"value\",\"fallback\"],1,[\"blue\"]]"
)]
#[case(
    "state.value||state.fallback",
    "0",
    "[[\"value\",\"fallback\"],1,[\"blue\"]]"
)]
#[case("state.value||state.fallback", "'0'", "[[\"value\"],1,[\"0px\"]]")]
#[serial]
fn normalized_presence_keeps_raw_control_flow_when_fixed_tail_is_truthy(
    #[case] expression: &str,
    #[case] value: &str,
    #[case] expected: &str,
) {
    // Given: getters expose branch laziness, while an empty normalized value is classless.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box bg={{{expression}}}/>}}let trace=[];const state={{get value(){{trace.push('value');return {value}}},get fallback(){{trace.push('fallback');return 'blue'}},get controller(){{trace.push('controller');return true}},get missing(){{trace.push('missing');return null}}}};const node=render(state);JSON.stringify([trace,String(node.className??'').split(/\\s+/).filter(Boolean).length,Object.values(node.style??{{}})]);"
    );
    // When: generated class selection and logical branches execute together.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: raw source truthiness chooses the branch, normalized presence chooses the class.
    assert_eq!(actual, expected);
}

#[rstest]
#[case("`${state.value};;;`", "red")]
#[case("`${state.value}`+';;;'", "red")]
#[case("`${state.value} !important;;;`", "red")]
#[case("`${state.value}`+' !important;;;'", "red")]
#[case("state.value", "red;")]
#[case("`${state.value}`", "red;")]
#[serial]
fn normalized_payload_keeps_static_neighbor_when_authored_tail_is_fixed(
    #[case] expression: &str,
    #[case] expected: &str,
) {
    // Given: fixed tails are removable, but a suffix inside a runtime string is not.
    let value = if expected == "red;" { "red;" } else { "red" };
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box color='blue' bg={{{expression}}} title={{state.after}}/>}}let trace=[];const state={{get value(){{trace.push('value');return {value:?}}},get after(){{trace.push('after');return 'title'}}}};const node=render(state);JSON.stringify([trace,String(node.className).split(/\\s+/).filter(Boolean).length,Object.values(node.style)]);"
    );
    // When: both style and retained attribute execute.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: one read each and the static class survive alongside the correct normalized payload.
    assert_eq!(actual, format!("[[\"value\",\"after\"],2,[{expected:?}]]"));
}

#[test]
#[serial]
fn normalized_shorthand_keeps_responsive_roles_when_static_neighbor_exists() {
    // Given: two responsive levels of a two-target shorthand share each source capture.
    let source = "import {Box}from '@devup-ui/react';function render(state){return <Box color='blue' px={[`${state.a}`+';',null,`${state.b}`+';;']}/>}let trace=[];const state={get a(){trace.push('a');return '4'},get b(){trace.push('b');return '0'}};const node=render(state);";
    // When: emitted classes resolve to real declarations and their inline payloads.
    let actual = crate::assignment_blocker_support::selected(
        source,
        "JSON.stringify([trace,assigned(node)]);",
    );
    // Then: both targets and responsive levels use normalized numeric-string conversion.
    assert_eq!(
        actual,
        "[[\"a\",\"b\"],[\"color:blue:0\",\"padding-left:16px:0\",\"padding-left:0px:2\",\"padding-right:16px:0\",\"padding-right:0px:2\"]]"
    );
}

#[rstest]
#[case(
    false,
    "[[\"getA\",\"coerceA:string\",\"getB\",\"coerceB:string\",\"after\",\"child\"],[\"red\"]]"
)]
#[case(true, "[[\"getA\",\"coerceA:string\"],\"coercion\"]")]
#[serial]
fn normalized_template_preserves_coercion_order_when_interpolation_is_observable(
    #[case] throws: bool,
    #[case] expected: &str,
) {
    // Given: object coercions expose hints and a failure must prevent later operands.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box bg={{`${{state.a}}${{state.b}}`+';'}} title={{state.after}}>{{state.child}}</Box>}}let trace=[];const state={{get a(){{trace.push('getA');return {{[Symbol.toPrimitive](hint){{trace.push('coerceA:'+hint);if({throws})throw Error('coercion');return 'r'}}}}}},get b(){{trace.push('getB');return {{[Symbol.toPrimitive](hint){{trace.push('coerceB:'+hint);return 'ed'}}}}}},get after(){{trace.push('after');return 'title'}},get child(){{trace.push('child');return 'child'}}}};let result;try{{const node=render(state);result=Object.values(node.style)}}catch(error){{result=error.message}}JSON.stringify([trace,result]);"
    );
    // When: Node executes the intact template; Boa eagerly reads all template operands before coercion.
    let script = serde_json::to_string(&compiled_jsx(&source))
        .unwrap_or_else(|error| panic!("script encoding: {error}"));
    let output = std::process::Command::new("node")
        .args(["-e", &format!("console.log(eval({script}));")])
        .output()
        .unwrap_or_else(|error| panic!("Node execution: {error}"));
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual =
        String::from_utf8(output.stdout).unwrap_or_else(|error| panic!("Node output: {error}"));
    // Then: each coercion occurs at its authored point and a throw halts later reads.
    assert_eq!(actual.trim(), expected);
}
