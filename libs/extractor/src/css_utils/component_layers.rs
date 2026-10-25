use super::CssToStyleResult;
use crate::extract_style::{
    extract_dynamic_style::ExtractDynamicStyle, extract_static_style::ExtractStaticStyle,
};
use crate::utils::{build_time_error, get_string_by_literal_expression, readable_code};
use oxc_ast::ast::{Expression, TemplateLiteral};

pub(crate) const LAYER_ORDER_REQUIREMENT: &str = "layer order statements belong in `globalCss`, not component styles; declare the order with `globalCss` and use named `@layer` blocks here";
pub(super) const LAYER_NAME_REQUIREMENT: &str = "name the layer, for example `@layer components { ... }`: component layer names must be dot-separated CSS identifiers, not CSS-wide reserved keywords; declare layer order with `globalCss`";

pub(crate) fn template_layer_errors(
    template: &TemplateLiteral<'_>,
    api: &str,
) -> Vec<(u32, String)> {
    let mut text = String::new();
    let mut source = Vec::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        source.push((text.len(), quasi.span.start));
        text.push_str(&quasi.value.raw);
        if let Some(expression) = template.expressions.get(index) {
            text.push_str(&get_string_by_literal_expression(expression).map_or_else(
                || format!("${{{}}}", readable_code(expression)),
                std::borrow::Cow::into_owned,
            ));
        }
    }
    statement_errors(&text, api, |offset| {
        source
            .iter()
            .rev()
            .find(|(start, _)| *start <= offset)
            .and_then(|(start, original)| {
                u32::try_from(offset - start)
                    .ok()
                    .and_then(|delta| original.checked_add(delta))
            })
            .unwrap_or(template.span.start)
    })
}

pub(crate) fn expression_layer_errors(
    expression: &Expression<'_>,
    api: &str,
) -> Vec<(u32, String)> {
    match crate::utils::unwrap_syntax_only(expression) {
        Expression::TemplateLiteral(template) => template_layer_errors(template, api),
        Expression::StringLiteral(literal) => {
            statement_errors(&literal.value, api, |_| literal.span.start)
        }
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .filter_map(|element| element.as_expression())
            .flat_map(|part| expression_layer_errors(part, api))
            .collect(),
        Expression::ConditionalExpression(choice) => [&choice.consequent, &choice.alternate]
            .into_iter()
            .flat_map(|part| expression_layer_errors(part, api))
            .collect(),
        Expression::LogicalExpression(logical) => expression_layer_errors(&logical.right, api),
        Expression::ObjectExpression(_) => super::object_layer_errors(expression, api),
        _ => vec![],
    }
}

fn statement_errors(text: &str, api: &str, offset: impl Fn(usize) -> u32) -> Vec<(u32, String)> {
    let mut errors = Vec::new();
    let mut from = 0;
    for (index, character) in super::layer_blocks::boundaries(text) {
        let head = &text[from..index];
        let trimmed = trim_trivia(head);
        if let Some(name) = layer_prelude(trimmed) {
            let requirement = match character {
                ';' => Some(LAYER_ORDER_REQUIREMENT),
                '{' if super::parse_layer_name(name).is_none() => Some(LAYER_NAME_REQUIREMENT),
                _ => None,
            };
            if let Some(requirement) = requirement {
                errors.push((
                    offset(from + head.len() - trimmed.len()),
                    build_time_error(api, &format!("{trimmed}{character}"), requirement),
                ));
            }
        }
        from = index + character.len_utf8();
    }
    errors
}

pub(super) fn trim_trivia(mut text: &str) -> &str {
    loop {
        text = text.trim_start();
        match text
            .strip_prefix("/*")
            .and_then(|comment| comment.find("*/").map(|end| &comment[end + 2..]))
        {
            Some(rest) => text = rest,
            None => return text,
        }
    }
}

pub(super) fn named_layer(prelude: &str) -> Option<String> {
    super::parse_layer_name(layer_prelude(prelude)?)
}

fn layer_prelude(prelude: &str) -> Option<&str> {
    if !prelude
        .get(..6)
        .is_some_and(|keyword| keyword.eq_ignore_ascii_case("@layer"))
    {
        return None;
    }
    prelude.get(6..).filter(|rest| {
        rest.is_empty()
            || rest.starts_with([' ', '\t', '\n', '\r', '\u{c}'])
            || rest.starts_with("/*")
    })
}

pub(crate) fn nest_layer(layer: &str, inner: &mut Option<String>) {
    *inner = Some(match inner.take() {
        Some(inner) => format!("{layer}.{inner}"),
        None => layer.to_string(),
    });
}

pub(super) fn dynamic_from(style: &ExtractStaticStyle, identifier: &str) -> CssToStyleResult {
    let mut dynamic = ExtractDynamicStyle::new(
        style.property(),
        style.level(),
        identifier,
        style.selector().cloned(),
    );
    dynamic.layer.clone_from(&style.layer);
    CssToStyleResult::Dynamic(dynamic)
}

#[cfg(test)]
#[path = "component_layer_coverage_tests.rs"]
mod coverage_tests;
