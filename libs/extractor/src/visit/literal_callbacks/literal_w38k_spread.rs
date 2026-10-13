use crate::visit::DevupVisitor;
use oxc_ast::ast::{Expression, ObjectPropertyKind, Statement};
use serial_test::serial;

#[test]
#[serial]
fn bare_nested_supplier_when_root_has_a_real_spread_preserves_it_unchanged() {
    // Given: the actual parser supplies a mixed root and a nested template callback.
    let source = "({...rest,_hover:`style-order:${p=>{return;}};color:red`});";
    let allocator = oxc_allocator::Allocator::default();
    let mut parsed =
        oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("mixed fixture")
    };
    let value = crate::utils::unwrap_syntax_only_mut(&mut statement.expression);
    let Expression::ObjectExpression(original) = value else {
        panic!("mixed root")
    };
    let ObjectPropertyKind::SpreadProperty(original_spread) = &original.properties[0] else {
        panic!("source spread")
    };
    let original_span = original_spread.span;
    let original_argument = crate::utils::readable_code(&original_spread.argument);
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    let mut captures = Vec::new();
    let mut renders = Vec::new();
    // When: literal_tree and the real consumer prepare the mixed source object.
    assert!(crate::css_utils::literal_tree::lower(
        &visitor.ast,
        value,
        crate::css_utils::literal_tree::Scope {
            source: Some(source),
            global: false,
        }
    ));
    assert!(visitor.capture_literal_styled(value, &mut captures, &mut renders));
    visitor.check_style_orders(value, false);
    // Then: one capture/render survives, the original spread stays and bare metadata goes.
    assert_eq!(visitor.errors, vec![]);
    assert_eq!(captures.len(), 1);
    assert_eq!(renders.len(), 1);
    let Expression::ObjectExpression(object) = value else {
        panic!("mixed root")
    };
    assert_eq!(object.properties.len(), 2);
    let ObjectPropertyKind::SpreadProperty(spread) = &object.properties[0] else {
        panic!("retained spread")
    };
    assert_eq!(spread.span, original_span);
    assert_eq!(
        crate::utils::readable_code(&spread.argument),
        original_argument
    );
    let ObjectPropertyKind::ObjectProperty(hover) = &object.properties[1] else {
        panic!("hover scope")
    };
    assert_eq!(hover.key.static_name().as_deref(), Some("_hover"));
    let Expression::ObjectExpression(inner) = &hover.value else {
        panic!("lowered hover")
    };
    assert_eq!(inner.properties.len(), 1);
    let ObjectPropertyKind::ObjectProperty(color) = &inner.properties[0] else {
        panic!("hover declaration")
    };
    assert_eq!(color.key.static_name().as_deref(), Some("color"));
    assert!(matches!(&color.value, Expression::StringLiteral(value) if value.value == "red"));
}
