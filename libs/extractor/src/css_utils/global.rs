use super::{
    cursor::{self, Item},
    literal::CssText,
};
use crate::{
    ExtractStyleProp,
    extract_style::{extract_css::ExtractCss, extract_style_value::ExtractStyleValue},
};
use oxc_ast::ast::Expression;
use oxc_ast::builder::AstBuilder;
use std::ops::Range;

pub(crate) fn reject(text: &CssText<'_>, range: Range<usize>, api: &str) -> Vec<(u32, String)> {
    let mut errors = Vec::new();
    for item in cursor::items(&text.text[range.clone()]) {
        match item {
            Item::Declaration { key, .. }
                if text
                    .key(key.start + range.start..key.end + range.start)
                    .is_ok_and(|name| crate::style_order::reserved(&name)) =>
            {
                errors.push(crate::style_order::no_effect(
                    api,
                    text.offset(range.start + key.start),
                ));
            }
            Item::Block { body, .. } => errors.extend(reject(
                text,
                body.start + range.start..body.end + range.start,
                api,
            )),
            Item::Declaration { .. } | Item::Statement(_) => {}
        }
    }
    errors
}

pub(crate) fn forbidden(text: &CssText<'_>, range: Range<usize>, errors: &mut Vec<(u32, String)>) {
    for item in cursor::items(&text.text[range.clone()]) {
        if let Item::Block { prelude, body } = item {
            let prelude = text
                .key(range.start + prelude.start..range.start + prelude.end)
                .unwrap_or_default();
            let body = range.start + body.start..range.start + body.end;
            if prelude.trim().starts_with("@font-face") {
                errors.extend(reject(text, body, "fontFaces"));
            } else if prelude.trim().starts_with("@keyframes")
                || prelude.trim().starts_with("@-webkit-keyframes")
            {
                errors.extend(reject(text, body, "keyframes"));
            } else {
                forbidden(text, body, errors);
            }
        }
    }
}

pub(crate) fn extract<'a>(
    ast: &AstBuilder<'a>,
    expression: &Expression<'a>,
    file: &str,
) -> Option<Vec<ExtractStyleProp<'a>>> {
    extract_text(ast, &CssText::new(ast, expression)?, file)
}

pub(crate) fn extract_text<'a>(
    ast: &AstBuilder<'a>,
    text: &CssText<'a>,
    file: &str,
) -> Option<Vec<ExtractStyleProp<'a>>> {
    if !text.has_order(0..text.text.len()) {
        return None;
    }
    let mut errors = Vec::new();
    forbidden(text, 0..text.text.len(), &mut errors);
    let mut object = text.scoped_object(ast, 0..text.text.len(), true);
    let mut raw = Vec::new();
    remove_opaque(&mut object, &mut raw, &[]);
    statements(&text.text, &mut raw, &[]);
    let mut props =
        crate::extractor::extract_global_style_from_expression::extract_literal_global_styles(
            ast,
            &mut object,
            file,
        );
    props.extend(
        errors
            .into_iter()
            .map(|(offset, message)| ExtractStyleProp::Diagnostic {
                offset,
                message,
                disposition: crate::ErrorDisposition::Definitive,
            }),
    );
    props.extend(raw.into_iter().map(|css| {
        ExtractStyleProp::Static(ExtractStyleValue::Css(ExtractCss {
            css,
            file: file.to_string(),
        }))
    }));
    Some(props)
}

fn remove_opaque(expression: &mut Expression<'_>, raw: &mut Vec<String>, wrappers: &[String]) {
    let Expression::ObjectExpression(object) = expression else {
        return;
    };
    object.properties.retain_mut(|property| {
        let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property else {
            return true;
        };
        let Some(key) = property.key.static_name() else {
            return true;
        };
        if key.starts_with('@')
            && !matches!(
                key.split_whitespace().next(),
                Some("@media" | "@supports" | "@container" | "@layer")
            )
        {
            let mut css = format!("{}{{{}}}", key, serialize(&property.value));
            for wrapper in wrappers.iter().rev() {
                css = format!("{wrapper}{{{css}}}");
            }
            raw.push(css);
            return false;
        }
        if key == "selectors" {
            remove_opaque(&mut property.value, raw, wrappers);
        } else if key.starts_with('@') {
            let mut nested = wrappers.to_vec();
            nested.push(key.to_string());
            remove_opaque(&mut property.value, raw, &nested);
        }
        true
    });
}

fn statements(text: &str, raw: &mut Vec<String>, wrappers: &[String]) {
    for item in cursor::items(text) {
        match item {
            Item::Statement(range) => {
                let mut css = format!("{};", cursor::clean(&text[range]).trim());
                for wrapper in wrappers.iter().rev() {
                    css = format!("{wrapper}{{{css}}}");
                }
                raw.push(css);
            }
            Item::Block { prelude, body }
                if matches!(
                    text[prelude.clone()].split_whitespace().next(),
                    Some("@media" | "@supports" | "@container" | "@layer")
                ) =>
            {
                let mut nested = wrappers.to_vec();
                nested.push(text[prelude].trim().to_string());
                statements(&text[body], raw, &nested);
            }
            Item::Declaration { .. } | Item::Block { .. } => {}
        }
    }
}

fn serialize(expression: &Expression<'_>) -> String {
    serialize_scope(expression, false)
}

fn serialize_scope(expression: &Expression<'_>, record: bool) -> String {
    let Expression::ObjectExpression(object) = expression else {
        return crate::utils::get_string_by_literal_expression(expression)
            .map_or_else(String::new, std::borrow::Cow::into_owned);
    };
    object
        .properties
        .iter()
        .filter_map(|property| {
            let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property else {
                return None;
            };
            let key = property.key.static_name()?;
            if !record && crate::style_order::reserved(&key) {
                return None;
            }
            if !record && key == "selectors" {
                return Some(serialize_scope(&property.value, true));
            }
            let value = serialize_scope(&property.value, false);
            Some(
                if matches!(&property.value, Expression::ObjectExpression(_)) {
                    format!("{key}{{{value}}}")
                } else {
                    format!("{key}:{value};")
                },
            )
        })
        .collect()
}
