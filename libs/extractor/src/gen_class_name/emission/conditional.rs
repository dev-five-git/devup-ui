use super::super::roots::ClassPayload;
use super::super::roots::products::{ConditionalEmission, Generated};
use crate::utils::is_same_expression;
use oxc_allocator::{Box, CloneIn, GetAllocator};
use oxc_ast::ast::{
    ComputedMemberExpression, ConditionalExpression, Expression, LogicalExpression,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use oxc_syntax::operator::LogicalOperator;

pub(crate) fn normalize_conditional<'a, E: ClassPayload<'a>>(
    ast: &AstBuilder<'a>,
    test: &Expression<'a>,
    sides: (Generated<'a, E>, Generated<'a, E>),
) -> ConditionalEmission<'a, E> {
    match sides.0 {
        Generated::String(value) => {
            let first = Expression::StringLiteral(value.clone_in(ast.allocator()));
            choose(
                ast,
                test,
                (ConditionalEmission::String(value), first),
                sides.1,
            )
        }
        Generated::Template(value) => {
            let first = Expression::TemplateLiteral(value.clone_in(ast.allocator()));
            choose(
                ast,
                test,
                (ConditionalEmission::Template(value), first),
                sides.1,
            )
        }
        Generated::Supplied(value) => {
            let first = value.clone_expression(ast.allocator());
            choose(
                ast,
                test,
                (ConditionalEmission::Supplied(value), first),
                sides.1,
            )
        }
        Generated::Conditional(value) => construct(
            ast,
            test,
            (
                Expression::ConditionalExpression(value),
                sides.1.into_expression(),
            ),
        ),
        Generated::Lookup(value) => construct(
            ast,
            test,
            (
                Expression::LogicalExpression(value),
                sides.1.into_expression(),
            ),
        ),
    }
}

fn choose<'a, E: ClassPayload<'a>>(
    ast: &AstBuilder<'a>,
    test: &Expression<'a>,
    first: (ConditionalEmission<'a, E>, Expression<'a>),
    second: Generated<'a, E>,
) -> ConditionalEmission<'a, E> {
    if is_same_expression(&first.1, &second.clone_expression(ast.allocator())) {
        first.0
    } else {
        construct(
            ast,
            test,
            (
                first.0.into_generated().into_expression(),
                second.into_expression(),
            ),
        )
    }
}

fn construct<'a, E>(
    ast: &AstBuilder<'a>,
    test: &Expression<'a>,
    sides: (Expression<'a>, Expression<'a>),
) -> ConditionalEmission<'a, E> {
    ConditionalEmission::Conditional(ConditionalExpression::boxed(
        SPAN,
        test.clone_in(ast.allocator()),
        sides.0,
        sides.1,
        ast,
    ))
}

pub(crate) fn class_lookup_root<'a>(
    ast: &AstBuilder<'a>,
    member: Box<'a, ComputedMemberExpression<'a>>,
) -> Box<'a, LogicalExpression<'a>> {
    LogicalExpression::boxed(
        SPAN,
        Expression::new_parenthesized_expression(
            SPAN,
            Expression::ComputedMemberExpression(member.clone_in(ast.allocator())),
            ast,
        ),
        LogicalOperator::Or,
        Expression::new_string_literal(SPAN, "", None, ast),
        ast,
    )
}
