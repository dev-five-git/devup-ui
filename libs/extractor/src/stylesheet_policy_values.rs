//! Nonthrowing value shapes used by the stylesheet dispatch proof.

use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKind};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use rustc_hash::FxHashMap;

use super::{MATH_CONSTANTS, Proof};
use crate::utils::unwrap_syntax_only;

#[derive(Clone, PartialEq)]
pub(super) enum Value {
    Scalar,
    Text(String),
    Number(f64),
    Nullish,
    Object(FxHashMap<String, Self>),
    Array(Vec<Self>),
    /// The expression is nonthrowing but its resulting shape is not known.
    Unknown,
}

impl Value {
    const fn primitive(&self) -> bool {
        matches!(
            self,
            Self::Scalar | Self::Text(_) | Self::Number(_) | Self::Nullish
        )
    }

    pub(super) const fn iterable(&self) -> bool {
        matches!(self, Self::Array(_) | Self::Text(_))
    }

    fn key(&self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text.clone()),
            Self::Number(number) => Some(crate::utils::js_number_string(*number)),
            _ => None,
        }
    }

    fn member(self, key: &str) -> Option<Self> {
        match self {
            Self::Object(object) => match object.get(key) {
                Some(value) => Some(value.clone()),
                None if key == "constructor"
                    || key == "__proto__"
                    || key == "toString"
                    || key == "valueOf" =>
                {
                    None
                }
                None => Some(Self::Nullish),
            },
            Self::Array(array) => {
                if key == "length" {
                    return Some(Self::Scalar);
                }
                let index = key.parse::<usize>().ok()?;
                Some(array.get(index).cloned().unwrap_or(Self::Nullish))
            }
            Self::Text(text) => {
                if key == "length" {
                    return Some(Self::Scalar);
                }
                let index = key.parse::<usize>().ok()?;
                Some(
                    text.encode_utf16()
                        .nth(index)
                        .map_or(Self::Nullish, |unit| {
                            char::from_u32(u32::from(unit))
                                .map_or(Self::Scalar, |c| Self::Text(c.to_string()))
                        }),
                )
            }
            Self::Scalar | Self::Number(_) | Self::Nullish | Self::Unknown => None,
        }
    }
}

