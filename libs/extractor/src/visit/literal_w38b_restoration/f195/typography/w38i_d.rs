use super::*;
use crate::ExtractStyleProp::{Conditional, Expression as Payload, Static, StaticArray};
use crate::gen_class_name::gen_class_names;
use std::collections::BTreeSet;

#[rstest]
#[case("`body-${state.part}`", true, "1")]
#[case("`body-${state.part}`", false, "1")]
#[case("state.preset", true, "body-1")]
#[case("state.preset", false, "body-1")]
#[case("state.preset", true, "")]
#[case("state.preset", false, "")]
#[serial]
fn typography_when_order_is_conditional_preserves_payload_selection_and_reads(
    #[case] typography: &str,
    #[case] active: bool,
    #[case] preset: &str,
) {
    // Given: real parsed typography producers, registered keys and authored order spans.
    let _keys = TypographyKeys::registered();
    let _debug = DebugMode::enabled();
    let source = format!("({{styleOrder:state.active?2:3,typography:{typography},color:'red'}});");
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut value = parsed(&allocator, &source);
    let span = value.span();
    let Expression::ObjectExpression(object) = &value else {
        panic!("parsed declaration object")
    };
    let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        panic!("authored order property")
    };
    let Expression::ConditionalExpression(order) = &property.value else {
        panic!("authored conditional order")
    };
    let test_span = order.test.span();
    // When: the normal wrapper extracts, clones the alternate and converts real payloads.
    let handling = LiteralHandling::ExpandResponsiveThemeToken;
    let mut result = extract_style_from_expression(&ast, None, &mut value, 0, &None, handling);
    // Then: both orders retain the producer, declaration and source-owned condition span.
    assert_eq!(value.span(), span);
    let [
        Conditional {
            condition,
            consequent,
            alternate,
        },
    ] = result.styles.as_slice()
    else {
        panic!("conditional extracted rules")
    };
    assert_eq!(condition.span(), test_span);
    assert_eq!(readable_code(condition), "state.active");
    for (branch, order) in [(consequent, 2), (alternate, 3)] {
        let Some(StaticArray(props)) = branch.as_deref() else {
            panic!("order branch declarations")
        };
        let [Static(declaration), Payload { expression, styles }] = &**props else {
            panic!("red declaration and typography payload")
        };
        let ExtractStyleValue::Static(s) = declaration else {
            panic!("static red declaration")
        };
        assert_eq!((s.property(), s.value()), ("color", "red"));
        assert_eq!(
            (s.level(), s.selector(), s.style_order()),
            (0, None, Some(order))
        );
        assert_eq!(class_of(declaration), format!("color-0-red--{order}"));
        assert_eq!(styles, &Vec::<ExtractStyleValue>::new());
        match (typography, expression) {
            ("`body-${state.part}`", Expression::TemplateLiteral(_))
            | ("state.preset", Expression::CallExpression(_)) => {}
            _ => panic!("source typography producer must survive clone and conversion"),
        }
    }
    let emitted = gen_class_names(&ast, &mut result.styles, None, None)
        .unwrap_or_else(|| panic!("selected typography classes"));
    let read = match typography {
        "`body-${state.part}`" => "part",
        "state.preset" => "preset",
        _ => panic!("listed typography source"),
    };
    let setup = format!(
        "const state={{get active(){{trace.push('order');return {active};}},get {read}(){{trace.push('{read}');return '{preset}';}}}}"
    );
    let actual = evaluated(&emitted, &setup);
    let applied: BTreeSet<_> = actual[0]
        .as_str()
        .unwrap_or_else(|| panic!("selected class text"))
        .split_whitespace()
        .collect();
    let red = format!("color-0-red--{}", if active { 2 } else { 3 });
    let mut expected = BTreeSet::from([red.as_str()]);
    if !preset.is_empty() {
        expected.insert("typo-body-1");
    }
    assert_eq!(applied, expected);
    assert_eq!(actual[1], serde_json::json!(["order", read]));
}
