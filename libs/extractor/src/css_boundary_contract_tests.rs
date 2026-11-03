use super::*;
use crate::extract_style::extract_dynamic_style::ExtractDynamicStyle;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

fn dynamic(name: &str) -> ExtractStyleProp<'_> {
    ExtractStyleProp::Static(ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
        "color", 0, name, None,
    )))
}

#[rstest]
#[case(true, vec!["a", "c", "b"])]
#[case(false, vec!["b", "c", "a"])]
fn nested_style_slots_follow_the_same_alternating_order_as_emission(
    #[case] reversed: bool,
    #[case] expected: Vec<&str>,
) {
    let styles = ExtractStyleProp::StaticArray(vec![
        dynamic("a"),
        ExtractStyleProp::StaticArray(vec![dynamic("b"), dynamic("c")]),
    ]);
    let mut slots = Vec::new();
    assert!(style_slots(&styles, reversed, &mut slots));
    assert_eq!(slots, expected);
}

#[rstest]
#[case("<div/>", "owned", false)]
#[case("<div xml:lang=\"en\"/>", "read()", false)]
#[case("<div ref=\"kept\"/>", "read()", false)]
#[case("<div/>", "read()", true)]
fn inline_refusals_preserve_the_element_and_owned_evaluation(
    #[case] source: &str,
    #[case] capture: &str,
    #[case] malformed: bool,
) {
    let arena = Allocator::default();
    let visitor = DevupVisitor::new(&arena, "boundary.tsx", "@devup-ui/react", vec![], None);
    let mut element = Parser::new(&arena, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let before = crate::utils::readable_code(&element);
    let value = Parser::new(&arena, capture, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut order = AttributeOrder {
        values: vec![("owned".into(), Some(value))],
        ..AttributeOrder::default()
    };
    let mut props = if malformed {
        vec![dynamic(")")]
    } else {
        Vec::new()
    };
    let mut raw = Vec::new();
    let Expression::JSXElement(jsx) = &mut element else {
        panic!("expected element")
    };
    assert!(
        visitor
            .inline_css_reads(jsx, &mut props, &mut raw, &mut order, 0)
            .is_none()
    );
    assert_eq!(crate::utils::readable_code(&element), before);
    assert_eq!(order.values.len(), 1);
    assert_eq!(order.values[0].0, "owned");
    assert_eq!(
        crate::utils::readable_code(
            order.values[0]
                .1
                .as_ref()
                .unwrap_or_else(|| panic!("captured value disappeared"))
        ),
        capture
    );
    assert!(raw.is_empty());
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
    if malformed {
        assert!(
            matches!(&props[0], ExtractStyleProp::Static(ExtractStyleValue::Dynamic(value)) if value.identifier() == ")")
        );
    }
}

#[rstest]
#[case("<div xml:lang=\"en\"/>")]
#[case("<div style={external}/>")]
fn style_object_capture_refuses_unowned_attribute_shapes(#[case] source: &str) {
    let arena = Allocator::default();
    let visitor = DevupVisitor::new(&arena, "boundary.tsx", "@devup-ui/react", vec![], None);
    let mut element = Parser::new(&arena, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let before = crate::utils::readable_code(&element);
    let value = Parser::new(&arena, "read()", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut values = vec![("owned".into(), value)];
    let Expression::JSXElement(jsx) = &mut element else {
        panic!("expected element")
    };
    visitor.capture_in_style(jsx, &mut values);
    assert_eq!(crate::utils::readable_code(&element), before);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].0, "owned");
    assert_eq!(crate::utils::readable_code(&values[0].1), "read()");
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
}