impl Proof<'_> {
    pub(super) fn value(&self, expression: &Expression<'_>) -> Option<Value> {
        let expression = unwrap_syntax_only(expression);
        match expression {
            Expression::BooleanLiteral(_) => Some(Value::Scalar),
            Expression::NullLiteral(_) => Some(Value::Nullish),
            Expression::NumericLiteral(number) => Some(Value::Number(number.value)),
            Expression::StringLiteral(text) => Some(Value::Text(text.value.to_string())),
            Expression::Identifier(identifier) => match self.symbol(identifier) {
                Some(symbol) => self.values.get(&symbol).cloned(),
                None => match identifier.name.as_str() {
                    "undefined" => Some(Value::Nullish),
                    "NaN" | "Infinity" => Some(Value::Scalar),
                    _ => None,
                },
            },
            Expression::ArrayExpression(array) => {
                let mut values = Vec::new();
                for element in &array.elements {
                    match element {
                        ArrayExpressionElement::Elision(_) => values.push(Value::Nullish),
                        ArrayExpressionElement::SpreadElement(spread) => {
                            match self.value(&spread.argument)? {
                                Value::Array(items) => values.extend(items),
                                Value::Text(text) => {
                                    values.extend(text.chars().map(|c| Value::Text(c.to_string())));
                                }
                                _ => return None,
                            }
                        }
                        element => values.push(self.value(element.to_expression())?),
                    }
                }
                Some(Value::Array(values))
            }
            Expression::ObjectExpression(object) => {
                let mut values = FxHashMap::default();
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::SpreadProperty(spread) => match self
                            .value(&spread.argument)?
                        {
                            Value::Object(properties) => values.extend(properties),
                            Value::Array(items) => values.extend(
                                items
                                    .into_iter()
                                    .enumerate()
                                    .map(|(index, value)| (index.to_string(), value)),
                            ),
                            Value::Text(text) => values.extend(
                                text.encode_utf16().enumerate().map(|(index, unit)| {
                                    (
                                        index.to_string(),
                                        char::from_u32(u32::from(unit))
                                            .map_or(Value::Scalar, |c| Value::Text(c.to_string())),
                                    )
                                }),
                            ),
                            Value::Nullish | Value::Scalar | Value::Number(_) => {}
                            Value::Unknown => return None,
                        },
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.method || property.kind != PropertyKind::Init {
                                return None;
                            }
                            let key = if property.computed {
                                self.value(property.key.as_expression()?)?.key()?
                            } else {
                                property.key.static_name()?.to_string()
                            };
                            if key == "__proto__" {
                                return None;
                            }
                            values.insert(key, self.value(&property.value)?);
                        }
                    }
                }
                Some(Value::Object(values))
            }
            Expression::StaticMemberExpression(member) => {
                if self.is_global(&member.object, "Math")
                    && MATH_CONSTANTS.contains(&member.property.name.as_str())
                {
                    return Some(Value::Scalar);
                }
                self.value(&member.object)?
                    .member(member.property.name.as_str())
            }
            Expression::ComputedMemberExpression(member) => self
                .value(&member.object)?
                .member(&self.value(&member.expression)?.key()?),
            Expression::TemplateLiteral(template) => {
                let mut text = String::new();
                let mut exact = true;
                for (index, quasi) in template.quasis.iter().enumerate() {
                    text.push_str(quasi.value.cooked.as_ref()?.as_str());
                    if let Some(expression) = template.expressions.get(index) {
                        let value = self.value(expression)?;
                        if !value.primitive() {
                            return None;
                        }
                        match value.key() {
                            Some(key) => text.push_str(&key),
                            None => exact = false,
                        }
                    }
                }
                Some(if exact {
                    Value::Text(text)
                } else {
                    Value::Scalar
                })
            }
            Expression::UnaryExpression(unary) => {
                if unary.operator == UnaryOperator::Delete
                    || !self.value(&unary.argument)?.primitive()
                {
                    return None;
                }
                Some(Value::Scalar)
            }
            Expression::BinaryExpression(binary) => {
                if matches!(
                    binary.operator,
                    BinaryOperator::Exponential | BinaryOperator::In | BinaryOperator::Instanceof
                ) || !self.value(&binary.left)?.primitive()
                    || !self.value(&binary.right)?.primitive()
                {
                    return None;
                }
                Some(Value::Scalar)
            }
            Expression::ConditionalExpression(conditional) => {
                self.value(&conditional.test)?;
                let left = self.value(&conditional.consequent)?;
                let right = self.value(&conditional.alternate)?;
                Some(if left == right { left } else { Value::Unknown })
            }
            Expression::LogicalExpression(logical) => {
                let left = self.value(&logical.left)?;
                let right = self.value(&logical.right)?;
                Some(if left == right { left } else { Value::Unknown })
            }
            Expression::SequenceExpression(sequence) => {
                let mut value = Value::Nullish;
                for expression in &sequence.expressions {
                    value = self.value(expression)?;
                }
                Some(value)
            }
            Expression::CallExpression(call) if self.folds(call) => {
                for argument in &call.arguments {
                    if argument.is_spread() || !self.value(argument.to_expression())?.primitive() {
                        return None;
                    }
                }
                Some(Value::Scalar)
            }
            _ if self.creation(expression) => Some(Value::Scalar),
            _ => None,
        }
    }
}

pub(super) fn literal(code: &str) -> Option<Value> {
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, code, oxc_span::SourceType::default())
        .parse_expression()
        .ok()?;
    let scoping = oxc_semantic::Scoping::default();
    let proof = Proof {
        scoping: &scoping,
        apis: FxHashMap::default(),
        namespaces: rustc_hash::FxHashSet::default(),
        values: FxHashMap::default(),
    };
    proof.value(&parsed)
}
