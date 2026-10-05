//! Validate only the root order fields consumed by style extraction.

use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::ast::{Expression, JSXAttributeValue, ObjectPropertyKind};
use oxc_span::GetSpan;
use oxc_syntax::operator::{LogicalOperator, UnaryOperator};

use crate::style_order::{invalid_order, static_order, string_order};
use crate::style_values::StyleValues;
use crate::utils::{get_str_by_property_key, is_pure, readable_code, unwrap_syntax_only};

struct Validation<'s> {
    bindings: &'s StyleValues,
    errors: &'s mut Vec<(u32, String)>,
}

impl Validation<'_> {
    fn value(&mut self, value: &Expression<'_>) {
        let value = unwrap_syntax_only(value);
        if static_order(value).is_some() {
            return;
        }
        match value {
            Expression::ConditionalExpression(conditional) => {
                self.value(&conditional.consequent);
                self.value(&conditional.alternate);
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                self.value(&logical.right);
            }
            value if self.static_invalid(value) => {
                let code = match value {
                    Expression::StringLiteral(literal) => literal
                        .raw
                        .as_ref()
                        .map_or_else(|| readable_code(value), ToString::to_string),
                    _ => readable_code(value),
                };
                self.errors.push((value.span().start, invalid_order(&code)));
            }
            _ => {}
        }
    }

    fn static_invalid(&self, value: &Expression<'_>) -> bool {
        match value {
            Expression::NumericLiteral(_)
            | Expression::StringLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::ArrayExpression(_)
            | Expression::ObjectExpression(_)
            | Expression::ClassExpression(_)
            | Expression::JSXElement(_)
            | Expression::JSXFragment(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_) => true,
            Expression::TemplateLiteral(template) => template.expressions.is_empty(),
            Expression::Identifier(identifier) => {
                matches!(identifier.name.as_str(), "undefined" | "NaN" | "Infinity")
                    && self.bindings.symbol(value).is_none()
            }
            Expression::UnaryExpression(unary) => match unary.operator {
                UnaryOperator::Void => is_pure(&unary.argument),
                UnaryOperator::UnaryNegation
                | UnaryOperator::UnaryPlus
                | UnaryOperator::LogicalNot
                | UnaryOperator::BitwiseNot
                | UnaryOperator::Typeof
                | UnaryOperator::Delete => {
                    let argument = unwrap_syntax_only(&unary.argument);
                    static_order(argument).is_some() || self.static_invalid(argument)
                }
            },
            _ => false,
        }
    }

    fn rules(&mut self, value: &Expression<'_>) {
        match unwrap_syntax_only(value) {
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property)
                            if get_str_by_property_key(&property.key).as_deref()
                                == Some("styleOrder") =>
                        {
                            self.value(&property.value);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => self.rules(&spread.argument),
                        ObjectPropertyKind::ObjectProperty(_) => {}
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(value) = element.as_expression() {
                        self.rules(value);
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.rules(&conditional.consequent);
                self.rules(&conditional.alternate);
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                self.rules(&logical.right);
            }
            _ => {}
        }
    }
}

/// Check declaration roots and composition parts, never arbitrary child data.
pub(crate) fn validate_rules(
    value: &Expression<'_>,
    bindings: &StyleValues,
    errors: &mut Vec<(u32, String)>,
) {
    Validation { bindings, errors }.rules(value);
}

/// Frame declaration roots use the shared object-style extractor.
pub(crate) fn validate_frames(
    value: &Expression<'_>,
    bindings: &StyleValues,
    errors: &mut Vec<(u32, String)>,
) {
    if let Expression::ObjectExpression(object) = unwrap_syntax_only(value) {
        let mut validation = Validation { bindings, errors };
        for property in &object.properties {
            if let ObjectPropertyKind::ObjectProperty(property) = property {
                validation.rules(&property.value);
            }
        }
    }
}

/// Reject invalid explicit JSX values at their own span before they are consumed.
pub(crate) fn validate_attribute(
    value: &JSXAttributeValue<'_>,
    bindings: &StyleValues,
    errors: &mut Vec<(u32, String)>,
) {
    match value {
        JSXAttributeValue::ExpressionContainer(container) => {
            if let Some(value) = container.expression.as_expression() {
                Validation { bindings, errors }.value(value);
            }
        }
        JSXAttributeValue::StringLiteral(literal) => {
            if string_order(literal.value.as_str()).is_none() {
                let code = literal
                    .raw
                    .as_ref()
                    .map_or_else(|| format!("{:?}", literal.value), ToString::to_string);
                errors.push((literal.span.start, invalid_order(&code)));
            }
        }
        JSXAttributeValue::Element(element) => {
            let allocator = Allocator::default();
            let value = Expression::JSXElement(element.clone_in(&allocator));
            errors.push((element.span.start, invalid_order(&readable_code(&value))));
        }
        JSXAttributeValue::Fragment(fragment) => {
            let allocator = Allocator::default();
            let value = Expression::JSXFragment(fragment.clone_in(&allocator));
            errors.push((fragment.span.start, invalid_order(&readable_code(&value))));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    #[test]
    fn style_order_generated_literals_use_readable_fallbacks() {
        let allocator = Allocator::default();
        let builder = oxc_ast::builder::AstBuilder::new(&allocator);
        let bindings = StyleValues::default();
        let mut errors = Vec::new();
        let value = Expression::new_string_literal(oxc_span::SPAN, "01", None, &builder);
        Validation {
            bindings: &bindings,
            errors: &mut errors,
        }
        .value(&value);
        let attribute = JSXAttributeValue::new_string_literal(oxc_span::SPAN, "01", None, &builder);
        validate_attribute(&attribute, &bindings, &mut errors);
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(|(_, error)| error.contains("`\"01\"`")));
    }

    #[test]
    fn style_order_empty_jsx_and_non_frame_values_remain_absent() {
        let allocator = Allocator::default();
        let bindings = StyleValues::default();
        let parsed = Parser::new(
            &allocator,
            "<Box styleOrder={/* empty */} />",
            SourceType::tsx(),
        )
        .parse();
        assert_eq!(parsed.diagnostics.len(), 1);
        let oxc_ast::ast::Statement::ExpressionStatement(statement) = &parsed.program.body[0]
        else {
            panic!("expected expression statement")
        };
        let value = &statement.expression;
        let Expression::JSXElement(element) = value else {
            panic!("expected JSX")
        };
        let oxc_ast::ast::JSXAttributeItem::Attribute(attribute) =
            &element.opening_element.attributes[0]
        else {
            panic!("expected attribute")
        };
        let Some(attribute) = attribute.value.as_ref() else {
            panic!("expected attribute value")
        };
        let mut errors = Vec::new();
        validate_attribute(attribute, &bindings, &mut errors);
        validate_frames(value, &bindings, &mut errors);
        assert_eq!(errors, vec![]);
        assert!(matches!(
            crate::style_order::attribute_order(attribute, &allocator),
            crate::utils::ParsedStyleOrder::None
        ));
    }
}
