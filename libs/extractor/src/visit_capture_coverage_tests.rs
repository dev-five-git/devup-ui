use super::DevupVisitor;
use crate::coverage_tests::{code, expression};
use oxc_allocator::Allocator;

#[test]
fn retained_prop_capture_leaves_style_operands_with_their_owner() {
    // Given
    let allocator = Allocator::default();
    let mut visitor = DevupVisitor::new(&allocator, "capture.tsx", "@devup-ui/react", vec![], None);
    let mut props = expression(
        &allocator,
        "({p:state.pad,id:state.id,...{title:state.title}})",
    );
    // When
    let captures = visitor.read_spreads_once(&mut props);
    // Then: padding remains authored for style lowering, id/spread are captured once.
    assert_eq!(captures.len(), 2);
    assert_eq!(code(&captures[0].1), "state.id");
    let oxc_ast::ast::Expression::ObjectExpression(object) = &props else {
        panic!("object fixture")
    };
    let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        panic!("padding fixture")
    };
    assert_eq!(property.key.name().as_deref(), Some("p"));
    assert_eq!(code(&property.value), "state.pad");
    assert!(code(&captures[1].1).contains("state.title"));
}
