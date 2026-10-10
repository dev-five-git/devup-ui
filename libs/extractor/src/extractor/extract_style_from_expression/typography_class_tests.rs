use rstest::rstest;
use serial_test::serial;

use crate::assignment_test_support::{extracted, jsx_js};

mod class_only_tests;
mod context_tests;
mod support;

#[rstest]
#[case(("t", &[][..]))]
#[case(("typography", &["t"][..]))]
#[case(("state.t", &["t"][..]))]
#[case(("state[state.key]", &["key", "t"][..]))]
#[case(("state.pick(state.arg)", &["arg", "call"][..]))]
#[case(("state.flag?state.t:state.bad", &["flag", "t"][..]))]
#[case(("state.flag?state.bad:state.t", &["flag", "t"][..]))]
#[case(("state.t||state.bad", &["t"][..]))]
#[case(("state.t??state.bad", &["t"][..]))]
#[case(("state.flag&&state.t", &["flag", "t"][..]))]
#[case(("state?.t", &["t"][..]))]
#[case(("(state.t, state.t)", &["t", "t"][..]))]
#[case(("`${state.t}`", &["t"][..]))]
#[serial]
fn selects_typography_once_when_scalar_is_dynamic(
    #[case] selection: (&str, &[&str]),
    #[values(false, true)] runtime: bool,
    #[values("alone", "before", "after")] order: &str,
) {
    // Given: observable operands and a throwing unselected branch.
    let (expression, reads) = selection;
    let mut source = support::fixture(expression, runtime, order);
    if expression == "state.flag?state.bad:state.t" {
        source = source.replace("return true", "return false");
    }
    let mut trace = vec!["seed"];
    if order == "before" {
        trace.push("c");
    }
    trace.extend_from_slice(reads);
    if order == "after" {
        trace.push("c");
    }
    let classes = if order == "alone" {
        vec!["typo-body"]
    } else {
        vec!["external", "typo-body"]
    };
    // When: both native engines execute the real extraction output.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}JSON.stringify([trace,String(node.className??'').split(/\\s+/).filter(Boolean).sort(),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: selected class, scheduling and absence of CSS consumers are joint contracts.
    assert_eq!(actual, serde_json::json!([trace, classes, []]));
    support::class_only(&output);
}

#[rstest]
#[case("t")]
#[case("typography")]
#[case("state.t")]
#[case("state[state.key]")]
#[case("state.pick(state.arg)")]
#[case("state.flag?state.t:state.bad")]
#[case("state.t??state.empty")]
#[serial]
fn omits_typography_when_selected_value_is_falsy(
    #[case] expression: &str,
    #[values(false, true)] runtime: bool,
    #[values("alone", "before", "after")] order: &str,
) {
    // Given: every falsy value admitted by the existing identifier convention.
    for value in ["false", "null", "undefined", "0", "''", "NaN"] {
        let source = support::fixture(expression, runtime, order)
            .replace("let value='body'", &format!("let value={value}"));
        // When: the authored selection executes, including a lazy nullish fallback.
        let output = extracted(&source);
        let actual = support::observe(&format!(
            "{}JSON.stringify([String(node.className??'').split(/\\s+/).filter(Boolean),Object.entries(node.style??{{}})]);",
            jsx_js(&output.code)
        ));
        // Then: no typography class or CSS variable survives.
        let classes: Vec<&str> = if order == "alone" {
            vec![]
        } else {
            vec!["external"]
        };
        assert_eq!(actual, serde_json::json!([classes, []]));
        support::class_only(&output);
    }
}

#[rstest]
#[case("state.t")]
#[case("state[state.key]")]
#[case("state.pick(state.arg)")]
#[case("state.flag?state.t:state.bad")]
#[serial]
fn reads_selected_value_at_its_authored_position_when_classname_mutates_it(
    #[case] expression: &str,
    #[values(false, true)] runtime: bool,
    #[values("before", "after")] order: &str,
) {
    // Given: className changes a mutable fixture value, not a resolver-foldable const.
    let source = support::fixture(expression, runtime, order)
        .replace("trace.push('c');", "trace.push('c');value='heading';");
    // When: the generated class selection observes that source order.
    let actual = support::observe(&format!(
        "{}JSON.stringify(String(node.className).split(/\\s+/).filter(Boolean).sort());",
        jsx_js(&extracted(&source).code)
    ));
    // Then: moving typography across className changes which preset is selected.
    let class = if order == "before" {
        "typo-heading"
    } else {
        "typo-body"
    };
    assert_eq!(actual, serde_json::json!(["external", class]));
}

