use super::{code, expression};
use crate::{ExtractStyleProp, ExtractStyleValue, assignment_test_support::evaluate};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, SPAN};

#[test]
fn component_capture_orders_evaluations_inside_each_render() {
    // Given: consumers are stored in reverse order, while spans retain authored order.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut component = expression(&allocator, "()=>({value:first+second})");
    let mut styles =
        [("second", 20), ("first", 10)].map(|(name, start)| ExtractStyleProp::Evaluated {
            styles: vec![],
            source: expression(&allocator, "input"),
            binding: name.into(),
            evaluation: Some(oxc_ast::ast::Expression::new_call_expression(
                oxc_span::Span::new(start, start + 1),
                expression(&allocator, name),
                None,
                oxc_allocator::Vec::new_in(&ast),
                false,
                &ast,
            )),
            alternate_order: None,
            alternate_class: false,
        });
    // When
    crate::assignment_capture::component(&ast, &mut component, &mut styles);
    let actual = evaluate(&format!(
        "let trace=[];function first(){{trace.push('first');return 2}}function second(){{trace.push('second');return 3}}const render={};const before=trace.length;const a=render(),b=render();JSON.stringify([before,trace,a.value,b.value]);",
        code(&component)
    ));
    // Then
    assert_eq!(
        actual,
        "[0,[\"first\",\"second\",\"first\",\"second\"],5,5]"
    );
    assert!(styles.iter().all(|style| matches!(
        style,
        ExtractStyleProp::Evaluated {
            evaluation: None,
            ..
        }
    )));
}

#[test]
fn class_capture_leaves_missing_variants_untouched() {
    // Given
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "state.value");
    let mut variants = [None, Some(expression(&allocator, "('alternate')"))];
    // When
    let captured = crate::class_evaluation::capture(&ast, &source, &mut variants);
    // Then
    assert!(captured.is_none());
    assert!(variants[0].is_none());
    assert_eq!(
        crate::utils::get_string_by_literal_expression(
            variants[1]
                .as_ref()
                .unwrap_or_else(|| panic!("alternate retained"))
        )
        .as_deref(),
        Some("alternate")
    );
}

#[test]
fn constant_typography_class_still_evaluates_its_authored_operand_once() {
    // Given: precomputed class data must not erase observable source evaluation.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "state.value");
    let mut styles = vec![ExtractStyleProp::Static(ExtractStyleValue::Typography(
        "heading".into(),
    ))];
    let mut next = 0;
    // When
    let (_, capture) = crate::element_evaluation::typography(&ast, &mut styles, &source, &mut next)
        .unwrap_or_else(|| panic!("typography capture"));
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{reads++;return 'heading'}}}};const value={};JSON.stringify([reads,value]);",
        code(&capture)
    ));
    // Then
    assert_eq!(actual, "[1,\"typo-heading\"]");
    assert_eq!(capture.span(), source.span());
}

#[test]
fn synthesized_attribute_capture_uses_the_authored_attribute_coordinate() {
    // Given
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut source = expression(&allocator, "<Box id={state.id} p={state.pad}/>");
    let oxc_ast::ast::Expression::JSXElement(element) = &mut source else {
        panic!("element fixture")
    };
    let oxc_ast::ast::JSXAttributeItem::Attribute(attribute) =
        &mut element.opening_element.attributes[0]
    else {
        panic!("attribute fixture")
    };
    let span = attribute.span;
    *crate::visit::attribute_value_mut(attribute).unwrap_or_else(|| panic!("id value")) =
        oxc_ast::ast::Expression::new_identifier(SPAN, "synthesized", &ast);
    // When
    let values = crate::element_evaluation::capture(&ast, element, &mut 0);
    // Then: scheduling gets the original id position rather than the synthetic zero.
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].1.span(), span);
    assert_eq!(code(&values[0].1), "synthesized");
}

#[test]
fn creation_capture_does_not_rewrite_an_authored_callback_body() {
    // Given
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "({color:state.value})");
    let mut component = expression(&allocator, "()=>state.value");
    // When
    crate::assignment_capture::styled_creation(
        &ast,
        &mut component,
        crate::assignment_capture::StyledCreation {
            source: &source,
            styles: &mut [],
        },
    );
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{return ++reads}}}};const render={};const created=reads;const first=render(),second=render();JSON.stringify([created,first,second,reads]);",
        code(&component)
    ));
    // Then: creating the literal and invoking the independent callback are separate reads.
    assert_eq!(actual, "[1,2,3,3]");
}
