use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{BinaryOperator, Expression},
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::number::NumberBase;

use crate::{ExtractStyleProp, extract_style::extract_static_style::ThemeTokenResolution};
use css::style_selector::StyleSelector;

#[cfg(test)]
#[path = "../../falsy_controller_tests.rs"]
mod tests;

pub(super) fn zero<'a>(
    ast: &AstBuilder<'a>,
    name: &str,
    controller: &Expression<'a>,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Option<Box<ExtractStyleProp<'a>>> {
    let styles = super::create_static_styles(
        name,
        "0",
        &[level],
        selector,
        ThemeTokenResolution::CssVariable,
    );
    (!styles.is_empty()).then(|| {
        Box::new(ExtractStyleProp::Conditional {
            condition: Expression::new_binary_expression(
                SPAN,
                controller.clone_in(ast.allocator()),
                BinaryOperator::StrictEquality,
                Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, ast),
                ast,
            ),
            consequent: Some(Box::new(ExtractStyleProp::StaticArray(styles))),
            alternate: None,
        })
    })
}
