use super::style_inputs::StyleInputs;
use crate::{
    ExtractStyleProp,
    utils::{get_string_by_literal_expression, readable_code, unwrap_syntax_only},
};
use css::{add_selector_params, style_selector::StyleSelector};
use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectExpression, ObjectPropertyKind};
use oxc_span::GetSpan;

pub(crate) const SELECTOR_PARAMS: &str = "selector params must be a statically readable argument list; functional-only selectors require a nonempty argument list";

fn reject<'a>(expression: &Expression<'a>) -> Box<ExtractStyleProp<'a>> {
    Box::new(super::extract_style_from_expression::misplaced(
        expression.span().start,
        readable_code(expression),
        SELECTOR_PARAMS,
    ))
}

fn requires_argument(selector: &StyleSelector) -> bool {
    css::selector_params::parameter_name(selector)
        .is_some_and(|name| !matches!(name, "host" | "cue" | "cue-region"))
}

pub(super) fn resolve<'a>(
    inputs: StyleInputs<'_, 'a>,
    object: &ObjectExpression<'a>,
    selector: &Option<StyleSelector>,
) -> Result<Option<StyleSelector>, Box<ExtractStyleProp<'a>>> {
    let Some(selector) = selector else {
        return Ok(None);
    };
    let Some(value) = object
        .properties
        .iter()
        .find_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property)
                if property.key.name().is_some_and(|name| name == "params") =>
            {
                Some(&property.value)
            }
            _ => None,
        })
    else {
        return Ok(Some(selector.clone()));
    };
    let required = requires_argument(selector);
    let array = match unwrap_syntax_only(value) {
        Expression::ArrayExpression(array) => array,
        _ if !required && inputs.absent(value) => return Ok(Some(selector.clone())),
        _ => return Err(reject(value)),
    };
    let mut params = String::new();
    let mut empty_argument = None;
    for element in &array.elements {
        let expression = match element {
            ArrayExpressionElement::SpreadElement(spread) => return Err(reject(&spread.argument)),
            ArrayExpressionElement::Elision(_) => continue,
            _ => element.to_expression(),
        };
        let Some(text) = get_string_by_literal_expression(expression) else {
            return Err(reject(expression));
        };
        if text.is_empty() {
            empty_argument = empty_argument.or(Some(expression));
            continue;
        }
        if !params.is_empty() {
            params.push(',');
        }
        params.push_str(&text);
    }
    if params.is_empty() {
        return if required {
            Err(reject(empty_argument.unwrap_or(value)))
        } else {
            Ok(Some(selector.clone()))
        };
    }
    Ok(Some(add_selector_params(selector.clone(), &params)))
}
