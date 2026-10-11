use super::*;

#[rstest]
#[case(
    "cx(external)",
    "Object.defineProperty(globalThis,'external',{get(){trace.push('identifier');return 'external-a';}})",
    serde_json::json!(["external-a", ["identifier"]])
)]
#[case(
    "cx(state.className)",
    "const state={get className(){trace.push('member');return 'external-b';}}",
    serde_json::json!(["external-b", ["member"]])
)]
#[case(
    "cx(state[key])",
    "const key={toString(){trace.push('key');return 'className';}};const state={get className(){trace.push('member');return 'external-c';}}",
    serde_json::json!(["external-c", ["key", "member"]])
)]
#[case(
    "cx(({a:'external-a',b:'external-b'})[state.key])",
    "const state={get key(){trace.push('selection');return 'b';}}",
    serde_json::json!(["external-b", ["selection"]])
)]
#[serial]
fn local_class_when_source_is_external_preserves_value_and_one_read(
    #[case] call: &str,
    #[case] setup: &str,
    #[case] expected: serde_json::Value,
) {
    // Given: actual bindings, semantic references and a distinguishable source read.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';<ClassNames>{{({{css,cx}})=>{call}}}</ClassNames>;"
    );
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, &source);
    let span = value.span();
    // When: the original local compiler supplies and wraps the real capture.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: selected classes, original outer span and read trace survive.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    assert_eq!(evaluated(&value, setup), expected);
}

#[rstest]
#[case("css({color:'red'})", "color", "red")]
#[case("css({p:1===1?1:'4px'})", "padding", "4px")]
#[serial]
fn local_rule_when_static_or_equal_retains_its_selected_declaration(
    #[case] call: &str,
    #[case] property: &str,
    #[case] expected: &str,
) {
    // Given: a parsed static rule or equal-value conditional, with debug naming enabled.
    let _debug = DebugMode::enabled();
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';<ClassNames>{{({{css,cx}})=>{call}}}</ClassNames>;"
    );
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, &source);
    let span = value.span();
    // When: real local rule preparation emits the selected class.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: independent declaration semantics determine the applied token.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let style = visitor.styles.iter().find(|style| {
        matches!(style, ExtractStyleValue::Static(style) if style.property() == property && style.value() == expected)
    }).unwrap_or_else(|| panic!("selected declaration"));
    assert_eq!(
        evaluated(&value, ""),
        serde_json::json!([class_of(style), []])
    );
}

#[test]
#[serial]
fn literal_mixin_when_known_keeps_root_and_nested_orders_distinct() {
    // Given: the existing public mixin fixture, supplied through real local helpers.
    let _debug = DebugMode::enabled();
    let source = "import {ClassNames} from '@emotion/react';import {css as make} from '@devup-ui/react';const base=make({backgroundColor:'blue'});<ClassNames>{({css,cx})=>css`${base};&:hover{style-order:3;color:red}`}</ClassNames>;";
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, source);
    let span = value.span();
    // When: the original local literal composition consumes the saved producer.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: ordinary blue and ordered hover-red survive, without runtime producer replay.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let blue = visitor.styles.iter().find(|style| {
        matches!(style, ExtractStyleValue::Static(style) if style.value() == "blue" && style.style_order() != Some(3) && style.selector().is_none())
    }).unwrap_or_else(|| panic!("ordinary blue"));
    let red = visitor.styles.iter().find(|style| {
        matches!(style, ExtractStyleValue::Static(style) if style.value() == "red" && style.style_order() == Some(3) && style.selector().is_some())
    }).unwrap_or_else(|| panic!("ordered nested red"));
    let actual = evaluated(&value, "");
    let tokens: std::collections::BTreeSet<_> = actual[0]
        .as_str()
        .unwrap_or_else(|| panic!("class"))
        .split_whitespace()
        .collect();
    let expected = [class_of(blue), class_of(red)];
    assert_eq!(tokens, expected.iter().map(String::as_str).collect());
    assert_eq!(actual[1], serde_json::json!([]));
}
