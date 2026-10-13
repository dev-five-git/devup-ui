//! Reserved cascade metadata shared by JSX and declaration objects.

use boa_engine::{Context, JsString, Source};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind};
use oxc_syntax::operator::UnaryOperator;

use crate::ExtractStyleProp;
use crate::utils::{build_time_error, expression_to_code, unwrap_syntax_only};

mod parsing;
mod rejection;
pub(crate) use parsing::{OrderError, parse_typed};

#[derive(Clone, Copy)]
pub(crate) enum MetadataContext {
    Ordinary,
    Order,
}

impl MetadataContext {
    pub(crate) const fn selected(metadata: bool) -> Self {
        if metadata {
            Self::Order
        } else {
            Self::Ordinary
        }
    }

    pub(crate) const fn preserves_order(self) -> bool {
        match self {
            Self::Ordinary => false,
            Self::Order => true,
        }
    }
}

/// Canonical decimal digits naming a user-addressable layer.
pub(crate) fn string_order(value: &str) -> Option<u8> {
    if !value
        .as_bytes()
        .first()
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

/// A Number integer or a canonical cooked string, without numeric truncation.
pub(crate) fn static_order(value: &Expression<'_>) -> Option<u8> {
    match unwrap_syntax_only(value) {
        Expression::StringLiteral(literal) => string_order(literal.value.as_str()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|cooked| string_order(cooked.as_str())),
        Expression::NumericLiteral(_)
        | Expression::UnaryExpression(_)
        | Expression::BinaryExpression(_) => number_value(value)
            .and_then(|number| number.to_string().parse::<u8>().ok())
            .filter(|order| (1..=254).contains(order)),
        _ => None,
    }
}

fn truthiness(value: &Expression<'_>) -> Option<bool> {
    match unwrap_syntax_only(value) {
        Expression::BooleanLiteral(literal) => Some(literal.value),
        Expression::NullLiteral(_) => Some(false),
        Expression::StringLiteral(literal) => Some(!literal.value.is_empty()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .map(|value| !value.is_empty()),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            truthiness(&unary.argument).map(|value| !value)
        }
        value => number_value(value).map(|number| number != 0.0 && !number.is_nan()),
    }
}

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
            UnaryOperator::UnaryPlus => number_value(&unary.argument),
            UnaryOperator::UnaryNegation => number_value(&unary.argument).map(|number| -number),
            _ => None,
        },
        Expression::BinaryExpression(binary)
            if number_value(&binary.left).is_some() && number_value(&binary.right).is_some() =>
        {
            let code = expression_to_code(value);
            Context::default()
                .eval(Source::from_bytes(code.as_bytes()))
                .ok()?
                .as_number()
        }
        _ => None,
    }
}

/// Explain a reserved metadata value through the existing located-error family.
pub(crate) fn invalid_order(code: &str) -> String {
    build_time_error(
        "styleOrder",
        code,
        "an explicit styleOrder must be an ECMAScript Number integer from 1 to 254 or canonical decimal string without signs, spaces or leading zeros",
    )
}

pub(crate) fn reserved(name: &str) -> bool {
    matches!(name, "styleOrder" | "style-order")
}

pub(crate) fn no_effect(api: &str, offset: u32) -> (u32, String) {
    let (api, reason) = if api == "fontFaces" {
        (
            "globalCss",
            "styleOrder has no effect in fontFaces descriptors",
        )
    } else {
        (
            api,
            "styleOrder is reserved cascade metadata and has no effect in this API",
        )
    };
    (offset, build_time_error(api, "styleOrder", reason))
}

pub(crate) fn reject(expression: &Expression<'_>, api: &str, errors: &mut Vec<(u32, String)>) {
    rejection::reject(expression, api, errors);
}

pub(crate) fn reject_object(
    object: &ObjectExpression<'_>,
    api: &str,
    errors: &mut Vec<(u32, String)>,
) {
    rejection::reject_object(object, api, errors);
}

