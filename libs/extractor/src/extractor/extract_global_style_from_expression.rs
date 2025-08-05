use std::collections::BTreeMap;

use crate::{
    ExtractStyleProp,
    css_utils::css_to_style,
    extract_style::{
        extract_font_face::ExtractFontFace, extract_import::ExtractImport,
        extract_style_value::ExtractStyleValue,
    },
    extractor::{
        GlobalExtractResult, extract_style_from_expression::extract_style_from_expression,
    },
    utils::get_string_by_literal_expression,
};
use css::{
    disassemble_property,
    optimize_multi_css_value::{check_multi_css_optimize, optimize_mutli_css_value},
    style_selector::StyleSelector,
};
use oxc_ast::{
    AstBuilder,
    ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKey},
};

pub fn extract_global_style_from_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    file: &str,
) -> GlobalExtractResult<'a> {
    let mut styles = vec![];

    if let Expression::ObjectExpression(obj) = expression {
        for p in obj.properties.iter_mut() {
            if let ObjectPropertyKind::ObjectProperty(o) = p
                && let Some(name) = if let PropertyKey::StaticIdentifier(ident) = &o.key {
                    Some(ident.name.to_string())
                } else if let PropertyKey::StringLiteral(s) = &o.key {
                    Some(s.value.to_string())
                } else if let PropertyKey::TemplateLiteral(t) = &o.key {
                    Some(
                        t.quasis
                            .iter()
                            .map(|q| q.value.raw.as_str())
                            .collect::<String>(),
                    )
                } else {
                    None
                }
            {
                if name == "imports" {
                    if let Expression::ArrayExpression(arr) = &o.value {
                        for p in arr.elements.iter() {
                            if let Some(url) = if let ArrayExpressionElement::StringLiteral(s) = p {
                                Some(s.value.trim().to_string())
                            } else if let ArrayExpressionElement::TemplateLiteral(t) = p {
                                Some(
                                    t.quasis
                                        .iter()
                                        .map(|q| q.value.raw.as_str())
                                        .collect::<String>()
                                        .trim()
                                        .to_string(),
                                )
                            } else {
                                None
                            } {
                                styles.push(ExtractStyleProp::Static(ExtractStyleValue::Import(
                                    ExtractImport {
                                        url,
                                        file: file.to_string(),
                                    },
                                )));
                            }
                        }
                    }
                } else if name == "fontFaces" {
                    if let Expression::ArrayExpression(arr) = &o.value {
                        for p in arr.elements.iter() {
                            if let ArrayExpressionElement::ObjectExpression(o) = p {
                                styles.push(ExtractStyleProp::Static(ExtractStyleValue::FontFace(ExtractFontFace {
                                    properties: BTreeMap::from_iter(
                                        o.properties
                                            .iter()
                                            .filter_map(|p| {
                                                if let ObjectPropertyKind::ObjectProperty(o) = p
                                                    && let PropertyKey::StaticIdentifier(ident) = &o.key
                                                    && let Some(s) = get_string_by_literal_expression(&o.value)
                                                {
                                                    Some(disassemble_property(&ident.name).iter().map(|p| (p.to_string(), if check_multi_css_optimize(p) { optimize_mutli_css_value(&s) } else { s.clone() })).collect::<Vec<_>>())
                                                } else {
                                                    None
                                                }
                                            })
                                            .flatten(),
                                    ),
                                    file: file.to_string(),
                                })));
                            } else if let ArrayExpressionElement::TemplateLiteral(t) = p {
                                let css_styles = css_to_style(
                                    t.quasis
                                        .iter()
                                        .map(|q| q.value.raw.as_str())
                                        .collect::<String>()
                                        .trim(),
                                    0,
                                    &None,
                                )
                                .into_iter()
                                .map(ExtractStyleValue::Static)
                                .collect::<Vec<_>>();
                                styles.push(ExtractStyleProp::Static(ExtractStyleValue::FontFace(
                                    ExtractFontFace {
                                        properties: BTreeMap::from_iter(
                                            css_styles.iter().filter_map(|p| {
                                                if let ExtractStyleValue::Static(st) = p {
                                                    Some((
                                                        st.property().to_string(),
                                                        st.value().to_string(),
                                                    ))
                                                } else {
                                                    None
                                                }
                                            }),
                                        ),
                                        file: file.to_string(),
                                    },
                                )));
                            }
                        }
                    }
                } else {
                    styles.extend(
                        extract_style_from_expression(
                            ast_builder,
                            None,
                            &mut o.value,
                            0,
                            &Some(StyleSelector::Global(name.clone(), file.to_string())),
                        )
                        .styles,
                    );
                }
            }
        }
    }
    GlobalExtractResult {
        styles,
        style_order: None,
    }
}
