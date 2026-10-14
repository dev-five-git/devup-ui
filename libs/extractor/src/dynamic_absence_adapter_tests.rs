use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(("{p:read()}", "null", "[]"))]
#[case(("{p:read()}", "4", "[\"padding:16px:0\"]"))]
#[case(("{_hover:{p:read()}}", "null", "[]"))]
#[case(("{p:[read(),null,'8px']}", "null", "[\"padding:8px:2\"]"))]
#[case(("true?{p:read()}:{p:8}", "null", "[]"))]
#[case(("{p:`${read()} !important`}", "''", "[]"))]
#[serial]
fn styled_object_presence_is_created_once_when_component_renders_twice(
    #[case] fixture: (&str, &str, &str),
) {
    // Given: object values run at creation, before either component render.
    let (object, value, expected) = fixture;
    let source = format!(
        "import {{styled}}from '@devup-ui/react';let reads=0;function read(){{reads++;return reads===1?{value}:8}}const A=styled.div({object});const created=reads;const first=A({{}});const second=A({{}});"
    );
    // When: both generated components reuse the creation tuple.
    let actual = selected(
        &source,
        "JSON.stringify([created,reads,assigned(first),assigned(second)]);",
    );
    // Then: presence and conversion neither move to render nor reread their source.
    assert_eq!(actual, format!("[1,1,{expected},{expected}]"));
}

#[rstest]
#[case("{p:void read()}")]
#[case("{p:test()?null:null}")]
#[serial]
fn styled_creation_effect_survives_when_all_styles_are_absent(#[case] object: &str) {
    // Given: no CSS records does not prove a creation expression is pure.
    let source = format!(
        "import {{styled}}from '@devup-ui/react';let reads=0;function read(){{reads++;return 4}}function test(){{reads++;return true}}const A=styled.div({object});const created=reads;const first=A({{}});const second=A({{}});JSON.stringify([created,reads,String(first.className??'').trim(),String(second.className??'').trim()]);"
    );
    // When: the empty-effect tuple executes at creation.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: the source runs once, while neither render selects a class.
    assert_eq!(actual, "[1,1,\"\",\"\"]");
}

#[test]
#[serial]
fn css_text_adapter_keeps_dynamic_class_when_value_is_absent() {
    // Given: CSS-text interpolation is outside ordinary property presence policy.
    let source = "import {styled}from '@devup-ui/react';let reads=0;function read(){reads++;return null}const A=styled.div`padding:${read()};color:red;`;const node=A({});";
    // When: the existing tagged-template render adapter executes.
    let actual = selected(source, "JSON.stringify([reads,selected(node).length]);");
    // Then: no incidental ordinary guard erases its dynamic declaration.
    assert_eq!(actual, "[1,2]");
}

#[test]
#[serial]
fn stylex_namespace_keeps_classes_when_dynamic_argument_is_absent() {
    // Given: one namespace aggregates a static and a dynamic declaration.
    let source = "import stylex from '@stylexjs/stylex';let reads=0;function read(){reads++;return null}const styles=stylex.create({dynamic:value=>({opacity:value,color:'red'})});const node=stylex.props(styles.dynamic(read()));";
    // When: the real StyleX call adapter executes its argument capture.
    let actual = selected(source, "JSON.stringify([reads,selected(node).length]);");
    // Then: default-off policy preserves the existing namespace selection.
    assert_eq!(actual, "[1,2]");
}

#[rstest]
#[case("undefined")]
#[case("NaN")]
#[case("Infinity")]
#[serial]
fn shadowed_ignored_name_is_inlined_when_binding_has_an_active_literal(#[case] name: &str) {
    // Given: ignored spelling alone is not an absence proof about a binding.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(){{const {name}='8px';return <Box p={{{name}}}/>}}const node=render();"
    );
    // When: the existing binding inliner resolves the shadowed name.
    let actual = selected(&source, "JSON.stringify(assigned(node));");
    // Then: the bound value stays active rather than using the ignored-name heuristic.
    assert_eq!(actual, "[\"padding:8px:0\"]");
}

#[rstest]
#[case("undefined")]
#[case("NaN")]
#[case("Infinity")]
#[case("undefined || '12px'")]
#[case("undefined ?? '12px'")]
#[serial]
fn shadowed_ignored_parameter_keeps_style_when_argument_is_active(#[case] expression: &str) {
    // Given: the parameter is bound even though its spelling names a global.
    let name = expression
        .split_whitespace()
        .next()
        .unwrap_or_else(|| panic!("fixture must contain a parameter name"));
    let source = format!(
        "import {{Box}}from '@devup-ui/react';let reads=0;function read(){{reads++;return '8px'}}function render({name}){{return <Box p={{{expression}}}/>}}const node=render(read());"
    );
    // When: ordinary presence and conversion consume the parameter value.
    let actual = selected(&source, "JSON.stringify([reads,assigned(node)]);");
    // Then: the active declaration survives and the source executes once.
    assert_eq!(actual, "[1,[\"padding:8px:0\"]]");
}

#[rstest]
#[case("", "undefined")]
#[case("", "NaN")]
#[case("", "Infinity")]
#[case("const absent=undefined;", "absent")]
#[case("const absent=0/0;", "absent")]
#[serial]
fn global_ignored_value_omits_style_when_unbound_or_inlined(
    #[case] setup: &str,
    #[case] value: &str,
) {
    // Given: unbound globals retain the ordinary no-style contract.
    let source =
        format!("import {{Box}}from '@devup-ui/react';{setup}const node=<Box p={{{value}}}/>;");
    // When: extraction encounters a global or a generated literal identifier.
    let actual = selected(&source, "JSON.stringify(assigned(node));");
    // Then: no dynamic or static declaration is selected.
    assert_eq!(actual, "[]");
}