#[rstest]
#[case("state.t")]
#[case("state[state.key]")]
#[case("state.pick(state.arg)")]
#[case("state.flag?state.t:state.bad")]
#[serial]
fn stops_later_operands_when_classname_throws(
    #[case] expression: &str,
    #[values(false, true)] runtime: bool,
    #[values("before", "after")] order: &str,
) {
    // Given: className throws, so only earlier typography operands may execute.
    let source = support::fixture(expression, runtime, order)
        .replace("return 'external'", "throw Error('class failed')")
        .replace(
            "const node=render(state.seed,state);",
            "let error;try{render(state.seed,state)}catch(value){error=value.message}",
        );
    let result = "JSON.stringify([trace,error]);";
    let original = support::observe(&format!("{}{result}", jsx_js(&source)));
    // When: compiled operands execute under the same exception boundary.
    let actual = support::observe(&format!("{}{result}", jsx_js(&extracted(&source).code)));
    // Then: throwing and counting source witnesses match, not merely generated text.
    assert_eq!(actual, original);
    assert_eq!(actual[1], "class failed");
    if order == "before" {
        assert_eq!(actual[0], serde_json::json!(["seed", "c"]));
    }
}

#[rstest]
#[case("state.t||state.empty")]
#[case("state.t??state.empty")]
#[serial]
fn reads_fallback_once_when_left_typography_is_nullish(
    #[case] expression: &str,
    #[values(false, true)] runtime: bool,
    #[values("alone", "before", "after")] order: &str,
) {
    // Given: distinct left/fallback values and an observable logical left operand.
    let source = support::fixture(expression, runtime, order)
        .replace("let value='body'", "let value=null")
        .replace(
            "trace.push('empty');return value",
            "trace.push('empty');return 'heading'",
        );
    let mut trace = vec!["seed"];
    if order == "before" {
        trace.push("c");
    }
    trace.extend(["t", "empty"]);
    if order == "after" {
        trace.push("c");
    }
    // When: a fallback is selected at the typography operand's authored position.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}JSON.stringify([trace,String(node.className??'').split(/\\s+/).filter(name=>name.startsWith('typo-')),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: left and fallback each read once, selecting only the fallback class.
    assert_eq!(actual, serde_json::json!([trace, ["typo-heading"], []]));
    support::class_only(&output);
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn guard_is_lazy_when_typography_branch_is_unselected(#[case] runtime: bool) {
    // Given: a false style guard protecting a throwing branch.
    let source = support::fixture("state.flag&&state.bad", runtime, "after")
        .replace("return true", "return false");
    // When: the authored logical selection executes.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}JSON.stringify([trace,String(node.className).trim(),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: the branch stays unexecuted and no typography class/variable is emitted.
    assert_eq!(
        actual,
        serde_json::json!([["seed", "flag", "c"], "external", []])
    );
    support::class_only(&output);
}

#[rstest]
#[serial]
fn template_typography_keeps_empty_string_class_when_authored_value_is_coerced(
    #[values(false, true)] runtime: bool,
    #[values("alone", "before", "after")] order: &str,
) {
    // Given: the existing template path produces a string, not a falsy raw preset.
    let source = support::fixture("`${state.t}`", runtime, order)
        .replace("let value='body'", "let value=''");
    // When: extraction captures the template's authored read before later operands.
    let output = extracted(&source);
    let actual = support::observe(&format!(
        "{}JSON.stringify([trace,String(node.className).split(/\\s+/).filter(name=>name.startsWith('typo-')),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: source scheduling and the pre-existing typo- class convention both survive.
    let trace = match order {
        "alone" => vec!["seed", "t"],
        "before" => vec!["seed", "c", "t"],
        "after" => vec!["seed", "t", "c"],
        _ => panic!("fixture order"),
    };
    assert_eq!(actual, serde_json::json!([trace, ["typo-"], []]));
    support::class_only(&output);
}
