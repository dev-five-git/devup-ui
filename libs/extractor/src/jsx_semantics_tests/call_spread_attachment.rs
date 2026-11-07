use super::*;
use rstest::rstest;
use serial_test::serial;

fn source(runtime: &str, props: &str) -> String {
    let module = match runtime {
        "jsx" | "jsxs" => "react/jsx-runtime",
        "jsxDEV" => "react/jsx-dev-runtime",
        _ => panic!("unknown factory: {runtime}"),
    };
    let tail = match runtime {
        "jsx" | "jsxs" => ", 'key'",
        "jsxDEV" => ", 'key', false, undefined, undefined",
        _ => panic!("unknown factory: {runtime}"),
    };
    format!(
        "{BOX}import {{{runtime} as jsx}} from '{module}';export const a=(first,read)=>jsx(Box,{props}{tail});"
    )
}

fn parity(call: &str, element: &str, probe: &str) -> whole::Evaluation {
    let plain = whole::evaluate(
        &format!("{BOX}export const a=(first,read)=>{element};"),
        probe,
    );
    let actual = whole::evaluate(call, probe);
    assert_eq!(actual.trace, plain.trace, "{call}");
    assert_eq!(actual.element["type"], plain.element["type"], "{call}");
    assert_eq!(actual.element["props"], plain.element["props"], "{call}");
    actual
}

#[rstest]
#[case("jsx")]
#[case("jsxs")]
#[case("jsxDEV")]
#[serial]
fn styles_attach_when_the_original_evidence_object_is_parenthesized(#[case] runtime: &str) {
    let actual = whole::evaluate(
        &source(runtime, "({color:first(),...read()})"),
        "(()=>{const e=a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {color:'green'};});return [e.type,e.props.color,(e.props.className||'').trim().length>0,Object.values(e.props.style||{})];})()",
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert_eq!(
        actual.element,
        serde_json::json!(["div", "green", true, ["red"]])
    );
}

#[rstest]
#[serial]
fn props_match_plain_jsx_when_syntax_wrappers_surround_the_object(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[values(
        "BODY",
        "(BODY)",
        "((BODY))",
        "(BODY as const)",
        "(BODY satisfies object)"
    )]
    wrapper: &str,
) {
    let props = wrapper.replace("BODY", "{color:first(),...read()}");
    let actual = parity(
        &source(runtime, &props),
        "<Box color={first()} {...read()}/>",
        "a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {color:'green',id:'kept',className:'external',style:{opacity:0.5}};})",
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert_eq!(actual.element["props"]["color"], "green");
    assert_eq!(actual.element["props"]["style"]["opacity"], 0.5);
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .unwrap_or_else(|| panic!("expected an authored style object"))
            .values()
            .any(|v| v == "red")
    );
}

#[rstest]
#[case("{color:'green'}")]
#[case("{}")]
#[case("null")]
#[case("undefined")]
#[case("Object.create({color:'inherited'})")]
#[case("Object.defineProperty({},'color',{value:'hidden'})")]
#[serial]
fn authored_values_when_spreads_copy_own_enumerable_keys_remain_attached(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[case] spread: &str,
) {
    let actual = parity(
        &source(runtime, "({color:first(),...read()})"),
        "<Box color={first()} {...read()}/>",
        &format!(
            "(()=>{{const e=a(()=>{{trace.push('first');return 'red';}},()=>{{trace.push('spread');return {spread};}});return Object.values(e.props.style||{{}});}})()"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert_eq!(actual.element, serde_json::json!(["red"]));
}

#[rstest]
#[serial]
fn absent_own_values_when_passed_through_keep_the_authored_style(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[values("null", "undefined", "false")] absent: &str,
) {
    let actual = parity(
        &source(runtime, "({color:first(),...read()})"),
        "<Box color={first()} {...read()}/>",
        &format!(
            "(()=>{{const e=a(()=>{{trace.push('first');return 'red';}},()=>{{trace.push('spread');return {{color:{absent}}};}});return [Object.hasOwn(e.props,'color'),Object.is(e.props.color,{absent}),Object.values(e.props.style||{{}})];}})()"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert_eq!(actual.element, serde_json::json!([true, true, ["red"]]));
}

#[rstest]
#[serial]
fn getters_when_passed_through_run_once(#[values("jsx", "jsxs", "jsxDEV")] runtime: &str) {
    let actual = parity(
        &source(runtime, "({color:first(),...read()})"),
        "<Box color={first()} {...read()}/>",
        "a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {get color(){trace.push('getter');return 'green';}};})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["first", "spread", "getter"])
    );
    assert_eq!(actual.element["props"]["color"], "green");
}

#[rstest]
#[serial]
fn explicit_values_when_written_after_unknown_spreads_attach(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
) {
    let actual = parity(
        &source(runtime, "({...read(),color:first(),_hover:{bg:'blue'}})"),
        "<Box {...read()} color={first()} _hover={{bg:'blue'}}/>",
        "a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {color:'green',id:'kept'};})",
    );
    assert_eq!(actual.trace, serde_json::json!(["spread", "first"]));
    assert_eq!(actual.element["props"]["color"], "green");
}

#[rstest]
#[serial]
fn responsive_values_when_spreads_follow_match_jsx(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[values("{}", "{color:'green'}")] spread: &str,
) {
    let actual = parity(
        &source(runtime, "({color:[first(),null,'blue'],...read()})"),
        "<Box color={[first(),null,'blue']} {...read()}/>",
        &format!(
            "a(()=>{{trace.push('first');return 'red';}},()=>{{trace.push('spread');return {spread};}})"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .unwrap_or_else(|| panic!("expected an authored style object"))
            .values()
            .any(|v| v == "red")
    );
}

#[rstest]
#[serial]
fn conditional_order_when_props_are_wrapped_attaches_both_branches(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[values("true", "false")] enabled: &str,
) {
    let actual = parity(
        &source(
            runtime,
            "({styleOrder:first.on?2:4,color:first(),...read(),_hover:{bg:'blue'}})",
        ),
        "<Box styleOrder={first.on?2:4} color={first()} {...read()} _hover={{bg:'blue'}}/>",
        &format!(
            "a(Object.assign(()=>{{trace.push('first');return 'red';}},{{on:{enabled}}}),()=>{{trace.push('spread');return {{color:'green'}};}})"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .unwrap_or_else(|| panic!("expected an authored style object"))
            .values()
            .any(|v| v == "red")
    );
}

#[rstest]
#[serial]
fn selectors_when_unknown_spreads_follow_wrapped_props_compile(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
) {
    let actual = parity(
        &source(runtime, "({_hover:{bg:'blue'},...read()})"),
        "<Box _hover={{bg:'blue'}} {...read()}/>",
        "a(null,()=>({_hover:{bg:'green'}}))",
    );
    assert_eq!(actual.element["props"]["_hover"]["bg"], "green");
    assert!(
        actual.element["props"]["className"]
            .as_str()
            .unwrap_or_else(|| panic!("expected a selector class name"))
            .contains("blue")
    );
}
