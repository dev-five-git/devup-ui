//! Parsing reserved cascade orders without changing general numeric coercion.

use boa_engine::JsString;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, JSXAttributeValue};
use oxc_syntax::operator::{LogicalOperator, UnaryOperator};

use crate::utils::{ParsedStyleOrder, build_time_error, unwrap_syntax_only};

/// Canonical decimal digits naming one of the user-addressable layers.
pub(crate) fn string_order(value: &str) -> Option<u8> {
    if !value
        .as_bytes()
        .first()
        .copied()
        .is_some_and(|byte| matches!(byte, b'1'..=b'9'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    value
        .parse::<u8>()
        .ok()
        .filter(|order| (1..=254).contains(order))
}

/// A finite integer order or a canonical decimal string, never a truncated cast.
pub(crate) fn static_order(value: &Expression<'_>) -> Option<u8> {
    let value = unwrap_syntax_only(value);
    match value {
        Expression::StringLiteral(literal) => string_order(literal.value.as_str()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| string_order(cooked.as_str())),
        Expression::NumericLiteral(_) | Expression::UnaryExpression(_) => number_value(value)
            .and_then(|number| number.to_string().parse::<u8>().ok())
            .filter(|order| (1..=254).contains(order)),
        _ => None,
    }
}

/// Preserve literal numeric coercion for unary orders, never template interpolations.
fn number_value(value: &Expression<'_>) -> Option<f64> {
    match unwrap_syntax_only(value) {
        Expression::NumericLiteral(literal) => Some(literal.value),
        Expression::StringLiteral(literal) => {
            Some(JsString::from(literal.value.as_str()).to_number())
        }
        Expression::BooleanLiteral(literal) => Some(f64::from(u8::from(literal.value))),
        Expression::NullLiteral(_) => Some(0.0),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .map(|cooked| JsString::from(cooked.as_str()).to_number()),
        Expression::UnaryExpression(unary) => match unary.operator {
            UnaryOperator::UnaryNegation => number_value(&unary.argument).map(|number| -number),
            UnaryOperator::UnaryPlus => number_value(&unary.argument),
            UnaryOperator::LogicalNot
            | UnaryOperator::BitwiseNot
            | UnaryOperator::Typeof
            | UnaryOperator::Void
            | UnaryOperator::Delete => None,
        },
        _ => None,
    }
}

/// Preserve the base's conditional representation and unresolved runtime fallback.
pub(crate) fn expression_order<'a>(
    value: &Expression<'a>,
    allocator: &'a Allocator,
) -> ParsedStyleOrder<'a> {
    match unwrap_syntax_only(value) {
        Expression::ConditionalExpression(conditional) => ParsedStyleOrder::Conditional {
            condition: conditional.test.clone_in(allocator),
            consequent: static_order(&conditional.consequent),
            alternate: static_order(&conditional.alternate),
        },
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
            ParsedStyleOrder::Conditional {
                condition: logical.left.clone_in(allocator),
                consequent: static_order(&logical.right),
                alternate: None,
            }
        }
        value => static_order(value).map_or(ParsedStyleOrder::None, ParsedStyleOrder::Static),
    }
}

/// Parse JSX strings as strings, not through the shared floating-point reader.
pub(crate) fn attribute_order<'a>(
    value: &JSXAttributeValue<'a>,
    allocator: &'a Allocator,
) -> ParsedStyleOrder<'a> {
    match value {
        JSXAttributeValue::StringLiteral(literal) => string_order(literal.value.as_str())
            .map_or(ParsedStyleOrder::None, ParsedStyleOrder::Static),
        JSXAttributeValue::ExpressionContainer(container) => container
            .expression
            .as_expression()
            .map_or(ParsedStyleOrder::None, |value| {
                expression_order(value, allocator)
            }),
        JSXAttributeValue::Element(_) | JSXAttributeValue::Fragment(_) => ParsedStyleOrder::None,
    }
}

/// Explain an invalid explicit order through the existing build-time error family.
pub(crate) fn invalid_order(code: &str) -> String {
    build_time_error(
        "styleOrder",
        code,
        "an explicit styleOrder must be an integer from 1 to 254; a string must be canonical decimal digits without a sign, spaces or leading zeros: use a numeric literal such as 1 or a canonical decimal string such as \"1\"",
    )
}
