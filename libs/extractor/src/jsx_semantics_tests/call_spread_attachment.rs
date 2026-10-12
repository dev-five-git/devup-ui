//! Whole-output regressions for syntax-wrapped jsx-runtime props and JSX-02.

use super::*;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{JsxRuntime, TransformOptions, Transformer};
use rstest::rstest;
use serial_test::serial;
use std::path::Path;

/// Run the complete extracted file, including every jsx-runtime factory.
fn evaluate(source: &str, probe: &str) -> whole::Evaluation {
    let compiled = output(source).code;
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &compiled, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{compiled}");
    let mut program = parsed.program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let mut options = TransformOptions::default();
    options.jsx.runtime = JsxRuntime::Classic;
    options.jsx.pragma = Some("h".to_string());
    let transformed = Transformer::new(&allocator, Path::new("a.tsx"), &options)
        .build_with_scoping(scoping, &mut program);
    assert_eq!(transformed.diagnostics.len(), 0, "{compiled}");
    let script = Codegen::new()
        .build(&program)
        .code
        .lines()
        .filter(|line| !line.starts_with("import "))
        .map(|line| line.strip_prefix("export ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    let joined = run(&format!(
        "const trace=[]; const h=(type,props)=>{{const copied={{...props}};if(typeof copied.className==='string')copied.className=copied.className.trim().split(/\\s+/).sort().join(' ');return {{type,props:copied}};}}; const jsx=h,jsxs=h,jsxDEV=h; {script}\nreturn JSON.stringify([{probe},trace]);"
    ));
    let (element, trace) =
        serde_json::from_str(&joined).unwrap_or_else(|error| panic!("{error}: {joined}"));
    whole::Evaluation { element, trace }
}

fn source(runtime: &str, props: &str) -> String {
    let module = match runtime {
        "jsx" | "jsxs" => "react/jsx-runtime",
        "jsxDEV" => "react/jsx-dev-runtime",
        _ => panic!("unknown test factory: {runtime}"),
    };
    let tail = match runtime {
        "jsx" | "jsxs" => ", 'key'",
        "jsxDEV" => ", 'key', false, undefined, undefined",
        _ => panic!("unknown test factory: {runtime}"),
    };
    format!(
        "{BOX}import {{{runtime}}} from '{module}'; export const a=(first,read)=>{runtime}(Box,{props}{tail});"
    )
}

/// Compare full native props, not just emitted CSS or a detached identifier.
fn assert_parity(call: &str, jsx: &str, probe: &str) -> whole::Evaluation {
    let plain = evaluate(&format!("{BOX}export const a=(first,read)=>{jsx};"), probe);
    let actual = evaluate(call, probe);
    assert_eq!(actual.trace, plain.trace, "{call}");
    assert_eq!(actual.element, plain.element, "{call}");
    actual
}

#[rstest]
#[case("jsx")]
#[case("jsxs")]
#[case("jsxDEV")]
#[serial]
fn styles_attach_when_the_original_evidence_object_is_parenthesized(#[case] runtime: &str) {
    let module = match runtime {
        "jsx" | "jsxs" => "react/jsx-runtime",
        "jsxDEV" => "react/jsx-dev-runtime",
        _ => panic!("unknown test factory: {runtime}"),
    };
    let fixture = format!(
        "{BOX}import {{{runtime}}} from '{module}'; export const a=(first:()=>string,read:()=>object)=>{runtime}(Box,({{color:first(),...read()}}));"
    );
    let actual = evaluate(
        &fixture,
        "(()=>{const e=a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {color:'green'};});return [e.type,e.props.color,(e.props.className||'').trim().length>0,Object.values(e.props.style||{})];})()",
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert_eq!(
        actual.element,
        serde_json::json!(["div", "green", true, ["green"]])
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
    let actual = assert_parity(
        &source(runtime, &props),
        "<Box color={first()} {...read()}/>",
        "a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {color:'green',id:'kept',className:'external',style:{opacity:0.5}};})",
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
}

#[rstest]
#[case("{color:'green'}", "green")]
#[case("{}", "red")]
#[case("null", "red")]
#[case("undefined", "red")]
#[case("Object.create({color:'inherited'})", "red")]
#[case("Object.defineProperty({},'color',{value:'hidden'})", "red")]
#[serial]
fn dynamic_fallback_matches_jsx_when_spread_copying_selects_own_enumerable_keys(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[case] spread: &str,
    #[case] expected: &str,
) {
    let actual = assert_parity(
        &source(runtime, "({color:first(),...read()})"),
        "<Box color={first()} {...read()}/>",
        &format!(
            "(()=>{{const e=a(()=>{{trace.push('first');return 'red';}},()=>{{trace.push('spread');return {spread};}});return Object.values(e.props.style||{{}});}})()"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
    assert_eq!(actual.element, serde_json::json!([expected]));
}

#[rstest]
#[serial]
fn absent_own_values_match_jsx_when_the_spread_replaces_the_dynamic_style(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[values("null", "undefined", "false")] absent: &str,
) {
    let actual = assert_parity(
        &source(runtime, "({color:first(),...read()})"),
        "<Box color={first()} {...read()}/>",
        &format!(
            "a(()=>{{trace.push('first');return 'red';}},()=>{{trace.push('spread');return {{color:{absent}}};}})"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
}

#[rstest]
#[serial]
fn getters_run_once_when_a_later_spread_supplies_the_style_value(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
) {
    let actual = assert_parity(
        &source(runtime, "({color:first(),...read()})"),
        "<Box color={first()} {...read()}/>",
        "a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {get color(){trace.push('getter');return 'green';}};})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["first", "spread", "getter"])
    );
}

#[rstest]
#[serial]
fn explicit_values_win_when_written_after_the_unknown_spread(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
) {
    let actual = assert_parity(
        &source(runtime, "({...read(),color:first(),_hover:{bg:'blue'}})"),
        "<Box {...read()} color={first()} _hover={{bg:'blue'}}/>",
        "a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {color:'green',id:'kept'};})",
    );
    assert_eq!(actual.trace, serde_json::json!(["spread", "first"]));
}

#[rstest]
#[serial]
fn responsive_values_match_jsx_when_followed_by_an_unknown_spread(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[values("{}", "{color:'green'}")] spread: &str,
) {
    let actual = assert_parity(
        &source(runtime, "({color:[first(),null,'blue'],...read()})"),
        "<Box color={[first(),null,'blue']} {...read()}/>",
        &format!(
            "a(()=>{{trace.push('first');return 'red';}},()=>{{trace.push('spread');return {spread};}})"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "spread"]));
}

#[rstest]
#[serial]
fn conditional_order_attaches_styles_when_wrapped_props_select_either_branch(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
    #[values("true", "false")] enabled: &str,
) {
    let actual = assert_parity(
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
}

#[rstest]
#[serial]
fn selector_replacement_rejects_wrapped_props_when_an_unknown_spread_follows(
    #[values("jsx", "jsxs", "jsxDEV")] runtime: &str,
) {
    let plain = error(&format!(
        "{BOX}export const a=(first,read)=><Box _hover={{{{bg:'blue'}}}} {{...read()}}/>;"
    ));
    let wrapped = error(&source(runtime, "({_hover:{bg:'blue'},...read()})"));
    assert_eq!(
        wrapped
            .split_once(": ")
            .unwrap_or_else(|| panic!("{wrapped}"))
            .1,
        plain
            .split_once(": ")
            .unwrap_or_else(|| panic!("{plain}"))
            .1,
    );
    assert!(wrapped.contains("`_hover`"), "{wrapped}");
}
