//! Capture authored element operands without changing their scheduling coordinates.

use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        Expression, JSXAttributeItem, JSXAttributeName, JSXChild, JSXElement,
        JSXExpressionContainer, Str,
    },
    builder::AstBuilder,
};
use oxc_span::{GetSpan, GetSpanMut};
use oxc_syntax::operator::LogicalOperator;

use super::needed;
use crate::{utils::stays_attribute, visit::attribute_value_mut};

#[cfg(test)]
#[path = "element_capture_tests.rs"]
mod tests;

pub(crate) fn default_type<'a>(
    ast: &AstBuilder<'a>,
    value: Expression<'a>,
    default: &str,
) -> Expression<'a> {
    if matches!(crate::utils::unwrap_syntax_only(&value), Expression::StringLiteral(literal) if !literal.value.is_empty())
    {
        return value;
    }
    let span = value.span();
    Expression::new_logical_expression(
        span,
        Expression::new_parenthesized_expression(span, value, ast),
        LogicalOperator::Or,
        Expression::new_string_literal(span, Str::from_in(default, ast.allocator()), None, ast),
        ast,
    )
}

pub(crate) fn capture<'a>(
    ast: &AstBuilder<'a>,
    element: &mut JSXElement<'a>,
    next: &mut usize,
) -> Vec<(String, Expression<'a>)> {
    if !needed(element) {
        return vec![];
    }
    let mut values = Vec::new();
    let mut read = |value: &mut Expression<'a>| {
        let name = format!("__devupSpread{next}");
        *next += 1;
        let reference = Expression::new_identifier(
            value.span(),
            Str::from_in(name.as_str(), ast.allocator()),
            ast,
        );
        values.push((name, std::mem::replace(value, reference)));
    };
    for attribute in &mut element.opening_element.attributes {
        match attribute {
            JSXAttributeItem::Attribute(attribute) => {
                let span = attribute.span;
                let class_name = matches!(&attribute.name, JSXAttributeName::Identifier(name) if name.name == "className");
                let retained = match &attribute.name {
                    JSXAttributeName::Identifier(name) => {
                        stays_attribute(&name.name)
                            || matches!(name.name.as_str(), "className" | "style")
                    }
                    JSXAttributeName::NamespacedName(_) => true,
                };
                if retained && let Some(value) = attribute_value_mut(attribute) {
                    if class_name {
                        continue;
                    }
                    if value.span().start == 0 {
                        *value.span_mut() = span;
                    }
                    read(value);
                }
            }
            JSXAttributeItem::SpreadAttribute(_) => {}
        }
    }
    for child in &mut element.children {
        match child {
            JSXChild::ExpressionContainer(container) => {
                if let Some(value) = container.expression.as_expression_mut() {
                    read(value);
                }
            }
            JSXChild::Spread(spread) => read(&mut spread.expression),
            JSXChild::Element(element) => {
                let span = element.span;
                let mut value = Expression::JSXElement(element.clone_in(ast.allocator()));
                read(&mut value);
                *child = JSXChild::ExpressionContainer(JSXExpressionContainer::boxed(
                    span,
                    value.into(),
                    ast,
                ));
            }
            JSXChild::Fragment(fragment) => {
                let span = fragment.span;
                let mut value = Expression::JSXFragment(fragment.clone_in(ast.allocator()));
                read(&mut value);
                *child = JSXChild::ExpressionContainer(JSXExpressionContainer::boxed(
                    span,
                    value.into(),
                    ast,
                ));
            }
            JSXChild::Text(_) => {}
        }
    }
    values
}
