use super::*;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

pub(super) fn is_pure(expression: &Expression<'_>) -> bool {
    use oxc_ast::ast::{ArrayExpressionElement, PropertyKind};
    match expression {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::Identifier(_)
        | Expression::ThisExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::FunctionExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.iter().all(is_pure),
        Expression::StaticMemberExpression(member) => is_pure(&member.object),
        Expression::PrivateFieldExpression(member) => is_pure(&member.object),
        Expression::ComputedMemberExpression(member) => {
            is_pure(&member.object) && is_pure(&member.expression)
        }
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && is_pure(&unary.argument)
        }
        Expression::BinaryExpression(binary) => is_pure(&binary.left) && is_pure(&binary.right),
        Expression::LogicalExpression(logical) => is_pure(&logical.left) && is_pure(&logical.right),
        Expression::ConditionalExpression(conditional) => {
            is_pure(&conditional.test)
                && is_pure(&conditional.consequent)
                && is_pure(&conditional.alternate)
        }
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::SpreadElement(spread) => is_pure(&spread.argument),
            ArrayExpressionElement::Elision(_) => true,
            element => element.as_expression().is_some_and(is_pure),
        }),
        Expression::ObjectExpression(object) => {
            object.properties.iter().all(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    property.key.as_expression().is_none_or(is_pure)
                        && (property.kind != PropertyKind::Init || is_pure(&property.value))
                }
                ObjectPropertyKind::SpreadProperty(spread) => is_pure(&spread.argument),
            })
        }
        Expression::ParenthesizedExpression(inner) => is_pure(&inner.expression),
        Expression::TSAsExpression(inner) => is_pure(&inner.expression),
        Expression::TSSatisfiesExpression(inner) => is_pure(&inner.expression),
        Expression::TSNonNullExpression(inner) => is_pure(&inner.expression),
        Expression::TSTypeAssertion(inner) => is_pure(&inner.expression),
        _ => false,
    }
}

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

#[rstest]
#[case("true?2:3", Some(2))]
#[case("false?2:3", Some(3))]
#[case("null?2:3", Some(3))]
#[case("''?2:3", Some(3))]
#[case("'x'?2:3", Some(2))]
#[case("``?2:3", Some(3))]
#[case("`x`?2:3", Some(2))]
#[case("!true?2:3", Some(3))]
#[case("true&&2", Some(2))]
#[case("false&&2", None)]
fn parsed_order_when_literal_guard_is_known_selects_exact_layer(
    #[case] source: &str,
    #[case] expected: Option<u8>,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    let actual = expression_to_style_order(&statement.expression, &allocator);
    assert!(!matches!(actual, ParsedStyleOrder::Unsupported));
    assert_eq!(actual.as_static(), expected);
}

#[test]
fn parsed_order_when_unselected_explicit_branch_is_invalid_rejects_it() {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, "true?2:'01'", SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    assert!(matches!(
        expression_to_style_order(&statement.expression, &allocator),
        ParsedStyleOrder::Unsupported
    ));
}

#[test]
fn style_arguments_when_literal_spread_replaces_a_declaration_keeps_other_keys() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let parsed = Parser::new(
        &allocator,
        "styled(...[{color:'red',backgroundColor:'black'},{color:'blue'}]);",
        SourceType::ts(),
    )
    .parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("call required")
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("call required")
    };
    let actual = style_arguments(&ast, &call.arguments)
        .unwrap_or_else(|| panic!("finite literal spread must compose"));
    let Expression::ObjectExpression(object) = actual.rules else {
        panic!("merged rules must be an object")
    };
    let declarations: Vec<_> = object
        .properties
        .iter()
        .filter_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property) => Some((
                property
                    .key
                    .static_name()
                    .unwrap_or_else(|| panic!("literal key"))
                    .to_string(),
                get_string_by_literal_expression(&property.value)
                    .unwrap_or_else(|| panic!("literal value"))
                    .to_string(),
            )),
            ObjectPropertyKind::SpreadProperty(_) => None,
        })
        .collect();
    assert_eq!(
        declarations,
        vec![
            ("backgroundColor".to_string(), "black".to_string()),
            ("color".to_string(), "blue".to_string())
        ]
    );
    assert_eq!(actual.classes.len(), 0);
}
