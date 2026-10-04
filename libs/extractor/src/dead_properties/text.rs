//! Declaration-aware CSS scanning with offsets into the authored text.

use super::declaration_error;
use crate::{ExtractStyleProp, utils::get_string_by_literal_expression};
use oxc_ast::ast::{Expression, StringLiteral, TemplateLiteral};
use oxc_span::GetSpan;

/// Find declaration names, ignoring selector preludes and balanced value tokens.
fn declarations(text: &str) -> Vec<(usize, std::borrow::Cow<'_, str>)> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut start = 0;
    let mut index = 0;
    let mut colon = None;
    let mut quote = None;
    let mut parentheses = 0_u32;
    let mut brackets = 0_u32;
    let mut value_braces = 0_u32;
    while index < bytes.len() {
        let byte = bytes[index];
        if let Some(delimiter) = quote {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if byte == delimiter {
                quote = None;
            }
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index < bytes.len()
                && !(bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/'))
            {
                index += 1;
            }
            index += 2;
            if colon.is_none()
                && text[start..index.min(bytes.len())]
                    .trim_start()
                    .starts_with("/*")
            {
                start = index.min(bytes.len());
            }
            continue;
        } else {
            match byte {
                b'\\' => {
                    index += 2;
                    continue;
                }
                b'\'' | b'"' => quote = Some(byte),
                b'(' => parentheses += 1,
                b')' => parentheses = parentheses.saturating_sub(1),
                b'[' => brackets += 1,
                b']' => brackets = brackets.saturating_sub(1),
                b':' if parentheses == 0 && brackets == 0 && colon.is_none() => colon = Some(index),
                b'{' if parentheses == 0 && brackets == 0 => {
                    if colon.is_some() && text[start..].trim_start().starts_with("--") {
                        value_braces += 1;
                    } else {
                        start = index + 1;
                        colon = None;
                    }
                }
                b'}' if value_braces > 0 => value_braces -= 1,
                b';' | b'}' if parentheses == 0 && brackets == 0 && value_braces == 0 => {
                    record(text, start, colon, &mut found);
                    start = index + 1;
                    colon = None;
                }
                _ => {}
            }
        }
        index += 1;
    }
    record(text, start, colon, &mut found);
    found
}

fn record<'a>(
    text: &'a str,
    start: usize,
    colon: Option<usize>,
    found: &mut Vec<(usize, std::borrow::Cow<'a, str>)>,
) {
    if let Some(colon) = colon {
        let head = &text[start..colon];
        let name = if head.contains("/*") {
            std::borrow::Cow::Owned(css::rm_css_comment::rm_css_comment(head).trim().to_string())
        } else {
            std::borrow::Cow::Borrowed(head.trim())
        };
        if name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            found.push((start + head.len() - head.trim_start().len(), name));
        }
    }
}

/// Scan decoded string text while retaining authored offsets across JS escapes.
pub(crate) fn literal_errors<'a>(literal: &StringLiteral<'_>) -> Vec<ExtractStyleProp<'a>> {
    let origins = super::strings::origins(literal);
    declarations(&literal.value)
        .into_iter()
        .filter_map(|(index, name)| declaration_error(&name, *origins.get(index)?))
        .collect()
}

/// Join raw quasis with interpolation origins rather than inventing contiguous offsets.
pub(crate) fn template_errors<'a>(template: &TemplateLiteral<'_>) -> Vec<ExtractStyleProp<'a>> {
    let mut text = String::new();
    let mut origins = Vec::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        text.push_str(&quasi.value.raw);
        origins.extend((0..quasi.value.raw.len()).filter_map(|offset| {
            u32::try_from(offset)
                .ok()
                .map(|offset| quasi.span.start + offset)
        }));
        if let Some(expression) = template.expressions.get(index) {
            if let Some(value) = get_string_by_literal_expression(expression) {
                text.push_str(&value);
                match expression {
                    Expression::StringLiteral(literal) => {
                        origins.extend(super::strings::origins(literal));
                    }
                    _ => origins.extend(std::iter::repeat_n(expression.span().start, value.len())),
                }
            } else {
                text.push_str("__value__");
                origins.extend(std::iter::repeat_n(expression.span().start, 9));
            }
        }
    }
    declarations(&text)
        .into_iter()
        .filter_map(|(index, name)| declaration_error(&name, *origins.get(index)?))
        .collect()
}

/// Validate CSS text only where an expression declares styles, not inside property values.
pub(crate) fn expression_errors<'a>(expression: &Expression<'_>) -> Vec<ExtractStyleProp<'a>> {
    match expression {
        Expression::StringLiteral(literal) => literal_errors(literal),
        Expression::TemplateLiteral(template) => template_errors(template),
        _ => Vec::new(),
    }
}
