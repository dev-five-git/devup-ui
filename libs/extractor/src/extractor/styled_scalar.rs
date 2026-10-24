use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::SPAN;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

pub(in crate::extractor) enum Origin<'k> {
    WholeRule(&'k str),
    OrdinaryProperty(&'k str),
}

pub(in crate::extractor) fn zero<'a>(builder: &AstBuilder<'a>) -> Expression<'a> {
    Expression::new_numeric_literal(
        SPAN,
        0.0,
        None,
        oxc_syntax::number::NumberBase::Decimal,
        builder,
    )
}

pub(in crate::extractor) fn length<'a>(
    builder: &AstBuilder<'a>,
    leaf: Expression<'a>,
) -> Expression<'a> {
    let value = || Expression::new_identifier(SPAN, "__devupValue", builder);
    let is_number = Expression::new_binary_expression(
        SPAN,
        Expression::new_unary_expression(SPAN, UnaryOperator::Typeof, value(), builder),
        BinaryOperator::StrictEquality,
        Expression::new_string_literal(SPAN, "number", None, builder),
        builder,
    );
    let length = Expression::new_binary_expression(
        SPAN,
        value(),
        BinaryOperator::Addition,
        Expression::new_string_literal(SPAN, "px", None, builder),
        builder,
    );
    let length = Expression::new_conditional_expression(
        SPAN,
        Expression::new_logical_expression(
            SPAN,
            is_number,
            LogicalOperator::And,
            Expression::new_binary_expression(
                SPAN,
                value(),
                BinaryOperator::StrictInequality,
                zero(builder),
                builder,
            ),
            builder,
        ),
        length,
        value(),
        builder,
    );
    crate::utils::wrap_direct_call(
        builder,
        &super::named_arrow(builder, "__devupValue", length),
        &[leaf],
    )
}

/// The producer supplies a stable capture and its styled numeric origin, never an authored read.
pub(in crate::extractor) fn captured<'a>(
    builder: &AstBuilder<'a>,
    origin: Origin<'_>,
    slot: Expression<'a>,
) -> Expression<'a> {
    let (key, pixels) = match origin {
        Origin::WholeRule(key) => (key, !crate::utils::keeps_bare_number(key)),
        Origin::OrdinaryProperty(key) => (key, false),
    };
    let read = || slot.clone_in(builder.allocator());
    let present = Expression::new_logical_expression(
        SPAN,
        Expression::new_binary_expression(
            SPAN,
            read(),
            BinaryOperator::Inequality,
            Expression::new_null_literal(SPAN, builder),
            builder,
        ),
        LogicalOperator::And,
        Expression::new_binary_expression(
            SPAN,
            read(),
            BinaryOperator::StrictInequality,
            Expression::new_boolean_literal(SPAN, false, builder),
            builder,
        ),
        builder,
    );
    let value = if pixels {
        length(builder, read())
    } else if !crate::utils::keeps_bare_number(key)
        || css::disassemble_property(key).all(|property| crate::utils::is_unitless_key(&property))
    {
        read()
    } else {
        let numeric = Expression::new_logical_expression(
            SPAN,
            Expression::new_binary_expression(
                SPAN,
                Expression::new_unary_expression(SPAN, UnaryOperator::Typeof, read(), builder),
                BinaryOperator::StrictEquality,
                Expression::new_string_literal(SPAN, "number", None, builder),
                builder,
            ),
            LogicalOperator::And,
            Expression::new_binary_expression(
                SPAN,
                read(),
                BinaryOperator::StrictInequality,
                zero(builder),
                builder,
            ),
            builder,
        );
        let scaled = Expression::new_binary_expression(
            SPAN,
            Expression::new_binary_expression(
                SPAN,
                read(),
                BinaryOperator::Multiplication,
                Expression::new_numeric_literal(
                    SPAN,
                    4.0,
                    None,
                    oxc_syntax::number::NumberBase::Decimal,
                    builder,
                ),
                builder,
            ),
            BinaryOperator::Addition,
            Expression::new_string_literal(SPAN, "px", None, builder),
            builder,
        );
        Expression::new_conditional_expression(SPAN, numeric, scaled, read(), builder)
    };
    Expression::new_conditional_expression(
        SPAN,
        present,
        value,
        Expression::new_null_literal(SPAN, builder),
        builder,
    )
}
