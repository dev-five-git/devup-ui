use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, FormalParameters},
    builder::AstBuilder,
};
use oxc_span::GetSpan;

pub(super) use crate::extractor::extract_style_from_styled::scalar::zero;

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

pub(super) fn captured<'a>(
    builder: &AstBuilder<'a>,
    key: &str,
    slot: Expression<'a>,
) -> Expression<'a> {
    crate::extractor::extract_style_from_styled::scalar::captured(
        builder,
        crate::extractor::extract_style_from_styled::scalar::Origin::WholeRule(key),
        slot,
    )
}
