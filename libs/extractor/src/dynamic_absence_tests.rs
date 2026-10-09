use crate::assignment_blocker_support::selected;
use rstest::rstest;
use serial_test::serial;

#[path = "dynamic_absence_adapter_tests.rs"]
mod adapter;
#[path = "dynamic_absence_composite_tests.rs"]
mod composite;
#[path = "dynamic_absence_order_tests.rs"]
mod order;

#[rstest]
#[case("null", "[]")]
#[case("void 0", "[]")]
#[case("false", "[]")]
#[case("true", "[]")]
#[case("''", "[]")]
#[case("0/0", "[]")]
#[case("1/0", "[]")]
#[case("-1/0", "[]")]
#[case("0", "[\"padding:0px:0\"]")]
#[case("-0", "[\"padding:0px:0\"]")]
#[case("4", "[\"padding:16px:0\"]")]
#[case("-0.5", "[\"padding:-2px:0\"]")]
#[case("'10px'", "[\"padding:10px:0\"]")]
#[case("'NaN'", "[\"padding:NaNpx:0\"]")]
#[case("'Infinity'", "[\"padding:Infinitypx:0\"]")]
#[case("' '", "[\"padding: :0\"]")]
#[case("'inherit'", "[\"padding:inherit:0\"]")]
#[serial]
fn ordinary_class_uses_first_raw_read_when_getter_changes(
    #[case] value: &str,
    #[case] expected: &str,
) {
    // Given: a second read would return a different, active declaration.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box p={{state.value}}/>}}let reads=0;const state={{get value(){{reads++;return reads===1?{value}:8}}}};const node=render(state);"
    );
    // When: transformed classes and converted variables execute together.
    let actual = selected(&source, "JSON.stringify([reads,assigned(node)]);");
    // Then: absence and active conversion both refer to the first value.
    assert_eq!(actual, format!("[1,{expected}]"));
}

#[rstest]
#[case(("p", "+read()", "4", "[\"padding:4:0\"]"))]
#[case(("p", "`${read()}`", "'4'", "[\"padding:16px:0\"]"))]
#[case(("p", "`${read()}px`", "10", "[\"padding:10px:0\"]"))]
#[case(("opacity", "read()", "0", "[\"opacity:0:0\"]"))]
#[case(("p", "+read()", "0/0", "[]"))]
#[case(("p", "`${read()}`", "''", "[]"))]
#[case(("--x", "read()", "''", "[\"--x::0\"]"))]
#[case(("--x", "read()", "false", "[]"))]
#[case(("--x", "read()", "true", "[]"))]
#[case(("--x", "read()", "null", "[]"))]
#[case(("--x", "read()", "1/0", "[]"))]
#[serial]
fn conversion_metadata_stays_independent_when_raw_presence_is_selected(
    #[case] fixture: (&str, &str, &str, &str),
) {
    // Given: type facts choose existing calc, string, Keep or unitless conversion.
    let (property, expression, value, expected) = fixture;
    let source = format!(
        "import {{Box}}from '@devup-ui/react';let reads=0;function read(){{reads++;return {value}}}function render(){{return <Box {{...{{'{property}':{expression}}}}}/>}}const node=render();"
    );
    // When: selected declarations execute through the ordinary props path.
    let actual = selected(&source, "JSON.stringify([reads,assigned(node)]);");
    // Then: the guard sees raw values, not a converted string's truthiness.
    assert_eq!(actual, format!("[1,{expected}]"));
}

#[rstest]
#[case("true")]
#[case("false")]
#[case("null")]
#[case("undefined")]
#[case("NaN")]
#[case("Infinity")]
#[case("-Infinity")]
#[case("1e999")]
#[case("-1e999")]
#[case("0/0")]
#[case("1/0")]
#[case("''")]
#[case("` !important`")]
#[case("`;`")]
#[serial]
fn ordinary_static_class_is_omitted_when_value_has_no_style(#[case] value: &str) {
    // Given: ordinary static syntax or arithmetic with a no-style result.
    let source = format!("import {{Box}}from '@devup-ui/react';const node=<Box p={{{value}}}/>;");
    // When: the generated props select declaration classes.
    let actual = selected(&source, "JSON.stringify(selected(node));");
    // Then: no empty or nonfinite padding declaration is selected.
    assert_eq!(actual, "[]");
}

#[test]
#[serial]
fn custom_empty_static_declaration_survives_when_ordinary_empty_drops() {
    // Given: custom empty declarations have a different CSS meaning.
    let source = "import {Box}from '@devup-ui/react';const node=<Box {...{'--x':'',p:''}}/>;";
    // When: actual extracted declarations are selected.
    let actual = selected(source, "JSON.stringify(selected(node));");
    // Then: custom empty remains; this does not certify React variable transport.
    assert_eq!(actual, "[\"--x::0\"]");
}

#[rstest]
#[case("'NaN'")]
#[case("'Infinity'")]
#[case("'-Infinity'")]
#[serial]
fn ordinary_static_string_keeps_class_when_text_names_a_nonfinite_number(#[case] value: &str) {
    // Given: keyword text is not a numeric primitive absence proof.
    let source = format!("import {{Box}}from '@devup-ui/react';const node=<Box p={{{value}}}/>;");
    // When: the existing static normalizer admits the nonempty string.
    let actual = selected(&source, "JSON.stringify(selected(node).length);");
    // Then: absence selection does not introduce a string keyword blacklist.
    assert_eq!(actual, "1");
}
