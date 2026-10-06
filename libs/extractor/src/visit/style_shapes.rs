use crate::utils::unwrap_syntax_only;
use oxc_ast::ast::{Argument, Expression, ObjectPropertyKind};

pub(super) fn malformed_selectors(arguments: &[Argument<'_>]) -> bool {
    fn malformed(expression: &Expression<'_>) -> bool {
        match unwrap_syntax_only(expression) {
            Expression::ObjectExpression(object) => {
                object.properties.iter().any(|property| match property {
                    ObjectPropertyKind::ObjectProperty(property) => {
                        let selector = property.key.static_name().is_some_and(|key| {
                            key.starts_with('_') || key.starts_with('@') || key.contains('&')
                        });
                        (selector
                            && matches!(
                                unwrap_syntax_only(&property.value),
                                Expression::StringLiteral(_) | Expression::NumericLiteral(_)
                            ))
                            || malformed(&property.value)
                    }
                    ObjectPropertyKind::SpreadProperty(spread) => malformed(&spread.argument),
                })
            }
            Expression::ConditionalExpression(value) => {
                malformed(&value.consequent) || malformed(&value.alternate)
            }
            Expression::LogicalExpression(value) => malformed(&value.right),
            Expression::ArrayExpression(value) => value
                .elements
                .iter()
                .filter_map(|value| value.as_expression())
                .any(malformed),
            _ => false,
        }
    }
    arguments
        .iter()
        .filter_map(Argument::as_expression)
        .any(malformed)
}

pub(super) fn conditional_unknown_keys(arguments: &[Argument<'_>]) -> bool {
    fn unknown(expression: &Expression<'_>, conditional: bool) -> bool {
        match unwrap_syntax_only(expression) {
            Expression::ObjectExpression(object) => {
                object.properties.iter().any(|property| match property {
                    ObjectPropertyKind::ObjectProperty(property) => {
                        (conditional
                            && property.computed
                            && crate::utils::get_string_by_property_key(&property.key).is_none())
                            || unknown(&property.value, conditional)
                    }
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        unknown(&spread.argument, conditional)
                    }
                })
            }
            Expression::ConditionalExpression(value) => {
                unknown(&value.consequent, true) || unknown(&value.alternate, true)
            }
            Expression::LogicalExpression(value) => unknown(&value.right, true),
            Expression::ArrayExpression(value) => value
                .elements
                .iter()
                .filter_map(|value| value.as_expression())
                .any(|value| unknown(value, conditional)),
            _ => false,
        }
    }
    arguments
        .iter()
        .filter_map(Argument::as_expression)
        .any(|value| unknown(value, false))
}

pub(super) fn finite_spread(expression: &Expression<'_>) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::ObjectExpression(_) | Expression::ArrayExpression(_) => true,
        Expression::ConditionalExpression(value) => {
            finite_spread(&value.consequent) && finite_spread(&value.alternate)
        }
        _ => false,
    }
}
