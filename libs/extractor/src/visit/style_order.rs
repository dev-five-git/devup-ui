//! `styleOrder` as the stylesheet places rules by: a number, or a condition
//! choosing between numbers, known at build time.

use super::DevupVisitor;
use crate::utils::{
    ParsedStyleOrder, element_error, expression_to_style_order_with, get_str_by_property_key,
    jsx_expression_to_style_order, readable_code,
};
use oxc_allocator::GetAllocator;
use oxc_ast::ast::{Expression, JSXAttributeValue, ObjectPropertyKind};
use oxc_span::GetSpan;

const STYLE_ORDER_VALUE: &str = "`styleOrder` must be a number, or a condition choosing between numbers, since the stylesheet places rules by it before the code runs";

impl<'a> DevupVisitor<'a> {
    pub(super) fn parsed_order(&self, value: &Expression<'a>) -> ParsedStyleOrder<'a> {
        expression_to_style_order_with(value, self.ast.allocator(), &|value| match value {
            Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => true,
            Expression::Identifier(identifier) => self.bindings.is_global_undefined(identifier),
            Expression::UnaryExpression(unary) => {
                unary.operator == oxc_ast::ast::UnaryOperator::Void
                    && super::order::reach(&self.bindings, &unary.argument)
                        == super::order::Reach::Constant
            }
            _ => false,
        })
    }
    /// The `styleOrder` the props `props` of the component `element` write
    pub(super) fn prescan_style_order(
        &mut self,
        element: &str,
        props: &Expression<'a>,
    ) -> ParsedStyleOrder<'a> {
        let Expression::ObjectExpression(object) = props else {
            return ParsedStyleOrder::None;
        };
        let written = object
            .properties
            .iter()
            .find_map(|property| match property {
                ObjectPropertyKind::ObjectProperty(property)
                    if get_str_by_property_key(&property.key)
                        .is_some_and(|key| key == "styleOrder") =>
                {
                    Some(&property.value)
                }
                _ => None,
            });
        let Some(value) = written else {
            return ParsedStyleOrder::None;
        };
        let parsed = self.parsed_order(value);
        if matches!(parsed, ParsedStyleOrder::Unsupported) {
            self.errors.push((
                value.span().start,
                element_error(element, &readable_code(value), STYLE_ORDER_VALUE),
            ));
        }
        parsed
    }

    /// The order the `styleOrder` attribute `value` of the element `element` gives
    pub(super) fn style_order_attribute(
        &mut self,
        element: &str,
        value: &JSXAttributeValue<'a>,
    ) -> ParsedStyleOrder<'a> {
        let parsed = match value {
            JSXAttributeValue::ExpressionContainer(container) => container
                .expression
                .as_expression()
                .map_or(ParsedStyleOrder::None, |value| self.parsed_order(value)),
            _ => jsx_expression_to_style_order(value, self.ast.allocator()),
        };
        if matches!(parsed, ParsedStyleOrder::Unsupported) {
            let (offset, code) = match value {
                JSXAttributeValue::ExpressionContainer(container) => container
                    .expression
                    .as_expression()
                    .map(|value| (value.span().start, readable_code(value)))
                    .unwrap_or_default(),
                JSXAttributeValue::StringLiteral(literal) => {
                    (literal.span.start, format!("{:?}", literal.value))
                }
                JSXAttributeValue::Element(element) => {
                    (element.span.start, "<element>".to_string())
                }
                JSXAttributeValue::Fragment(fragment) => {
                    (fragment.span.start, "<>...</>".to_string())
                }
            };
            self.errors
                .push((offset, element_error(element, &code, STYLE_ORDER_VALUE)));
        }
        parsed
    }
}
