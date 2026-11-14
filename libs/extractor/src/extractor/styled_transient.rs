use oxc_allocator::{FromIn, GetAllocator, Vec};
use oxc_ast::{
    ast::{
        BindingPattern, Expression, FormalParameter, FormalParameterKind, FormalParameters,
        IdentifierName, StaticMemberExpression, Str,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::{
    number::NumberBase,
    operator::{BinaryOperator, LogicalOperator},
};

/// Filter only the forwarded copy, leaving styled reads on the original rest
/// object, and leave out the props the styles withhold from it
pub(super) fn filter<'a>(
    builder: &AstBuilder<'a>,
    props: Expression<'a>,
    withheld: &[String],
) -> Expression<'a> {
    let key = Expression::new_computed_member_expression(
        SPAN,
        Expression::new_identifier(SPAN, "k", builder),
        Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, builder),
        false,
        builder,
    );
    let differs = |left, right| {
        Expression::new_binary_expression(
            SPAN,
            left,
            BinaryOperator::StrictInequality,
            right,
            builder,
        )
    };
    let body = withheld.iter().fold(
        differs(
            key,
            Expression::new_string_literal(SPAN, "$", None, builder),
        ),
        |body, name| {
            let name = Str::from_in(name.as_str(), builder.allocator());
            Expression::new_logical_expression(
                SPAN,
                body,
                LogicalOperator::And,
                differs(
                    Expression::new_identifier(SPAN, "k", builder),
                    Expression::new_string_literal(SPAN, name, None, builder),
                ),
                builder,
            )
        },
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
