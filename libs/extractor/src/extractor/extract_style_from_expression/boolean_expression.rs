use oxc_ast::ast::{Expression, UnaryOperator};

use crate::utils::unwrap_syntax_only;

/// Prove a boolean result from syntax, without inferring identifier or call types.
pub(super) fn is_boolean(expression: &Expression<'_>) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::BooleanLiteral(_) => true,
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::LogicalNot,
        Expression::BinaryExpression(binary) => {
            binary.operator.is_equality()
                || binary.operator.is_compare()
                || binary.operator.is_relational()
        }
        Expression::ConditionalExpression(conditional) => {
            is_boolean(&conditional.consequent) && is_boolean(&conditional.alternate)
        }
        Expression::LogicalExpression(logical) => {
            is_boolean(&logical.left) && is_boolean(&logical.right)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_boolean;
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    #[rstest]
    #[case("true", true)]
    #[case("false", true)]
    #[case("a == b", true)]
    #[case("a != b", true)]
    #[case("a === b", true)]
    #[case("a !== b", true)]
    #[case("a < b", true)]
    #[case("a <= b", true)]
    #[case("a > b", true)]
    #[case("a >= b", true)]
    #[case("a in b", true)]
    #[case("a instanceof b", true)]
    #[case("!a", true)]
    #[case("!!a", true)]
    #[case("((a === b) as boolean) satisfies boolean", true)]
    #[case("(a === b)!", true)]
    #[case("flag ? a === b : !a", true)]
    #[case("a === b || !a", true)]
    #[case("a === b && !a", true)]
    #[case("a === b ?? !a", true)]
    #[case("a", false)]
    #[case("a as boolean", false)]
    #[case("Boolean(a)", false)]
    #[case("'true'", false)]
    #[case("a + b", false)]
    #[case("a & b", false)]
    #[case("+a", false)]
    #[case("a === b || '4px'", false)]
    #[case("a === b && value", false)]
    #[case("flag ? true : value", false)]
    fn boolean_result_is_proven_when_syntax_guarantees_it(
        #[case] expression: &str,
        #[case] expected: bool,
    ) {
        // Given: parsed syntax, with no identifier-resolution assumptions.
        let allocator = Allocator::default();
        let source = format!("({expression});");
        let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("expected expression statement");
        };
        // When: syntax-only wrappers are stripped for result classification.
        let actual = is_boolean(&statement.expression);
        // Then: only guaranteed booleans are excluded from CSS consumers.
        assert_eq!(actual, expected);
    }
}
