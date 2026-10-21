use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, FormalParameters},
    builder::AstBuilder,
};
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

use super::super::{identifier, named_arrow};

/// Only theme analysis uses a synthetic per-leaf arrow; emitted execution never does.
pub(super) fn analysis<'a>(
    builder: &AstBuilder<'a>,
    params: &FormalParameters<'a>,
    leaf: Expression<'a>,
) -> Expression<'a> {
    Expression::new_arrow_function_expression(
        leaf.span(),
        false,
        None,
        oxc_allocator::Box::new_in(
            params.clone_in_with_semantic_ids(builder.allocator()),
            builder,
        ),
        None,
        leaf.into(),
        builder,
    )
}

pub(super) fn is_written_out(value: &Expression<'_>) -> bool {
    let value = crate::utils::unwrap_syntax_only(value);
    crate::utils::get_string_by_literal_expression(value).is_some()
        || matches!(value, Expression::NullLiteral(_))
}

pub(super) fn zero<'a>(builder: &AstBuilder<'a>) -> Expression<'a> {
    Expression::new_numeric_literal(
        SPAN,
        0.0,
        None,
        oxc_syntax::number::NumberBase::Decimal,
        builder,
    )
}

/// Existing whole-rule conversion: nonzero numbers become px; zero/strings pass through.
pub(super) fn length<'a>(builder: &AstBuilder<'a>, leaf: Expression<'a>) -> Expression<'a> {
    let value = || identifier(builder, "__devupValue");
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
        &named_arrow(builder, "__devupValue", length),
        &[leaf],
    )
}
