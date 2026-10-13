use super::literal_w38m_source::parsed;
use crate::utils::{readable_code, unwrap_syntax_only_mut};
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_span::GetSpan;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>p.stop?2:3", 2)]
#[case("p=>{return;}", 1)]
#[serial]
fn direct_capture_when_real_spread_is_parsed_keeps_identity_and_render_placement(
    #[case] callback: &str,
    #[case] inner_count: usize,
) {
    // Given: real source spans and semantic reference IDs survive the parser.
    let source = format!(
        "const rest={{color:'red'}};({{...rest,_hover:`style-order:${{{callback}}};color:blue`}});"
    );
    let allocator = Allocator::default();
    let (mut visitor, mut expression) = parsed(&allocator, &source);
    let value = unwrap_syntax_only_mut(&mut expression);
    let Expression::ObjectExpression(root) = &*value else {
        panic!("mixed source root")
    };
    let ObjectPropertyKind::SpreadProperty(spread) = &root.properties[0] else {
        panic!("authored spread")
    };
    let span = spread.span;
    let argument_span = spread.argument.span();
    let Expression::Identifier(rest) = &spread.argument else {
        panic!("source rest reference")
    };
    let reference = rest
        .reference_id
        .get()
        .unwrap_or_else(|| panic!("source reference ID"));
    let mut captures = Vec::new();
    let mut renders = Vec::new();
    // When: the real recursive lowerer and preservation consumer run, in that order.
    assert!(crate::css_utils::literal_tree::lower(
        &visitor.ast,
        value,
        crate::css_utils::literal_tree::Scope {
            source: Some(&source),
            global: false
        }
    ));
    let Expression::ObjectExpression(lowered) = &*value else {
        panic!("lowered mixed root")
    };
    let ObjectPropertyKind::ObjectProperty(hover) = &lowered.properties[1] else {
        panic!("lowered hover property")
    };
    let Expression::ObjectExpression(inner) = &hover.value else {
        panic!("literal supplier intermediate")
    };
    let ObjectPropertyKind::ObjectProperty(order) = &inner.properties[0] else {
        panic!("generated metadata")
    };
    assert_eq!(order.key.static_name().as_deref(), Some("style-order"));
    let generated_callback = matches!(&order.value, Expression::ArrowFunctionExpression(_));
    assert!(generated_callback);
    assert!(visitor.capture_literal_styled(value, &mut captures, &mut renders));
    visitor.check_style_orders(value, false);
    // Then: spread identity is unchanged and the callback is not a construction value.
    assert_eq!(visitor.errors, vec![]);
    assert_eq!(captures.len(), 1);
    assert_eq!(renders.len(), 1);
    let Expression::ObjectExpression(root) = value else {
        panic!("prepared mixed root")
    };
    let ObjectPropertyKind::SpreadProperty(spread) = &root.properties[0] else {
        panic!("retained source spread")
    };
    assert_eq!(spread.span, span);
    assert_eq!(spread.argument.span(), argument_span);
    assert_eq!(readable_code(&spread.argument), "rest");
    let Expression::Identifier(rest) = &spread.argument else {
        panic!("retained reference")
    };
    assert_eq!(rest.reference_id.get(), Some(reference));
    let ObjectPropertyKind::ObjectProperty(hover) = &root.properties[1] else {
        panic!("prepared hover")
    };
    let Expression::ObjectExpression(inner) = &hover.value else {
        panic!("lowered nested literal")
    };
    assert_eq!(inner.properties.len(), inner_count);
    let color = inner.properties.last().unwrap_or_else(|| panic!("color"));
    let ObjectPropertyKind::ObjectProperty(color) = color else {
        panic!("declaration")
    };
    assert_eq!(color.key.static_name().as_deref(), Some("color"));
    assert!(matches!(&color.value, Expression::StringLiteral(value) if value.value == "blue"));
}
