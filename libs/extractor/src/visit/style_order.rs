//! `styleOrder` as the stylesheet places rules by: a number, or a condition
//! choosing between numbers, known at build time.

use super::DevupVisitor;
use crate::style_order::{STYLE_ORDER_VALUE, supported_order};
use crate::utils::{
    ParsedStyleOrder, element_error, get_str_by_property_key, jsx_expression_to_style_order,
    readable_code, unwrap_syntax_only,
};
use oxc_allocator::GetAllocator;
use oxc_ast::ast::{Expression, JSXAttributeValue, ObjectPropertyKind};
use oxc_span::GetSpan;

impl<'a> DevupVisitor<'a> {
    pub(super) fn parsed_order(&self, value: &Expression<'a>) -> ParsedStyleOrder<'a> {
        if supported_order(value) {
            crate::utils::expression_to_style_order(value, self.ast.allocator())
        } else {
            ParsedStyleOrder::Unsupported
        }
    }
    /// The `styleOrder` the props `props` of the component `element` write
    pub(super) fn prescan_style_order(
        &mut self,
        element: &str,
        props: &Expression<'a>,
    ) -> ParsedStyleOrder<'a> {
        let Expression::ObjectExpression(object) = unwrap_syntax_only(props) else {
            return ParsedStyleOrder::None;
        };
        let written = object
            .properties
            .iter()
            .rev()
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
        let mut errors = Vec::new();
        crate::style_order_validation::validate_value(value, &self.style_values, &mut errors);
        if errors.is_empty() && matches!(parsed, ParsedStyleOrder::Unsupported) {
            self.errors.push((
                value.span().start,
                element_error(element, &readable_code(value), STYLE_ORDER_VALUE),
            ));
        }
        self.errors.extend(errors);
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
        let mut errors = Vec::new();
        crate::style_order_validation::validate_attribute(value, &self.style_values, &mut errors);
        if errors.is_empty()
            && matches!(parsed, ParsedStyleOrder::Unsupported)
            && let JSXAttributeValue::ExpressionContainer(container) = value
        {
            let (offset, code) = container
                .expression
                .as_expression()
                .map_or_default(|value| (value.span().start, readable_code(value)));
            self.errors
                .push((offset, element_error(element, &code, STYLE_ORDER_VALUE)));
        }
        self.errors.extend(errors);
        parsed
    }
}
