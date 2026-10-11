use super::inventory;
use super::*;
use crate::extract_style::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};
use rstest::rstest;

fn selected(actual: &ExtractOutput, classes: &str, expected: &[(&str, &str, u8)]) {
    let tokens: Vec<_> = classes.split_whitespace().collect();
    let found: Vec<_> = super::super::w38o_logical_source::declarations(
        &actual.styles.iter().cloned().collect::<Vec<_>>(),
        &tokens,
    );
    let mut authored: Vec<_> = expected
        .iter()
        .map(|(property, value, order)| {
            ExtractStyleValue::Static(ExtractStaticStyle {
                property: (*property).into(),
                value: (*value).into(),
                level: 0,
                selector: None,
                style_order: Some(*order),
                layer: None,
                theme_token_resolution: ThemeTokenResolution::CssVariable,
            })
        })
        .collect();
    authored.sort_unstable();
    assert_eq!(found, authored);
}

#[rstest]
#[case::void_coalesce("cx(void 0 ?? 'fallback')", "fallback", &[])]
#[case::void_or("cx(void 0 || 'fallback')", "fallback", &[])]
#[case::template_left("cx(`left` || 'right')", "left", &[])]
#[case::captured_template("cx(external, `constant`)", "ext constant", &[])]
#[serial]
fn w38r_u3_classes_when_literal_or_empty_sides_survive_capture_preserve_order(
    #[case] expression: &str,
    #[case] expected: &str,
    #[case] trace: &[&str],
) {
    // Given: ordinary render with real Emotion helpers and authored literal order.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';const render=external=>{{const mark=(n,v)=>(trace.push(n),v);return <ClassNames>{{({{css,cx}})=>{expression}}}</ClassNames>}};"
    );
    // When: extract publicly, then invoke the outer render once.
    let actual = compile_emotion(&source).required("literal sides must compile");
    let evaluated = whole::evaluate_code(&actual.code, "render('ext')");
    // Then: constant templates retain dynamic order; empty void chooses the fallback.
    assert_eq!(evaluated.element, serde_json::json!(expected));
    assert_eq!(evaluated.trace, serde_json::json!(trace));
    inventory(&actual, &[]);
}

#[rstest]
#[case::finite_fallback(true, "", " color-0-red--2-a")]
#[case::empty_finite_fallback(false, "", " ")]
#[case::external_wins(true, "external", "external ")]
#[serial]
fn w38r_u3_finite_right_when_external_falls_back_selects_saved_producer(
    #[case] active: bool,
    #[case] external: &str,
    #[case] expected: &str,
) {
    // Given: a saved finite producer, evaluated once in the original render scope.
    let source = "import {ClassNames} from '@emotion/react';import {css as makeCss} from '@devup-ui/react';const render=(active,external)=>{const saved=makeCss((trace.push('produce'),active)?{styleOrder:2,color:'red'}:null);return <ClassNames>{({css,cx})=>cx(external||saved)}</ClassNames>};";
    // When: extract and evaluate the chosen external/finite side.
    let actual = compile_emotion(source).required("finite fallback must compile");
    let evaluated = whole::evaluate_code(
        &actual.code,
        &format!("render({active},{})", serde_json::json!(external)),
    );
    // Then: no condition re-execution; only the selected finite declarations survive.
    assert_eq!(evaluated.element, serde_json::json!(expected));
    assert_eq!(evaluated.trace, serde_json::json!(["produce"]));
    inventory(&actual, &[("color", "red", 0, Some(2))]);
    let declarations = if active && external.is_empty() {
        vec![("color", "red", 2)]
    } else {
        vec![]
    };
    selected(&actual, expected, &declarations);
}

