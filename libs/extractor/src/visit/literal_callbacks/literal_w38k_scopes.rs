use super::literal_w38k_support::{GETTERS, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "style-order:${p=>{return;}};color:red;&:hover{style-order:3;color:blue}",
    "color:red;&:hover{style-order:3;color:blue}",
    vec![("blue", Some(3)), ("red", None)]
)]
#[case(
    "style-order:2;color:red;&:hover{style-order:3;style-order:${p=>{return;}};color:blue}",
    concat!("style-order:2;color:red;&:hover", "{", "color:blue", "}"),
    vec![("blue", Some(2)), ("red", Some(2))]
)]
#[serial]
fn local_absence_when_scopes_are_nested_preserves_outer_and_inner_orders(
    #[case] body: &str,
    #[case] omitted: &str,
    #[case] expected_orders: Vec<(&str, Option<u8>)>,
) {
    // Given: omission belongs to one declaration scope, not to its neighboring scope.
    let source = fixture(body, GETTERS);
    let control_source = fixture(omitted, GETTERS);
    // When: both independently sourced components are extracted and rendered.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let control = compile(&control_source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    let (control_classes, control_trace) = evaluate(&control.code);
    // Then: every emitted declaration and selected class matches scoped omission.
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(trace, control_trace);
    assert_eq!(tokens(&classes), tokens(&control_classes));
    assert_eq!(declarations(&output), declarations(&control));
    let styles = declarations(&output);
    let mut orders = styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => (style.value.as_str(), style.style_order),
            other => panic!("unexpected declaration: {other:?}"),
        })
        .collect::<Vec<_>>();
    orders.sort_unstable();
    assert_eq!(orders, expected_orders);
}

#[rstest]
#[case("${p=>({color:'red'})};")]
#[serial]
fn mixin_when_followed_by_absence_keeps_its_actual_emitted_declarations(#[case] mixin: &str) {
    // Given: actual callback and css() mixins, rather than manufactured carriers.
    let source = fixture(&format!("{mixin}style-order:${{p=>{{return;}}}};"), GETTERS)
        .replace("import {styled}", "import {styled,css}");
    let control_source = fixture(mixin, GETTERS).replace("import {styled}", "import {styled,css}");
    // When: production suppliers prepare the mixin and generated components run.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let control = compile(&control_source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    let (control_classes, control_trace) = evaluate(&control.code);
    // Then: the full declaration vector and exact selected classes retain the mixin.
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(trace, control_trace);
    assert_eq!(tokens(&classes), tokens(&control_classes));
    assert_eq!(declarations(&output), red(&[None]));
    assert_eq!(declarations(&output), declarations(&control));
}
