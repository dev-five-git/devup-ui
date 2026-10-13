use super::inventory;
use super::*;
use rstest::rstest;

#[rstest]
#[case::diagnostic(
    "{styleOrder:0,color:'red'}",
    "0",
    "`styleOrder()` cannot use `0` at build time: an explicit styleOrder must be an ECMAScript Number integer from 1 to 254 or canonical decimal string without signs, spaces or leading zeros"
)]
#[case::unreadable(
    "{_hover:runtimeRules}",
    "css({_hover:runtimeRules})",
    "`css()` cannot use `runtimeRules` at build time: its values must be literals, theme tokens or constants, or be computed from them"
)]
#[serial]
fn w38r_u5_closed_local_when_rules_retain_errors_reports_original_location(
    #[case] rules: &str,
    #[case] located: &str,
    #[case] message: &str,
) {
    // Given: closed local rules retain their errors through normalized emission.
    let line =
        format!("const render=()=> <ClassNames>{{({{css,cx}})=>css({rules})}}</ClassNames>;");
    let source = format!("import {{ClassNames}} from '@emotion/react';\n{line}");
    // When: the real public ClassNames pipeline observes those error roots.
    let actual = compile_emotion(&source)
        .err()
        .required("invalid local rules fail");
    // Then: diagnostics still name the authored consumer/value, not an emitted class.
    assert_eq!(
        actual,
        format!(
            "a.tsx:2:{}: {message}",
            line.find(located).required("authored location") + 1
        )
    );
}

#[rstest]
#[case::supplied("({styleOrder:mark('order',true)?2:3,typography:`head-${external} !important`})", "typo-head-title !important", &["type", "order"])]
#[case::template("`style-order:${mark('order',true)?2:3};typography:head-${external} !important;${'extra'}`", "extra typo-head-title !important", &["type", "order"])]
#[serial]
fn w38r_u5_equal_metadata_branches_when_typography_is_supplied_keep_payload_once(
    #[case] argument: &str,
    #[case] classes: &str,
    #[case] trace: &[&str],
) {
    // Given: order cannot alter an opaque typography payload, so sides are equal.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';const render=()=>{{const mark=(n,v)=>(trace.push(n),v);const external=mark('type','title');return <ClassNames>{{({{css,cx}})=>css{argument}}}</ClassNames>}};"
    );
    // When: extraction captures the original values and emits normalized branches.
    let actual = compile_emotion(&source).required("equal typography branches compile");
    let evaluated = whole::evaluate_code(&actual.code, "render()");
    // Then: equality keeps the supplied/template class; neither effect is replayed.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!(trace));
    inventory(&actual, &[]);
}

#[rstest]
#[case::array("[{id:'left'},{id:'right'}]", "[a.props[0].id,a.props[1].id]", serde_json::json!(["left","right"]), &[])]
#[case::yes("[mark(true)?{id:'left'}:{id:'right'}]", "a.props[0].id", serde_json::json!("left"), &["choose"])]
#[case::no("[mark(false)?{id:'left'}:{id:'right'}]", "a.props[0].id", serde_json::json!("right"), &["choose"])]
#[serial]
fn w38r_u5_runtime_jsx_spread_when_shape_is_array_or_conditional_preserves_dom_values(
    #[case] spread: &str,
    #[case] probe: &str,
    #[case] expected: serde_json::Value,
    #[case] trace: &[&str],
) {
    // Given: these spreads contain DOM props only, with no extractable declarations.
    let source = format!(
        "import {{Box}} from '@devup-ui/react';const mark=v=>(trace.push('choose'),v);const a=<Box {{...({spread})}}/>;"
    );
    // When: public extraction retains the runtime spread and recursively reads it.
    let actual = output(&source);
    let evaluated = whole::evaluate_code(&actual.code, probe);
    // Then: JS spread semantics and once-only branch selection are unchanged.
    assert_eq!(evaluated.element, expected);
    assert_eq!(evaluated.trace, serde_json::json!(trace));
    inventory(&actual, &[]);
}

#[rstest]
#[case::number("2", "{\"styleOrder\":2,\"margin\":\"3px\"}")]
#[case::undefined("undefined", "{\"styleOrder\":null,\"margin\":\"3px\"}")]
#[serial]
fn w38r_u5_vanilla_when_metadata_is_serialized_preserves_number_or_explicit_null(
    #[case] order: &str,
    #[case] expected: &str,
) {
    // Given: the actual stylesheet API serializes metadata and ordinary lengths.
    reset_file_map();
    let source = format!(
        "import {{style}} from '@devup-ui/react';export const card=style({{styleOrder:{order},margin:3}});"
    );
    // When: public stylesheet execution runs the native JSON replacer.
    let (actual, imports) = crate::vanilla_extract::execute_stylesheet(
        &source,
        "/card.css.ts",
        &ExtractOption::default(),
        None,
    )
    .unwrap_or_else(|error| panic!("stylesheet fails: {error}"));
    // Then: metadata is not pixelified or dropped, but the margin is a px length.
    let card = actual.styles.get("card").required("exported card style");
    assert_eq!(actual.styles.len(), 1);
    assert_eq!(card.json, expected);
    assert!(card.exported);
    assert_eq!(card.bases.to_vec(), Vec::<String>::new());
    assert_eq!(card.classes.to_vec(), Vec::<String>::new());
    assert_eq!(actual.global_styles, vec![]);
    assert_eq!(actual.constant_exports, vec![]);
    assert_eq!(imports.dependencies, std::collections::BTreeSet::new());
}
