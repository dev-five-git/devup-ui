//! `styleOrder` as the stylesheet places rules by: a number, or a condition
//! choosing between numbers, known at build time.

use super::DevupVisitor;
use crate::utils::{
    ParsedStyleOrder, element_error, expression_to_style_order, get_str_by_property_key,
    jsx_expression_to_style_order, lowered_style_order, readable_code,
};
use oxc_allocator::GetAllocator;
use oxc_ast::ast::{Expression, JSXAttributeValue, ObjectPropertyKind};

const STYLE_ORDER_VALUE: &str = "`styleOrder` must be a number, or a condition choosing between numbers, since the stylesheet places rules by it before the code runs";

impl<'a> DevupVisitor<'a> {
    pub(super) fn parsed_order(&self, value: &Expression<'a>) -> ParsedStyleOrder<'a> {
        expression_to_style_order(value, self.ast.allocator())
    }

    fn checked_order(&mut self, element: &str, value: &Expression<'a>) -> ParsedStyleOrder<'a> {
        match crate::style_order::parse_typed(value, self.ast.allocator()) {
            Ok(order) => lowered_style_order(order),
            Err(error) => {
                self.error_disposition.include(error.disposition);
                self.errors.push((
                    error.diagnostic.0,
                    element_error(element, &readable_code(value), STYLE_ORDER_VALUE),
                ));
                ParsedStyleOrder::Unsupported
            }
        }
    }

    /// The `styleOrder` the props `props` of the component `element` write
    pub(super) fn prescan_style_order(
        &mut self,
        element: &str,
        props: &mut Expression<'a>,
    ) -> ParsedStyleOrder<'a> {
        let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only_mut(props)
        else {
            return ParsedStyleOrder::None;
        };
        crate::extractor::extract_style_from_expression::flatten_spreads(&self.ast, object);
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
        let parsed = self.checked_order(element, value);
        if let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only_mut(props) {
            object.properties.retain(|property| !matches!(property, ObjectPropertyKind::ObjectProperty(property) if get_str_by_property_key(&property.key).is_some_and(|key| key == "styleOrder")));
        }
        parsed
    }

    /// The order the `styleOrder` attribute `value` of the element `element` gives
    pub(super) fn style_order_attribute(
        &mut self,
        element: &str,
        value: &JSXAttributeValue<'a>,
    ) -> ParsedStyleOrder<'a> {
        let (offset, code) = match value {
            JSXAttributeValue::ExpressionContainer(container) => {
                return container
                    .expression
                    .as_expression()
                    .map_or(ParsedStyleOrder::None, |value| {
                        self.checked_order(element, value)
                    });
            }
            JSXAttributeValue::StringLiteral(literal) => {
                let parsed = jsx_expression_to_style_order(value, self.ast.allocator());
                if !matches!(parsed, ParsedStyleOrder::Unsupported) {
                    return parsed;
                }
                self.error_disposition
                    .include(crate::ErrorDisposition::Definitive);
                (literal.span.start, format!("{:?}", literal.value))
            }
            JSXAttributeValue::Element(element) => (element.span.start, "<element>".to_string()),
            JSXAttributeValue::Fragment(fragment) => (fragment.span.start, "<>...</>".to_string()),
        };
        self.errors
            .push((offset, element_error(element, &code, STYLE_ORDER_VALUE)));
        ParsedStyleOrder::Unsupported
    }
}
