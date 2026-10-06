use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        AssignmentTarget, BindingPattern, ComputedMemberExpression, Expression, FormalParameter,
        FormalParameterKind, FormalParameters, Str,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::operator::AssignmentOperator;

#[derive(Clone)]
pub(crate) struct Environment {
    pub key: String,
    pub names: Vec<String>,
}

fn identifier<'a>(builder: &AstBuilder<'a>, name: &str) -> Expression<'a> {
    Expression::new_identifier(SPAN, Str::from_in(name, builder.allocator()), builder)
}

pub(crate) fn continuation<'a>(
    builder: &AstBuilder<'a>,
    names: &[String],
    body: Expression<'a>,
) -> Expression<'a> {
    let parameters = names.iter().map(|name| {
        FormalParameter::new(
            SPAN,
            oxc_allocator::Vec::new_in(builder),
            BindingPattern::new_binding_identifier(
                SPAN,
                Str::from_in(name.as_str(), builder.allocator()),
                builder,
            ),
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
            None::<oxc_allocator::Box<Expression<'a>>>,
            false,
            None,
            false,
            false,
            builder,
        )
    });
    Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            oxc_allocator::Vec::from_iter_in(parameters, builder),
            None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
            builder,
        ),
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body.into(),
        builder,
    )
}

/// The carrier only passes saved values to generated continuations; it never receives styles.
pub(crate) fn attach<'a>(
    builder: &AstBuilder<'a>,
    environment: &Environment,
    component: Expression<'a>,
) -> Expression<'a> {
    let component_name = format!("{}Component", environment.key);
    let continue_name = format!("{}Continue", environment.key);
    let values: Vec<_> = environment
        .names
        .iter()
        .map(|name| identifier(builder, name))
        .collect();
    let carrier = continuation(
        builder,
        std::slice::from_ref(&continue_name),
        crate::utils::wrap_direct_call(builder, &identifier(builder, &continue_name), &values),
    );
    let target = AssignmentTarget::ComputedMemberExpression(ComputedMemberExpression::boxed(
        SPAN,
        identifier(builder, &component_name),
        Expression::new_string_literal(
            SPAN,
            Str::from_in(environment.key.as_str(), builder.allocator()),
            None,
            builder,
        ),
        false,
        builder,
    ));
    let assignment = Expression::new_assignment_expression(
        SPAN,
        AssignmentOperator::Assign,
        target,
        carrier,
        builder,
    );
    let body = Expression::new_sequence_expression(
        SPAN,
        oxc_allocator::Vec::from_array_in(
            [assignment, identifier(builder, &component_name)],
            builder,
        ),
        builder,
    );
    crate::utils::call_with_values(builder, vec![(component_name, component)], body)
}

pub(crate) fn rebind<'a>(
    builder: &AstBuilder<'a>,
    component: &Expression<'a>,
    environment: &Environment,
    body: Expression<'a>,
) -> Expression<'a> {
    let carrier = Expression::new_computed_member_expression(
        SPAN,
        component.clone_in(builder.allocator()),
        Expression::new_string_literal(
            SPAN,
            Str::from_in(environment.key.as_str(), builder.allocator()),
            None,
            builder,
        ),
        false,
        builder,
    );
    crate::utils::wrap_direct_call(
        builder,
        &carrier,
        &[continuation(builder, &environment.names, body)],
    )
}
