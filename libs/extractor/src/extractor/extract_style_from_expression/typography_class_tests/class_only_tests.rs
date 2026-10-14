use rstest::rstest;
use serial_test::serial;

use super::support;
use crate::assignment_test_support::{extracted, jsx_js};

#[rstest]
#[case("state.t")]
#[case("state[state.key]")]
#[case("state.pick(state.arg)")]
#[case("state.flag?state.t:state.bad")]
#[case("state.t||state.bad")]
#[case("state.t??state.bad")]
#[serial]
fn positioning_keeps_expanded_classes_when_classname_changes_the_selected_value(
    #[case] expression: &str,
    #[values(false, true)] runtime: bool,
    #[values("before", "after")] order: &str,
) {
    // Given: an enum class-only property with an order-sensitive selection.
    let source = support::fixture(expression, runtime, order)
        .replace("typography", "positioning")
        .replace("let value='body'", "let value='top-left'")
        .replace("trace.push('c');", "trace.push('c');value='bottom-right';");
    // When: selected generated classes are joined to their actual CSS declarations.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}const records={};JSON.stringify([String(node.className).split(/\\s+/).filter(name=>name!=='external').map(name=>records.find(([key])=>key===name)[1]).sort(),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code),
        support::declarations(&output)
    ));
    // Then: no enum declaration/variable appears and the correct two edge classes win.
    let declarations = if order == "before" {
        vec!["bottom:0:0", "right:0:0"]
    } else {
        vec!["left:0:0", "top:0:0"]
    };
    assert_eq!(actual, serde_json::json!([declarations, []]));
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|value| matches!(value, crate::ExtractStyleValue::Dynamic(_)))
            .count(),
        0
    );
}

#[rstest]
#[case("before", &["seed", "c", "t"][..])]
#[case("after", &["seed", "t", "c"][..])]
#[serial]
fn as_keeps_type_and_order_when_adjacent_to_classname(
    #[case] order: &str,
    #[case] trace: &[&str],
    #[values(false, true)] runtime: bool,
) {
    // Given: a class-free tag operand beside an observable className.
    let source = support::fixture("state.t", runtime, order)
        .replace("typography", "as")
        .replace("let value='body'", "let value='section'");
    // When: both caller forms execute the compiled type capture.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}JSON.stringify([trace,node.tag,node.className,Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: the selected tag and operand schedule are unchanged.
    assert_eq!(
        actual,
        serde_json::json!([trace, "section", "external", []])
    );
    assert_eq!(output.styles.len(), 0);
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn typography_keeps_supported_unknown_spread_when_it_follows_selection(#[case] runtime: bool) {
    // Given: a mutable spread supplies only an ordinary id, not #756 fallback semantics.
    let source = support::fixture("state.t", runtime, "alone")
        .replace("/>", " {...state.rest}/>")
        .replace("typography:state.t}", "typography:state.t,...state.rest}")
        .replace(
            "get c()",
            "get rest(){trace.push('rest');return {id:'target'}},get c()",
        );
    // When: the supported unknown-spread program runs through extraction.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}JSON.stringify([trace,String(node.className).trim(),node.id,Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: selection precedes the once-read spread, retaining its ordinary property.
    assert_eq!(
        actual,
        serde_json::json!([["seed", "t", "rest"], "typo-body", "target", []])
    );
    support::class_only(&output);
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn call_typography_coerces_selected_value_before_next_operand(#[case] runtime: bool) {
    // Given: receiver/argument-sensitive call returns an observably coerced preset.
    let source = support::fixture("state.pick(state.arg)", runtime, "after").replace(
        "let value='body'",
        "let value={toString(){trace.push('coerce');return 'body'}}",
    );
    // When: the entire call is captured, then its result is used as a class.
    let actual = support::observe(&format!(
        "{}JSON.stringify([trace,String(node.className).split(/\\s+/).filter(Boolean).sort()]);",
        jsx_js(&extracted(&source).code)
    ));
    // Then: coercion, receiver and arguments are each observed exactly once in order.
    assert_eq!(
        actual,
        serde_json::json!([
            ["seed", "arg", "call", "coerce", "c"],
            ["external", "typo-body"]
        ])
    );
}

#[rstest]
#[serial]
fn positioning_selects_ordered_atoms_when_style_order_is_conditional(
    #[values(false, true)] runtime: bool,
    #[values(false, true)] active: bool,
) {
    // Given: conditional order precedes an observable enum selection.
    let source = support::fixture("state.t", runtime, "after")
        .replace("typography", "positioning")
        .replace("let value='body'", "let value='top-left'")
        .replace(
            "positioning:state.t",
            "styleOrder:state.flag?5:10,positioning:state.t",
        )
        .replace(
            "positioning={state.t}",
            "styleOrder={state.flag?5:10} positioning={state.t}",
        )
        .replace(
            "return true",
            if active {
                "return true"
            } else {
                "return false"
            },
        );
    // When: both order branches generate their classes after capturing the raw enum value.
    let output = extracted(&source);
    let records = output
        .styles
        .iter()
        .filter_map(|value| {
            let crate::ExtractStyleValue::Static(style) = value else {
                return None;
            };
            let crate::extract_style::style_property::StyleProperty::ClassName(class) =
                value.extract(None)?
            else {
                return None;
            };
            Some((class, (&style.property, &style.value, style.style_order)))
        })
        .collect::<Vec<_>>();
    let actual = support::observe(&format!(
        "{}const records={};JSON.stringify([trace,String(node.className).split(/\\s+/).filter(name=>name&&name!=='external').map(name=>records.find(([key])=>key===name)[1]).sort(),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code),
        serde_json::to_string(&records).unwrap_or_else(|error| panic!("{error}"))
    ));
    // Then: exactly the selected order's left/top atoms match emitted CSS, with one enum read.
    let order = if active { 5 } else { 10 };
    assert_eq!(
        actual,
        serde_json::json!([
            ["seed", "flag", "t", "c"],
            [["left", "0", order], ["top", "0", order]],
            []
        ])
    );
}
