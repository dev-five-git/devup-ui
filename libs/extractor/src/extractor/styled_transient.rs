use oxc_allocator::Vec;
use oxc_ast::{
    ast::{
        BindingPattern, Expression, FormalParameter, FormalParameterKind, FormalParameters,
        IdentifierName, StaticMemberExpression,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::{number::NumberBase, operator::BinaryOperator};

/// Filter only the forwarded copy, leaving styled reads on the original rest object.
pub(super) fn filter<'a>(builder: &AstBuilder<'a>, props: Expression<'a>) -> Expression<'a> {
    let key = Expression::new_computed_member_expression(
        SPAN,
        Expression::new_identifier(SPAN, "k", builder),
        Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, builder),
        false,
        builder,
    );
    let body = Expression::new_binary_expression(
        SPAN,
        key,
        BinaryOperator::StrictInequality,
        Expression::new_string_literal(SPAN, "$", None, builder),
        builder,
    );
    let pattern = BindingPattern::new_array_pattern(
        SPAN,
        Vec::from_array_in(
            [Some(BindingPattern::new_binding_identifier(
                SPAN, "k", builder,
            ))],
            builder,
        ),
        None,
        builder,
    );
    let parameter = FormalParameter::new(
        SPAN,
        Vec::new_in(builder),
        pattern,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        None::<oxc_allocator::Box<Expression<'a>>>,
        false,
        None,
        false,
        false,
        builder,
    );
    let params = FormalParameters::boxed(
        SPAN,
        FormalParameterKind::ArrowFormalParameters,
        Vec::from_array_in([parameter], builder),
        None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
        builder,
    );
    let predicate = Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        params,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body.into(),
        builder,
    );
    let object = || Expression::new_identifier(SPAN, "Object", builder);
    let entries = call(builder, member(builder, object(), "entries"), props);
    let filtered = call(builder, member(builder, entries, "filter"), predicate);
    call(builder, member(builder, object(), "fromEntries"), filtered)
}

fn member<'a>(
    builder: &AstBuilder<'a>,
    object: Expression<'a>,
    name: &'static str,
) -> Expression<'a> {
    Expression::StaticMemberExpression(StaticMemberExpression::boxed(
        SPAN,
        object,
        IdentifierName::new(SPAN, name, builder),
        false,
        builder,
    ))
}

fn call<'a>(
    builder: &AstBuilder<'a>,
    callee: Expression<'a>,
    argument: Expression<'a>,
) -> Expression<'a> {
    Expression::new_call_expression(
        SPAN,
        callee,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
        Vec::from_array_in([argument.into()], builder),
        false,
        builder,
    )
}
