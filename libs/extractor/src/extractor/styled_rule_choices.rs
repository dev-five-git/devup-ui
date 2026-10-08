use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

use super::StyledBindings;
use crate::utils::{is_pure, unwrap_syntax_only};

#[cfg(test)]
#[path = "w27_styled_choice_coverage.rs"]
mod coverage_tests;

enum Choice<'a> {
    Rules {
        value: Expression<'a>,
        truthy: bool,
    },
    Scalar {
        value: Expression<'a>,
        truthy: Option<bool>,
        nullish: Option<bool>,
    },
    Branch {
        test: Expression<'a>,
        yes: Box<Choice<'a>>,
        no: Box<Choice<'a>>,
    },
}

struct ShortCircuit<'s, 'a, 'b> {
    operator: LogicalOperator,
    right: &'s Expression<'a>,
    bindings: &'s StyledBindings<'b>,
}

pub(super) enum RuleError {
    Opaque,
    BoundUndefined,
}

pub(super) fn normalize<'a>(
    ast: &AstBuilder<'a>,
    value: &Expression<'a>,
    bindings: &StyledBindings<'_>,
) -> Result<Expression<'a>, RuleError> {
    emit(ast, read(ast, value, bindings)?).ok_or(RuleError::Opaque)
}

fn read<'a>(
    ast: &AstBuilder<'a>,
    value: &Expression<'a>,
    bindings: &StyledBindings<'_>,
) -> Result<Choice<'a>, RuleError> {
    let value = unwrap_syntax_only(value);
    if let Some(styles) = bindings.styles(value) {
        return Ok(Choice::Rules {
            value: value.clone_in_with_semantic_ids(ast.allocator()),
            truthy: !styles.is_empty(),
        });
    }
    match value {
        Expression::ObjectExpression(_) => Ok(Choice::Rules {
            value: value.clone_in_with_semantic_ids(ast.allocator()),
            truthy: true,
        }),
        Expression::ConditionalExpression(branch) if is_pure(&branch.test) => Ok(Choice::Branch {
            test: branch.test.clone_in_with_semantic_ids(ast.allocator()),
            yes: Box::new(read(ast, &branch.consequent, bindings)?),
            no: Box::new(read(ast, &branch.alternate, bindings)?),
        }),
        Expression::LogicalExpression(logical) => {
            let left = read(ast, &logical.left, bindings)?;
            select(
                ast,
                left,
                &ShortCircuit {
                    operator: logical.operator,
                    right: &logical.right,
                    bindings,
                },
            )
        }
        Expression::NullLiteral(_) => Ok(scalar(ast, value, (Some(false), Some(true)))),
        Expression::UnaryExpression(unary)
            if unary.operator == UnaryOperator::Void && is_pure(&unary.argument) =>
        {
            Ok(scalar(ast, value, (Some(false), Some(true))))
        }
        Expression::BooleanLiteral(literal) => {
            Ok(scalar(ast, value, (Some(literal.value), Some(false))))
        }
        Expression::Identifier(identifier) if identifier.name == "undefined" => {
            if bindings.values.symbol(value).is_some() {
                Err(RuleError::BoundUndefined)
            } else if identifier.reference_id.get().is_some() {
                Ok(scalar(ast, value, (Some(false), Some(true))))
            } else {
                Err(RuleError::Opaque)
            }
        }
        Expression::NumericLiteral(literal) => Ok(scalar(
            ast,
            value,
            (
                Some(literal.value != 0.0 && !literal.value.is_nan()),
                Some(false),
            ),
        )),
        Expression::StringLiteral(literal) => Ok(scalar(
            ast,
            value,
            (Some(!literal.value.is_empty()), Some(false)),
        )),
        value if is_pure(value) => Ok(scalar(ast, value, (None, None))),
        _ => Err(RuleError::Opaque),
    }
}

fn scalar<'a>(
    ast: &AstBuilder<'a>,
    value: &Expression<'a>,
    facts: (Option<bool>, Option<bool>),
) -> Choice<'a> {
    let (truthy, nullish) = facts;
    Choice::Scalar {
        value: value.clone_in_with_semantic_ids(ast.allocator()),
        truthy,
        nullish,
    }
}

fn select<'a>(
    ast: &AstBuilder<'a>,
    left: Choice<'a>,
    next: &ShortCircuit<'_, 'a, '_>,
) -> Result<Choice<'a>, RuleError> {
    let right = || read(ast, next.right, next.bindings);
    match left {
        Choice::Branch { test, yes, no } => Ok(Choice::Branch {
            test,
            yes: Box::new(select(ast, *yes, next)?),
            no: Box::new(select(ast, *no, next)?),
        }),
        left @ Choice::Rules { truthy, .. } => match next.operator {
            LogicalOperator::And if truthy => right(),
            LogicalOperator::Or if !truthy => right(),
            LogicalOperator::And | LogicalOperator::Or | LogicalOperator::Coalesce => Ok(left),
        },
        Choice::Scalar {
            value,
            truthy,
            nullish,
        } => {
            let selected = match next.operator {
                LogicalOperator::And => truthy.map(|truthy| !truthy),
                LogicalOperator::Or => truthy,
                LogicalOperator::Coalesce => nullish.map(|nullish| !nullish),
            };
            let left = scalar(ast, &value, (truthy, nullish));
            match selected {
                Some(true) => Ok(left),
                Some(false) => right(),
                None => {
                    let (test, yes, no) = match next.operator {
                        LogicalOperator::And => (
                            value.clone_in_with_semantic_ids(ast.allocator()),
                            right()?,
                            scalar(ast, &value, (Some(false), nullish)),
                        ),
                        LogicalOperator::Or => (
                            value.clone_in_with_semantic_ids(ast.allocator()),
                            scalar(ast, &value, (Some(true), Some(false))),
                            right()?,
                        ),
                        LogicalOperator::Coalesce => (
                            is_nullish(ast, &value),
                            right()?,
                            scalar(ast, &value, (truthy, Some(false))),
                        ),
                    };
                    Ok(Choice::Branch {
                        test,
                        yes: Box::new(yes),
                        no: Box::new(no),
                    })
                }
            }
        }
    }
}

fn is_nullish<'a>(ast: &AstBuilder<'a>, value: &Expression<'a>) -> Expression<'a> {
    let compare = |right| {
        Expression::new_binary_expression(
            SPAN,
            value.clone_in_with_semantic_ids(ast.allocator()),
            BinaryOperator::StrictEquality,
            right,
            ast,
        )
    };
    let undefined = Expression::new_unary_expression(
        SPAN,
        UnaryOperator::Void,
        Expression::new_numeric_literal(
            SPAN,
            0.0,
            None,
            oxc_syntax::number::NumberBase::Decimal,
            ast,
        ),
        ast,
    );
    Expression::new_logical_expression(
        SPAN,
        compare(Expression::new_null_literal(SPAN, ast)),
        LogicalOperator::Or,
        compare(undefined),
        ast,
    )
}

fn emit<'a>(ast: &AstBuilder<'a>, choice: Choice<'a>) -> Option<Expression<'a>> {
    match choice {
        Choice::Rules { value, .. } => Some(value),
        Choice::Scalar {
            value,
            truthy,
            nullish,
        } => (truthy == Some(false)
            || nullish == Some(true)
            || matches!(value, Expression::BooleanLiteral(_)))
        .then(|| Expression::new_null_literal(value.span(), ast)),
        Choice::Branch { test, yes, no } => Some(Expression::new_conditional_expression(
            test.span(),
            test,
            emit(ast, *yes)?,
            emit(ast, *no)?,
            ast,
        )),
    }
}
