use rstest::rstest;
use serial_test::serial;

use super::support;
use crate::{
    ExtractStyleValue,
    assignment_test_support::{extracted, jsx_js},
};

#[rstest]
#[case("state.t", &["t"][..])]
#[case("state[state.key]", &["key", "t"][..])]
#[case("state.pick(state.arg)", &["arg", "call"][..])]
#[case("state.flag?state.t:state.bad", &["flag", "t"][..])]
#[case("state.t||state.bad", &["t"][..])]
#[serial]
fn selector_typography_selects_only_scoped_preset_when_value_is_dynamic(
    #[case] expression: &str,
    #[case] reads: &[&str],
    #[values(false, true)] runtime: bool,
) {
    // Given: a dynamic preset inside a mixed selector object with an adjacent literal.
    let _theme = support::TypographyTheme::register();
    let source = support::fixture(expression, runtime, "after");
    let source = if runtime {
        source.replace(
            &format!("typography:{expression}"),
            &format!("_hover:{{typography:{expression},color:'red'}}"),
        )
    } else {
        source.replace(
            &format!("typography={{{expression}}}"),
            &format!("_hover={{{{typography:{expression},color:'red'}}}}"),
        )
    };
    // When: class-only and declaration consumers lower together at the selector field.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}const records={};JSON.stringify([String(node.className).split(/\\s+/).filter(name=>name&&name!=='external').map(name=>records.find(([key])=>key===name)[1]).sort(),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code),
        support::declarations(&output)
    ));
    // Then: only body is selected, and its CSS atom is still scoped to hover.
    assert_eq!(
        actual,
        serde_json::json!([["color:red:0", "typography:body:0"], []])
    );
    let mut trace = vec!["seed"];
    trace.extend_from_slice(reads);
    trace.push("c");
    assert_eq!(
        support::observe(&format!("{}JSON.stringify(trace);", jsx_js(&output.code))),
        serde_json::json!(trace)
    );
    for value in &output.styles {
        let ExtractStyleValue::Static(style) = value else {
            panic!("scoped static atom expected")
        };
        assert_eq!(
            style.selector.as_ref().map(ToString::to_string).as_deref(),
            Some("&:hover")
        );
    }
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn responsive_typography_keeps_breakpoint_when_dynamic_preset_follows_static_preset(
    #[case] runtime: bool,
    #[values("alone", "before", "after")] order: &str,
) {
    // Given: a shared static root preset and a dynamic preset at breakpoint one.
    let _theme = support::TypographyTheme::register();
    let source = support::fixture("['heading',state.t]", runtime, order);
    // When: the two class roles are selected through the same authored array operand.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}const records={};JSON.stringify([String(node.className).split(/\\s+/).filter(name=>name&&name!=='external').map(name=>name==='typo-heading'?name:records.find(([key])=>key===name)[1]).sort(),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code),
        support::declarations(&output)
    ));
    // Then: the later breakpoint selects body, without applying all presets globally.
    assert_eq!(
        actual,
        serde_json::json!([["typo-heading", "typography:body:1"], []])
    );
    let trace = match order {
        "alone" => vec!["seed", "t"],
        "before" => vec!["seed", "c", "t"],
        "after" => vec!["seed", "t", "c"],
        _ => panic!("fixture order"),
    };
    assert_eq!(
        support::observe(&format!("{}JSON.stringify(trace);", jsx_js(&output.code))),
        serde_json::json!(trace)
    );
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|value| matches!(value, ExtractStyleValue::Dynamic(_)))
            .count(),
        0
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn static_typography_remains_shared_when_a_literal_preset_is_authored(#[case] runtime: bool) {
    // Given: the pre-existing static preset path, with an adjacent dynamic className.
    let source = support::fixture("'heading'", runtime, "after");
    // When: the literal is extracted normally rather than rerouted into dynamic CSS.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}JSON.stringify(String(node.className).split(/\\s+/).filter(Boolean).sort());",
        jsx_js(&output.code)
    ));
    // Then: its shared preset class survives without declarations or variables.
    assert_eq!(actual, serde_json::json!(["external", "typo-heading"]));
    support::class_only(&output);
}
