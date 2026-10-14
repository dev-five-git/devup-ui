use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKind};

/// Only literal syntax after the existing inliner is a proof of absent runtime evaluation.
pub(crate) fn literal_source(source: &Expression<'_>) -> bool {
    let source = crate::utils::unwrap_syntax_only(source);
    match source {
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::Elision(_) => true,
            ArrayExpressionElement::SpreadElement(_) => false,
            _ => element.as_expression().is_some_and(literal_source),
        }),
        Expression::ObjectExpression(object) => {
            object.properties.iter().all(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    property.kind == PropertyKind::Init
                        && !property.method
                        && !property.computed
                        && literal_source(&property.value)
                }
                ObjectPropertyKind::SpreadProperty(_) => false,
            })
        }
        _ => primitive_source(source),
    }
}

fn primitive_source(source: &Expression<'_>) -> bool {
    let source = crate::utils::unwrap_syntax_only(source);
    match source {
        Expression::NullLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_) => true,
        Expression::TemplateLiteral(template) => {
            template.expressions.iter().all(primitive_source)
                && crate::utils::get_string_by_literal_expression(source).is_some()
        }
        Expression::UnaryExpression(unary) => {
            primitive_source(&unary.argument)
                && crate::utils::get_string_by_literal_expression(source).is_some()
        }
        _ => false,
    }
}

pub(crate) fn class_only(style: &crate::ExtractStyleProp<'_>) -> bool {
    use crate::{ExtractStyleProp, ExtractStyleValue};
    match style {
        ExtractStyleProp::Static(
            ExtractStyleValue::Static(_) | ExtractStyleValue::Typography(_),
        ) => true,
        ExtractStyleProp::StaticArray(styles) => styles.iter().all(class_only),
        ExtractStyleProp::Static(_)
        | ExtractStyleProp::Evaluated { .. }
        | ExtractStyleProp::Conditional { .. }
        | ExtractStyleProp::Enum { .. }
        | ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::MemberExpression { .. }
        | ExtractStyleProp::Unreadable { .. } => false,
    }
}
