use crate::visit::DevupVisitor;
use oxc_ast::ast::{Expression, ObjectPropertyKind, Statement};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>{return;}")]
#[case("function(p){return;}")]
#[case("p=>{}")]
#[case("function(p){}")]
#[case("p=>{'use strict';}")]
#[case("p=>{;return;}")]
#[case("p=>{{{return;}}}")]
#[case("p=>{if(p.stop){return;}else{return;}}")]
#[case("p=>{if(p.stop)return;}")]
#[case("p=>{const value=opaque();return;}")]
#[case("p=>{function inner(){return 255;}return;}")]
#[case("p=>{const inner=()=>255;return;}")]
#[serial]
fn bare_supplier_when_lowered_retains_one_capture_and_one_props_invocation(#[case] callback: &str) {
    // Given: the real parser and CssText supplier receive each pure/opaque source.
    let source = format!("`style-order:${{{callback}}};color:red`;");
    let allocator = oxc_allocator::Allocator::default();
    let mut parsed =
        oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
        panic!("literal fixture")
    };
    let value = crate::utils::unwrap_syntax_only_mut(&mut statement.expression);
    let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    assert!(crate::css_utils::literal_tree::lower(
        &visitor.ast,
        value,
        crate::css_utils::literal_tree::Scope {
            source: Some(&source),
            global: false,
        }
    ));
    let mut captures = Vec::new();
    let mut renders = Vec::new();
    // When: the actual order consumer prepares the production-supplied object.
    assert!(visitor.capture_literal_styled(value, &mut captures, &mut renders));
    // Then: even pure callbacks retain exactly one captured one-argument invocation.
    assert_eq!(visitor.errors, vec![]);
    assert_eq!(captures.len(), 1);
    assert_eq!(renders.len(), 1);
    let Expression::CallExpression(invocation) = &renders[0].1 else {
        panic!("retained invocation")
    };
    assert_eq!(
        crate::utils::readable_code(&invocation.callee),
        captures[0].0
    );
    assert_eq!(invocation.arguments.len(), 1);
    let props = invocation.arguments[0]
        .as_expression()
        .unwrap_or_else(|| panic!("props"));
    assert_eq!(crate::utils::readable_code(props), "__devupStyleProps");
    let Expression::ObjectExpression(object) = value else {
        panic!("lowered literal")
    };
    assert_eq!(object.properties.len(), 1);
    let ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        panic!("declaration")
    };
    assert_eq!(property.key.static_name().as_deref(), Some("color"));
    assert!(matches!(&property.value, Expression::StringLiteral(value) if value.value == "red"));
}
