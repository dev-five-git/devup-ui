use super::*;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("undefined", None)]
#[case("void 0", None)]
#[case("null", None)]
#[case("false", None)]
#[case("3", Some(3))]
#[case("dynamic", None)]
fn generic_style_order_when_a_jsx_container_holds_a_value_is_classified(
    #[case] code: &str,
    #[case] expected: Option<u8>,
) {
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = format!("({code});");
    let mut parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = parsed.program.body.remove(0) else {
        panic!("expression")
    };
    let value = JSXAttributeValue::new_expression_container(
        SPAN,
        statement.unbox().expression.into(),
        &builder,
    );
    assert_eq!(
        jsx_expression_to_style_order(&value, &allocator).as_static(),
        expected
    );
}

#[test]
fn private_member_when_pure_is_recognized_without_executing_a_getter() {
    struct Members {
        pure: Vec<bool>,
    }
    impl<'a> oxc_ast_visit::Visit<'a> for Members {
        fn visit_expression(&mut self, value: &Expression<'a>) {
            if matches!(value, Expression::PrivateFieldExpression(_)) {
                self.pure.push(is_pure(value));
            }
            oxc_ast_visit::walk::walk_expression(self, value);
        }
    }
    let allocator = Allocator::default();
    let source = "class A{#x;read(){return this.#x;}}";
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let mut members = Members { pure: Vec::new() };
    oxc_ast_visit::Visit::visit_program(&mut members, &parsed.program);
    assert_eq!(members.pure, vec![true]);
}

#[test]
fn object_style_order_when_it_is_not_a_number_or_empty_is_unsupported() {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, "([]);", SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    assert!(matches!(
        expression_to_style_order(&statement.expression, &allocator),
        ParsedStyleOrder::Unsupported
    ));
}

#[test]
fn unary_plus_when_its_literal_is_numeric_preserves_the_number() {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, "+3;", SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    assert_eq!(
        get_number_by_literal_expression(&statement.expression),
        Some(3.0)
    );
}
