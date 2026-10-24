use oxc_allocator::{FromIn, GetAllocator, TakeIn};
use oxc_ast::{
    ast::{
        ArrowFunctionBody, AssignmentTarget, BindingPattern, ComputedMemberExpression, Expression,
        Statement, VariableDeclaration, VariableDeclarationKind, VariableDeclarator,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::operator::AssignmentOperator;

pub(super) fn slot<'a>(
    builder: &AstBuilder<'a>,
    name: &str,
    index: usize,
) -> oxc_allocator::Box<'a, ComputedMemberExpression<'a>> {
    ComputedMemberExpression::boxed(
        SPAN,
        Expression::new_identifier(
            SPAN,
            oxc_ast::ast::Ident::from_in(name, builder.allocator()),
            builder,
        ),
        Expression::new_string_literal(
            SPAN,
            oxc_ast::ast::Str::from_in(index.to_string().as_str(), builder.allocator()),
            None,
            builder,
        ),
        false,
        builder,
    )
}

pub(super) fn assign<'a>(
    builder: &AstBuilder<'a>,
    target: oxc_allocator::Box<'a, ComputedMemberExpression<'a>>,
    value: Expression<'a>,
) -> Expression<'a> {
    Expression::new_assignment_expression(
        SPAN,
        AssignmentOperator::Assign,
        AssignmentTarget::ComputedMemberExpression(target),
        value,
        builder,
    )
}

pub(super) fn discriminator<'a>(builder: &AstBuilder<'a>, value: usize) -> Expression<'a> {
    Expression::new_string_literal(
        SPAN,
        oxc_ast::ast::Str::from_in(value.to_string().as_str(), builder.allocator()),
        None,
        builder,
    )
}

pub(super) fn declare<'a>(builder: &AstBuilder<'a>, function: &mut Expression<'a>, name: &str) {
    let declaration = Statement::VariableDeclaration(VariableDeclaration::boxed(
        SPAN,
        VariableDeclarationKind::Const,
        oxc_allocator::Vec::from_array_in(
            [VariableDeclarator::new(
                SPAN,
                BindingPattern::new_binding_identifier(
                    SPAN,
                    oxc_ast::ast::Ident::from_in(name, builder.allocator()),
                    builder,
                ),
                None,
                Some(Expression::new_array_expression(
                    SPAN,
                    oxc_allocator::Vec::from_array_in([discriminator(builder, 0).into()], builder),
                    builder,
                )),
                false,
                builder,
            )],
            builder,
        ),
        false,
        builder,
    ));
    match crate::utils::unwrap_syntax_only_mut(function) {
        Expression::ArrowFunctionExpression(arrow) => match &mut arrow.body {
            ArrowFunctionBody::FunctionBody(body) => body.statements.insert(0, declaration),
            body => {
                let value = body.as_expression_mut().map(|value| value.take_in(builder));
                let statements = oxc_allocator::Vec::from_array_in(
                    [
                        declaration,
                        Statement::new_return_statement(SPAN, value, builder),
                    ],
                    builder,
                );
                *body = ArrowFunctionBody::new_function_body(
                    SPAN,
                    oxc_allocator::Vec::new_in(builder),
                    statements,
                    builder,
                );
            }
        },
        Expression::FunctionExpression(function) => {
            if let Some(body) = &mut function.body {
                body.statements.insert(0, declaration);
            }
        }
        _ => unreachable!("only original callbacks have capture declarations"),
    }
}
