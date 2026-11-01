use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::operator::LogicalOperator;

use crate::utils::{is_pure, unwrap_syntax_only, unwrap_syntax_only_mut};

/// Distribute only the controllers the existing pure-only policy permits.
/// Authored evaluations, when prepared, are already shared render bindings.
pub(super) fn distribute_conditions<'a>(
    builder: &AstBuilder<'a>,
    part: &mut Expression<'a>,
    refused: &[u32],
) {
    let clone = |expression: &Expression<'a>| expression.clone_in(builder.allocator());
    let parts_of = |expression: &Expression<'a>| match unwrap_syntax_only(expression) {
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .map(|element| element.as_expression().map(clone))
            .collect::<Option<Vec<_>>>(),
        _ => None,
    };
    let is_empty = |expression: &Expression<'a>| {
        matches!(
            unwrap_syntax_only(expression),
            Expression::NullLiteral(_) | Expression::BooleanLiteral(_)
        )
    };
    let null = || Expression::new_null_literal(SPAN, builder);
    let elements: Vec<Expression<'a>> = match unwrap_syntax_only_mut(part) {
        Expression::ArrayExpression(array) => {
            for element in &mut array.elements {
                if let Some(element) = element.as_expression_mut() {
                    distribute_conditions(builder, element, refused);
                }
            }
            return;
        }
        Expression::LogicalExpression(logical)
            if logical.operator == LogicalOperator::And
                && is_pure(&logical.left)
                && !refused.contains(&logical.span.start)
                && let Some(parts) = parts_of(&logical.right) =>
        {
            parts
                .into_iter()
                .map(|part| {
                    Expression::new_logical_expression(
                        SPAN,
                        clone(&logical.left),
                        LogicalOperator::And,
                        part,
                        builder,
                    )
                })
                .collect()
        }
        Expression::ConditionalExpression(conditional)
            if is_pure(&conditional.test)
                && !refused.contains(&conditional.span.start)
                && let Some(chosen) = parts_of(&conditional.consequent)
                    .or_else(|| is_empty(&conditional.consequent).then(Vec::new))
                && let Some(otherwise) = parts_of(&conditional.alternate)
                    .or_else(|| is_empty(&conditional.alternate).then(Vec::new)) =>
        {
            let choose = |part, chosen: bool| {
                let (consequent, alternate) = if chosen {
                    (part, null())
                } else {
                    (null(), part)
                };
                Expression::new_conditional_expression(
                    SPAN,
                    clone(&conditional.test),
                    consequent,
                    alternate,
                    builder,
                )
            };
            chosen
                .into_iter()
                .map(|part| choose(part, true))
                .chain(otherwise.into_iter().map(|part| choose(part, false)))
                .collect()
        }
        _ => return,
    };
    *part = Expression::new_array_expression(
        SPAN,
        oxc_allocator::Vec::from_iter_in(elements.into_iter().map(Into::into), builder),
        builder,
    );
    distribute_conditions(builder, part, refused);
}

pub(crate) fn refused_controllers(expression: &Expression<'_>, refused: &mut Vec<u32>) {
    match unwrap_syntax_only(expression) {
        Expression::ArrayExpression(array) => {
            for element in &array.elements {
                if let Some(element) = element.as_expression() {
                    refused_controllers(element, refused);
                }
            }
        }
        Expression::ConditionalExpression(choice) => {
            if !is_pure(&choice.test) {
                refused.push(choice.span.start);
            }
            refused_controllers(&choice.consequent, refused);
            refused_controllers(&choice.alternate, refused);
        }
        Expression::LogicalExpression(logical) => {
            if !is_pure(&logical.left) {
                refused.push(logical.span().start);
            }
            if logical.operator != LogicalOperator::And {
                refused_controllers(&logical.left, refused);
            }
            refused_controllers(&logical.right, refused);
        }
        _ => {}
    }
}
