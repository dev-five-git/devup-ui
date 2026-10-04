use oxc_ast::ast::{Expression, UnaryOperator};

use crate::{ExtractStyleValue, utils::js_number_literal};

/// An exact scalar captured from a dynamic namespace default or call argument.
#[derive(Debug, Clone)]
pub enum Scalar {
    Undefined,
    Null,
    Text(String),
    Number(f64),
    Boolean(bool),
}

impl Scalar {
    pub fn literal(value: &Expression<'_>) -> Option<Self> {
        let value = value.without_parentheses();
        if let Some(number) = js_number_literal(value) {
            return Some(Self::Number(number));
        }
        match value {
            Expression::StringLiteral(value) => Some(Self::Text(value.value.to_string())),
            Expression::NullLiteral(_) => Some(Self::Null),
            Expression::BooleanLiteral(value) => Some(Self::Boolean(value.value)),
            Expression::UnaryExpression(value)
                if value.operator == UnaryOperator::Void
                    && Self::literal(&value.argument).is_some() =>
            {
                Some(Self::Undefined)
            }
            _ => None,
        }
    }
}

/// A dynamic namespace's exact parameter defaults and property mappings.
#[derive(Debug, Clone, Default)]
pub struct DynamicNamespace {
    pub defaults: Vec<Option<Scalar>>,
    /// Parameter index, CSS variable, numeric unit, CSS property.
    pub css_vars: Vec<(usize, String, &'static str, String)>,
}

/// Information required to specialize a call or capture its runtime arguments.
#[derive(Debug, Clone)]
pub struct StylexDynamicInfo {
    pub class_name: String,
    pub namespace: DynamicNamespace,
    pub styles: Vec<ExtractStyleValue>,
}
