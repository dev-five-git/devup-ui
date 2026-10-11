use super::inventory;
use super::*;
use rstest::rstest;

fn imported(source: &str, module: &str) -> ExtractOutput {
    reset_class_map();
    reset_file_map();
    css::debug::set_debug(true);
    let module = module.to_string();
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./values").then(|| ResolvedModule {
            path: "/values.ts".to_string(),
            code: module.clone(),
        })
    };
    let actual = extract_with_modules("a.tsx", source, ExtractOption::default(), false, &resolver)
        .unwrap_or_else(|error| panic!("imported fixture fails: {error}"));
    css::debug::set_debug(false);
    assert_eq!(actual.dependencies, vec!["/values.ts".to_string()]);
    actual
}

#[rstest]
#[case::call("css({styleOrder:active?2:3,color:'red'})")]
#[case::tag("css`style-order:${active?2:3};color:red;`")]
#[serial]
fn w38r_u5_record_producer_when_nested_style_is_finite_folds_its_truthiness(
    #[case] producer: &str,
) {
    // Given: nested producers are evaluated as record properties, not const roots.
    let module = format!(
        "import {{css}} from '@devup-ui/react';let active=true;export const bundle={{base:{producer}}};"
    );
    let source = "import {css} from '@devup-ui/react';import {bundle} from './values';const a=css({color:bundle.base?'blue':'green'});";
    // When: module evaluation discovers the actual finite call/tag span.
    let actual = imported(source, &module);
    let evaluated = whole::evaluate_code(&actual.code, "a");
    // Then: every producer result is nonempty, so the consumer is statically blue.
    assert_eq!(evaluated.element, serde_json::json!("color-0-blue--255-a"));
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(&actual, &[("color", "blue", 0, None)]);
}

#[rstest]
#[case::empty("css({styleOrder:mark()?2:3})", "color-0-green--255-a", false)]
#[case::nonempty(
    "css({styleOrder:mark()?2:3,color:'red'})",
    "color-0-blue--255-a",
    true
)]
#[case::mixed(
    "css(mark()?{styleOrder:2,color:'red'}:null)",
    "color-0-blue--255-a",
    true
)]
#[serial]
fn w38r_u5_saved_producer_when_used_as_condition_respects_all_or_mixed_results(
    #[case] producer: &str,
    #[case] classes: &str,
    #[case] blue: bool,
) {
    // Given: the producer's saved primitive is distinct from its source condition.
    let module = format!(
        "import {{css}} from '@devup-ui/react';let active=true;const mark=()=>(trace.push('produce'),active);export const base={producer};active=false;"
    );
    let source = "import {css} from '@devup-ui/react';import {base} from './values';const a=css({color:base?'blue':'green'});";
    // When: the public module loader supplies the producer's finite alternatives.
    let actual = imported(source, &module);
    css::debug::set_debug(true);
    let compiled_module =
        extract_without_source_map("/values.ts", &module, ExtractOption::default())
            .required("real producer compiles");
    css::debug::set_debug(false);
    let evaluated =
        whole::evaluate_code(&format!("{}\n{}", compiled_module.code, actual.code), "a");
    // Then: source effects run once; only mixed emptiness requires a runtime choice.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!(["produce"]));
    let expected = if producer.contains(":null") {
        vec![("color", "blue", 0, None), ("color", "green", 0, None)]
    } else {
        vec![("color", if blue { "blue" } else { "green" }, 0, None)]
    };
    inventory(&actual, &expected);
}

#[rstest]
#[case::populated("{color:'red'}", "blue")]
#[case::empty("{}", "green")]
#[serial]
fn w38r_u5_known_style_when_finite_discovery_fails_uses_static_truthiness(
    #[case] rules: &str,
    #[case] color: &str,
) {
    // Given: unrelated invalid metadata prevents the module's finite origin table.
    let module = format!(
        "import {{css}} from '@devup-ui/react';export const base=css({rules});const invalid=css({{styleOrder:0}});"
    );
    let source = "import {css} from '@devup-ui/react';import {base} from './values';const a=css({color:base?'blue':'green'});";
    // When: the imported constant falls back to the readable static css inventory.
    let actual = imported(source, &module);
    let evaluated = whole::evaluate_code(&actual.code, "a");
    // Then: the known style's emptiness, not object truthiness, chooses the branch.
    assert_eq!(
        evaluated.element,
        serde_json::json!(format!("color-0-{color}--255-a"))
    );
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(&actual, &[("color", color, 0, None)]);
}
