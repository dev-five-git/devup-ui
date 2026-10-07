use super::{Order, invalid_order, number_value, static_order, truthiness};
use crate::{
    ErrorDisposition,
    utils::{readable_code, unwrap_syntax_only},
};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;
use oxc_syntax::operator::LogicalOperator;

/// A metadata rejection retains its evaluation policy until the public fallback boundary.
pub(crate) struct OrderError {
    pub diagnostic: (u32, String),
    pub disposition: ErrorDisposition,
}

pub(crate) fn parse_typed<'a>(
    value: &Expression<'a>,
    allocator: &'a Allocator,
) -> Result<Order<'a>, OrderError> {
    match unwrap_syntax_only(value) {
        Expression::ConditionalExpression(conditional) => {
            let branches = (
                parse_typed(&conditional.consequent, allocator),
                parse_typed(&conditional.alternate, allocator),
            );
            let (yes, no) = match branches {
                (Ok(yes), Ok(no)) => (yes, no),
                (Err(yes), Err(no)) => {
                    return Err(match (yes.disposition, no.disposition) {
                        (ErrorDisposition::NeedsEvaluation, ErrorDisposition::Definitive) => no,
                        (
                            ErrorDisposition::Definitive,
                            ErrorDisposition::Definitive | ErrorDisposition::NeedsEvaluation,
                        )
                        | (ErrorDisposition::NeedsEvaluation, ErrorDisposition::NeedsEvaluation) => {
                            yes
                        }
                    });
                }
                (Err(error), Ok(_)) | (Ok(_), Err(error)) => return Err(error),
            };
            Ok(match truthiness(&conditional.test) {
                Some(true) => yes,
                Some(false) => no,
                None => Order::Conditional {
                    test: conditional.test.clone_in(allocator),
                    yes: Box::new(yes),
                    no: Box::new(no),
                },
            })
        }
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
            let yes = parse_typed(&logical.right, allocator)?;
            Ok(match truthiness(&logical.left) {
                Some(true) => yes,
                Some(false) => Order::Absent,
                None => Order::Conditional {
                    test: logical.left.clone_in(allocator),
                    yes: Box::new(yes),
                    no: Box::new(Order::Absent),
                },
            })
        }
        value => static_order(value)
            .map(Order::Static)
            .ok_or_else(|| OrderError {
                diagnostic: (value.span().start, invalid_order(&readable_code(value))),
                disposition: match value {
                    Expression::StringLiteral(_)
                    | Expression::NumericLiteral(_)
                    | Expression::BooleanLiteral(_)
                    | Expression::NullLiteral(_)
                    | Expression::BigIntLiteral(_)
                    | Expression::ObjectExpression(_)
                    | Expression::ArrayExpression(_) => ErrorDisposition::Definitive,
                    Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
                        ErrorDisposition::Definitive
                    }
                    value if number_value(value).is_some() => ErrorDisposition::Definitive,
                    _ => ErrorDisposition::NeedsEvaluation,
                },
            }),
    }
}
