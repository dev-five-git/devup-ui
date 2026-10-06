use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKey};
use oxc_span::Span;

pub(super) fn place(value: &mut Expression<'_>, origin: Span) {
    match value {
        Expression::StringLiteral(value) => value.span = origin,
        Expression::NumericLiteral(value) => value.span = origin,
        Expression::BooleanLiteral(value) => value.span = origin,
        Expression::NullLiteral(value) => value.span = origin,
        Expression::ObjectExpression(value) => {
            value.span = origin;
            for property in &mut value.properties {
                if let ObjectPropertyKind::ObjectProperty(property) = property {
                    property.span = origin;
                    if let PropertyKey::StringLiteral(key) = &mut property.key {
                        key.span = origin;
                    }
                    place(&mut property.value, origin);
                }
            }
        }
        Expression::ArrayExpression(value) => {
            value.span = origin;
            for element in &mut value.elements {
                if let Some(element) = element.as_expression_mut() {
                    place(element, origin);
                }
            }
        }
        _ => {}
    }
}