/// Finite alternatives chosen by source code, with absence only synthesized by &&.
#[derive(Debug)]
pub(crate) enum Order<'a> {
    Absent,
    Static(u8),
    Conditional {
        test: Expression<'a>,
        yes: Box<Order<'a>>,
        no: Box<Order<'a>>,
    },
}

/// Parse every explicit branch, retaining the location of the invalid value.
pub(crate) fn parse<'a>(
    value: &Expression<'a>,
    allocator: &'a Allocator,
) -> Result<Order<'a>, (u32, String)> {
    parse_typed(value, allocator).map_err(|error| error.diagnostic)
}

/// Remove reserved keys before CSS extraction; the last written order controls the object.
pub(crate) fn take<'a>(
    object: &mut ObjectExpression<'a>,
    allocator: &'a Allocator,
) -> Option<Result<Order<'a>, OrderError>> {
    let mut order = None;
    object.properties.retain(|property| {
        if let ObjectPropertyKind::ObjectProperty(property) = property
            && property
                .key
                .static_name()
                .or_else(|| crate::utils::get_str_by_property_key(&property.key))
                .is_some_and(|name| reserved(&name))
        {
            order = Some(parse_typed(&property.value, allocator));
            false
        } else {
            true
        }
    });
    order
}

/// Lower metadata into existing style branches; inner explicit orders have already been applied.
pub(crate) fn apply<'a>(
    order: Order<'a>,
    styles: Vec<ExtractStyleProp<'a>>,
    allocator: &'a Allocator,
) -> Vec<ExtractStyleProp<'a>> {
    apply_with_payload(order, styles, allocator, |expression, allocator| {
        expression.clone_in(allocator)
    })
}

pub(crate) fn apply_with_payload<'a, E>(
    order: Order<'a>,
    styles: Vec<ExtractStyleProp<'a, E>>,
    allocator: &'a Allocator,
    clone_payload: impl Fn(&E, &'a Allocator) -> E,
) -> Vec<ExtractStyleProp<'a, E>> {
    apply_payload(order, styles, allocator, &clone_payload)
}

fn apply_payload<'a, E, F: Fn(&E, &'a Allocator) -> E>(
    order: Order<'a>,
    mut styles: Vec<ExtractStyleProp<'a, E>>,
    allocator: &'a Allocator,
    clone_payload: &F,
) -> Vec<ExtractStyleProp<'a, E>> {
    match order {
        Order::Absent => styles,
        Order::Static(order) => {
            for style in &mut styles {
                fill(style, order);
            }
            styles
        }
        Order::Conditional { test, yes, no } => {
            let alternate = styles
                .iter()
                .map(|style| style.clone_payload_in(allocator, clone_payload))
                .collect();
            vec![ExtractStyleProp::Conditional {
                condition: test,
                consequent: Some(Box::new(ExtractStyleProp::StaticArray(apply_payload(
                    *yes,
                    styles,
                    allocator,
                    clone_payload,
                )))),
                alternate: Some(Box::new(ExtractStyleProp::StaticArray(apply_payload(
                    *no,
                    alternate,
                    allocator,
                    clone_payload,
                )))),
            }]
        }
    }
}

/// Inherit an outer order without replacing inner metadata, for static and dynamic atoms alike.
fn fill<E>(prop: &mut ExtractStyleProp<'_, E>, order: u8) {
    match prop {
        ExtractStyleProp::Static(value) => value.set_style_order(order),
        ExtractStyleProp::StaticArray(styles) => {
            for style in styles {
                fill(style, order);
            }
        }
        ExtractStyleProp::Conditional {
            consequent,
            alternate,
            ..
        } => {
            for branch in [consequent, alternate].into_iter().flatten() {
                fill(branch, order);
            }
        }
        ExtractStyleProp::Enum { map, .. } => {
            for style in map.values_mut().flatten() {
                fill(style, order);
            }
        }
        ExtractStyleProp::MemberExpression { map, .. } => {
            for style in map.values_mut() {
                fill(style, order);
            }
        }
        ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::Unreadable { .. }
        | ExtractStyleProp::Diagnostic { .. } => {}
    }
}
