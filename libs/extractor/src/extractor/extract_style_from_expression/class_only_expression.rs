use oxc_ast::ast::Expression;

use crate::utils::unwrap_syntax_only;

/// Scalar class selections retain their entire authored operand, rather than
/// separately evaluating a logical predicate and the value it selected.
pub(super) fn runtime_scalar(expression: &Expression<'_>) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::LogicalExpression(value) => {
            runtime_scalar(&value.left) && runtime_scalar(&value.right)
        }
        Expression::ComputedMemberExpression(value) => !matches!(
            unwrap_syntax_only(&value.object),
            Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
        ),
        Expression::ConditionalExpression(_)
        | Expression::TemplateLiteral(_)
        | Expression::ObjectExpression(_)
        | Expression::ArrayExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::FunctionExpression(_)
        | Expression::ClassExpression(_) => false,
        _ => true,
    }
}
