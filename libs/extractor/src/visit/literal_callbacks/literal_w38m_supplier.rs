use super::literal_w38m_source::parsed;
use super::literal_w38m_support::rules;
use crate::utils::readable_code;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_span::GetSpan;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>p.stop?2:3", vec![Some(2), Some(3)])]
#[case("p=>{return;}", vec![None])]
#[serial]
fn supplier_when_direct_styled_source_is_unlowered_prepares_nested_callback(
    #[case] callback: &str,
    #[case] orders: Vec<Option<u8>>,
) {
    // Given: the actual parsed direct call, not a pre-lowered object or seeded registry.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';styled('div',{{...{{color:'red'}},_hover:`style-order:${{{callback}}};color:blue`}});"
    );
    let allocator = Allocator::default();
    let (mut visitor, mut expression) = parsed(&allocator, &source);
    let Expression::CallExpression(call) = &expression else {
        panic!("direct source call")
    };
    let Some(Expression::ObjectExpression(object)) = call.arguments[1].as_expression() else {
        panic!("authored object")
    };
    let ObjectPropertyKind::SpreadProperty(spread) = &object.properties[0] else {
        panic!("real source spread")
    };
    let spread_span = spread.span;
    let ObjectPropertyKind::ObjectProperty(hover) = &object.properties[1] else {
        panic!("authored hover")
    };
    let Expression::TemplateLiteral(template) = &hover.value else {
        panic!("unlowered source template")
    };
    let callback_span = template.expressions[0].span();
    assert_eq!(hover.key.static_name().as_deref(), Some("_hover"));
    let Expression::ObjectExpression(rest) = &spread.argument else {
        panic!("inline source record")
    };
    assert_eq!(rest.properties.len(), 1);
    let start = usize::try_from(spread_span.start).unwrap_or_else(|error| panic!("{error}"));
    let end = usize::try_from(spread_span.end).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(&source[start..end], "...{color:'red'}");
    let mut captures = Vec::new();
    // When: the production styled supplier itself performs lowering and capture.
    let prepared = visitor
        .prepare_styled_parts(&mut expression, &mut captures, &mut [])
        .unwrap_or_else(|| panic!("supplier rejected: {:?}", visitor.errors));
    // Then: source-derived styles and one captured callback/props invocation result.
    assert_eq!(visitor.errors, vec![]);
    let mut styles = prepared
        .styles
        .iter()
        .flat_map(crate::ExtractStyleProp::extract)
        .collect::<Vec<_>>();
    styles.sort_unstable();
    assert_eq!(styles, rules("red", &orders));
    assert_eq!(captures.len(), 1);
    assert_eq!(captures[0].1.span(), callback_span);
    assert_eq!(prepared.renders.len(), 1);
    let Expression::CallExpression(invocation) = &prepared.renders[0].1 else {
        panic!("one render invocation")
    };
    assert_eq!(readable_code(&invocation.callee), captures[0].0);
    assert_eq!(invocation.arguments.len(), 1);
    let props = invocation.arguments[0]
        .as_expression()
        .unwrap_or_else(|| panic!("props"));
    assert_eq!(readable_code(props), "__devupStyleProps");
    let Expression::CallExpression(call) = &expression else {
        panic!("prepared direct call")
    };
    assert_eq!(call.arguments.len(), 2);
    assert!(matches!(
        call.arguments[0].as_expression(),
        Some(Expression::StringLiteral(tag)) if tag.value == "div"
    ));
    let Some(Expression::ObjectExpression(placeholder)) = call.arguments[1].as_expression() else {
        panic!("prepared placeholder")
    };
    assert_eq!(placeholder.properties.len(), 0);
}
