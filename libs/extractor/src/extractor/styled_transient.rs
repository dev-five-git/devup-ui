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

/// The props a filter leaves out: the names it withholds, and those its
/// `shouldForwardProp`, as code testing the prop `k` at runtime, rejects
pub(super) struct Omitted<'s> {
    pub withheld: &'s [String],
    pub forward: Option<String>,
}

impl<'s> Omitted<'s> {
    /// What `forward` leaves out of what renders `element`: every prop it
    /// rejects, a spread's included, when the runtime can run it, which then
    /// decides the withheld names too; otherwise the withheld names
    pub(super) fn of(
        withheld: &'s [String],
        forward: Option<&crate::styled_reads::Forward>,
        element: &str,
    ) -> Self {
        match forward.and_then(|forward| forward.code("k", element)) {
            Some(code) => Self {
                withheld: &[],
                forward: Some(code),
            },
            None => Self {
                withheld,
                forward: None,
            },
        }
    }
}

/// `code`, generated, as an expression
fn parsed<'a>(builder: &AstBuilder<'a>, code: &str) -> Expression<'a> {
    let options = oxc_parser::ParseOptions {
        preserve_parens: false,
        ..oxc_parser::ParseOptions::default()
    };
    let mut expression = oxc_parser::Parser::new(
        builder.allocator(),
        builder.allocator().alloc_str(code),
        oxc_span::SourceType::mjs(),
    )
    .with_options(options)
    .parse_expression()
    // Generated from strings the build quoted, it always parses; were it not
    // to, its code is still written as it is
    .unwrap_or_else(|_| {
        Expression::new_identifier(SPAN, Str::from_in(code, builder.allocator()), builder)
    });
    crate::utils::clear_generated_spans(&mut expression);
    expression
}

/// Filter only the forwarded copy, leaving styled reads on the original rest
/// object, and leave out the props the styles withhold from it
pub(super) fn filter<'a>(
    builder: &AstBuilder<'a>,
    props: Expression<'a>,
    omitted: &Omitted<'_>,
) -> Expression<'a> {
    let withheld = omitted.withheld;
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
    let body = match &omitted.forward {
        Some(code) => Expression::new_logical_expression(
            SPAN,
            body,
            LogicalOperator::And,
            parsed(builder, code),
            builder,
        ),
        None => body,
    };
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
