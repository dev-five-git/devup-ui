use super::Constant;
use crate::utils::unwrap_syntax_only;
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind, PropertyKind};

pub(super) fn own_scalar(object: &ObjectExpression<'_>, key: &str) -> Option<Constant> {
    if !plain_literal(object) {
        return None;
    }
    let mut result = None;
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        if property.key.static_name()?.as_ref() == key {
            result = match unwrap_syntax_only(&property.value) {
                Expression::StringLiteral(value) => Some(Constant::String(value.value.to_string())),
                Expression::NumericLiteral(value) if value.value.is_finite() => {
                    Some(Constant::Number(value.value))
                }
                Expression::BooleanLiteral(value) => Some(Constant::Bool(value.value)),
                Expression::NullLiteral(_) => Some(Constant::Null),
                _ => None,
            };
        }
    }
    result
}

fn plain_literal(object: &ObjectExpression<'_>) -> bool {
    object.properties.iter().all(|property| match property {
        ObjectPropertyKind::ObjectProperty(property)
            if property.kind == PropertyKind::Init
                && !property.computed
                && !property.method
                && property.key.static_name().is_some_and(|key| {
                    !matches!(
                        key.as_ref(),
                        "__proto__" | "toJSON" | "toString" | "valueOf"
                    )
                }) =>
        {
            match unwrap_syntax_only(&property.value) {
                Expression::StringLiteral(_)
                | Expression::NumericLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_) => true,
                Expression::ObjectExpression(child) => plain_literal(child),
                _ => false,
            }
        }
        ObjectPropertyKind::ObjectProperty(_) | ObjectPropertyKind::SpreadProperty(_) => false,
    })
}
