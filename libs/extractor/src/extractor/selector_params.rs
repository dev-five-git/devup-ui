use super::style_inputs::StyleInputs;
use crate::{
    ExtractStyleProp,
    utils::{get_string_by_literal_expression, readable_code, unwrap_syntax_only},
};
use css::{add_selector_params, style_selector::StyleSelector};
use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectExpression, ObjectPropertyKind};
use oxc_span::GetSpan;

pub(crate) const SELECTOR_PARAMS: &str = "selector params must be a statically readable argument list; functional-only selectors require a nonempty argument list; computed params arrays must be dense: holes, null and undefined share a null representation after build-time JSON evaluation and are refused; return a dense array";

fn reject<'a>(expression: &Expression<'a>) -> Box<ExtractStyleProp<'a>> {
    Box::new(super::extract_style_from_expression::misplaced(
        expression.span().start,
        readable_code(expression),
        SELECTOR_PARAMS,
    ))
}

pub(crate) fn requires_argument(selector: &StyleSelector) -> bool {
    css::selector_params::parameter_name(selector)
        .is_some_and(|name| !matches!(name, "host" | "cue" | "cue-region"))
}

pub(crate) enum ArgumentList<I, W> {
    List(I, W),
    Absent(W),
    Invalid(W),
}

/// The AST and evaluated adapters supply scalar text, holes, and refusal witnesses.
/// Requiredness is queried only for an absent or entirely empty list.
pub(crate) fn parse<I, W: Copy>(
    list: ArgumentList<I, W>,
    required: impl FnOnce() -> bool,
) -> Result<Option<String>, W>
where
    I: IntoIterator<Item = Result<Option<(String, W)>, W>>,
{
    let (entries, value) = match list {
        ArgumentList::List(entries, value) => (entries, value),
        ArgumentList::Absent(value) => return if required() { Err(value) } else { Ok(None) },
        ArgumentList::Invalid(value) => return Err(value),
    };
    let mut params = String::new();
    let mut empty_argument = None;
    for entry in entries {
        let Some((text, witness)) = entry? else {
            continue;
        };
        if text.is_empty() {
            empty_argument = empty_argument.or(Some(witness));
            continue;
        }
        if !params.is_empty() {
            params.push(',');
        }
        params.push_str(&text);
    }
    if params.is_empty() {
        return if required() {
            Err(empty_argument.unwrap_or(value))
        } else {
            Ok(None)
        };
    }
    Ok(Some(params))
}

pub(crate) fn parse_expression<'r, 'a>(
    value: &'r Expression<'a>,
    absent: bool,
    required: impl FnOnce() -> bool,
) -> Result<Option<String>, &'r Expression<'a>> {
    let list = match unwrap_syntax_only(value) {
        Expression::ArrayExpression(array) => ArgumentList::List(
            array.elements.iter().map(|element| {
                let expression = match element {
                    ArrayExpressionElement::SpreadElement(spread) => return Err(&spread.argument),
                    ArrayExpressionElement::Elision(_) => return Ok(None),
                    _ => element.to_expression(),
                };
                get_string_by_literal_expression(expression)
                    .map(|text| Some((text.into_owned(), expression)))
                    .ok_or(expression)
            }),
            value,
        ),
        _ if absent => ArgumentList::Absent(value),
        _ => ArgumentList::Invalid(value),
    };
    parse(list, required)
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
    let params = parse_expression(value, inputs.absent(value), || requires_argument(selector))
        .map_err(reject)?;
    Ok(Some(params.map_or_else(
        || selector.clone(),
        |params| add_selector_params(selector.clone(), &params),
    )))
}