#[rstest]
#[case::yes(true, "color-0-blue--2-a background-0-black--1-a")]
#[case::no(false, "color-0-blue--2-a background-0-white--1-a")]
#[serial]
fn w38r_u3_array_when_saved_finite_elements_are_composed_preserves_index_and_override(
    #[case] active: bool,
    #[case] expected: &str,
) {
    // Given: two actual finite class producers and a later same-layer override.
    let source = "import {ClassNames} from '@emotion/react';import {css as makeCss} from '@devup-ui/react';const render=active=>{const mark=(n,v)=>(trace.push(n),v);const parts=[makeCss(mark('color',active)?{styleOrder:2,color:'red'}:{styleOrder:2,color:'green'}),makeCss(mark('background',active)?{styleOrder:1,background:'black'}:{styleOrder:1,background:'white'})];return <ClassNames>{({css,cx})=>cx(parts,css({styleOrder:2,color:'blue'}))}</ClassNames>};";
    // When: extract and render without re-evaluating any producer condition.
    let actual = compile_emotion(source).required("saved finite array must compile");
    let evaluated = whole::evaluate_code(&actual.code, &format!("render({active})"));
    // Then: indexed finite values select independently; later color replaces earlier.
    assert_eq!(evaluated.element, serde_json::json!(expected));
    assert_eq!(evaluated.trace, serde_json::json!(["color", "background"]));
    inventory(
        &actual,
        &[
            ("color", "red", 0, Some(2)),
            ("color", "green", 0, Some(2)),
            ("color", "blue", 0, Some(2)),
            ("background", "black", 0, Some(1)),
            ("background", "white", 0, Some(1)),
        ],
    );
    selected(
        &actual,
        expected,
        &[
            ("color", "blue", 2),
            ("background", if active { "black" } else { "white" }, 1),
        ],
    );
}

#[rstest]
#[case::style_left(
    "css([{color:'red'}]||{color:'blue'},42)",
    "css([{ color: \"red\" }] || { color: \"blue\" }, 42)"
)]
#[case::boolean_coalesce("cx(false??'ignored',42)", "cx(false ?? \"ignored\", 42)")]
#[case::string_left("cx('left'||'right',42)", "cx(\"left\" || \"right\", 42)")]
#[case::conditional_left(
    "cx([active&&'left']||'right',42)",
    "cx([active && \"left\"] || \"right\", 42)"
)]
#[case::guarded_styles(
    "css([active&&{color:'red'}]||{color:'blue'})",
    "css([active && { color: \"red\" }] || { color: \"blue\" })"
)]
#[case::mixed_left(
    "cx(['external',saved]||'fallback')",
    "cx([\"external\", saved] || \"fallback\")"
)]
#[serial]
fn w38r_u3_original_parts_when_preflight_rejects_capture_report_exact_consumer(
    #[case] expression: &str,
    #[case] readable: &str,
) {
    // Given: an unsupported original shape keeps the whole call out of capture.
    let prefix = "const render=(external,active)=><ClassNames>{({css,cx})=>";
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';import {{css as makeCss}} from '@devup-ui/react';const saved=makeCss({{styleOrder:2,color:'red'}});\n{prefix}{expression}}}</ClassNames>;"
    );
    // When: the public extractor diagnoses the original composition call.
    let actual = compile_emotion(&source)
        .err()
        .required("unsupported composition must fail");
    // Then: pin original location, authored readable syntax and exact requirement.
    assert_eq!(
        actual,
        format!(
            "a.tsx:2:{}: `<ClassNames>` cannot use `{readable}` at build time: `css` and `cx` compose only style objects, CSS text, classes, calls of them, or arrays or conditions of these",
            prefix.len() + 1,
        ),
    );
}

#[rstest]
#[case::template("css({typography:`head-${mark('typography',external)} !important`})")]
#[serial]
fn w38r_u3_typography_when_object_capture_preserves_important_template_emits_class(
    #[case] expression: &str,
) {
    // Given: capture preserves an important suffix as a template around its saved value.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';const render=external=>{{const mark=(n,v)=>(trace.push(n),v);return <ClassNames>{{({{css,cx}})=>{expression}}}</ClassNames>}};"
    );
    // When: the template-valued object property reaches local RuleClass conversion.
    let actual = compile_emotion(&source).required("important typography template must compile");
    let evaluated = whole::evaluate_code(&actual.code, "render('title')");
    // Then: typography prefixes the preserved template and evaluates its value once.
    assert_eq!(
        evaluated.element,
        serde_json::json!("typo-head-title !important")
    );
    assert_eq!(evaluated.trace, serde_json::json!(["typography"]));
    inventory(&actual, &[]);
}
