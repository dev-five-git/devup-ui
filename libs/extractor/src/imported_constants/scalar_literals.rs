use super::Constant;
use crate::utils::unwrap_syntax_only;
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind, PropertyKind};

pub(super) fn own_scalar(object: &ObjectExpression<'_>, key: &str) -> Option<Constant> {
    literal_record(object, Some(key))?.scalar
}

struct LiteralRecord {
    scalar: Option<Constant>,
}

fn literal_record(object: &ObjectExpression<'_>, key: Option<&str>) -> Option<LiteralRecord> {
    let mut result = None;
    for property in &object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        let name = property.key.static_name()?;
        if property.kind != PropertyKind::Init
            || property.computed
            || property.method
            || matches!(
                name.as_ref(),
                "__proto__" | "toJSON" | "toString" | "valueOf"
            )
        {
            return None;
        }
        let selected = key == Some(name.as_ref());
        let value = match unwrap_syntax_only(&property.value) {
            Expression::StringLiteral(value) => {
                selected.then(|| Constant::String(value.value.to_string()))
            }
            Expression::NumericLiteral(value) => {
                (selected && value.value.is_finite()).then_some(Constant::Number(value.value))
            }
            Expression::BooleanLiteral(value) => selected.then_some(Constant::Bool(value.value)),
            Expression::NullLiteral(_) => selected.then_some(Constant::Null),
            Expression::ObjectExpression(child) => {
                literal_record(child, None)?;
                None
            }
            _ => return None,
        };
        if selected {
            result = value;
        }
    }
    Some(LiteralRecord { scalar: result })
}

#[cfg(test)]
#[path = "scalar_literal_tests.rs"]
mod tests;
