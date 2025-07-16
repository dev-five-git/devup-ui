use crate::{
    ExtractStyleProp,
    extract_style::{extract_import::ExtractImport, extract_style_value::ExtractStyleValue},
    extractor::{
        GlobalExtractResult, extract_style_from_expression::extract_style_from_expression,
    },
};
use css::style_selector::StyleSelector;
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
            if let ObjectPropertyKind::ObjectProperty(o) = p {
                let name = if let PropertyKey::StaticIdentifier(ident) = &o.key {
                    ident.name.to_string()
                } else if let PropertyKey::StringLiteral(s) = &o.key {
                    s.value.to_string()
                } else if let PropertyKey::TemplateLiteral(t) = &o.key {
                    t.quasis
                        .iter()
                        .map(|q| q.value.raw.as_str())
                        .collect::<Vec<_>>()
                        .join("")
                } else {
                    continue;
                };

                if name == "imports" {
                    if let Expression::ArrayExpression(arr) = &o.value {
                        for p in arr.elements.iter() {
                            styles.push(ExtractStyleProp::Static(ExtractStyleValue::Import(
                                ExtractImport {
                                    url: if let ArrayExpressionElement::StringLiteral(s) = p {
                                        s.value.trim().to_string()
                                    } else if let ArrayExpressionElement::TemplateLiteral(t) = p {
                                        t.quasis
                                            .iter()
                                            .map(|q| q.value.raw.as_str())
                                            .collect::<Vec<_>>()
                                            .join("")
                                            .trim()
                                            .to_string()
                                    } else {
                                        continue;
                                    },
                                    file: file.to_string(),
                                },
                            )));
                        }
                    }
                    continue;
                }
                styles.extend(
                    extract_style_from_expression(
                        ast_builder,
                        None,
                        &mut o.value,
                        0,
                        Some(&StyleSelector::Global(name.clone(), file.to_string())),
                    )
                    .styles,
                );
            }
        }
    }
    GlobalExtractResult {
        styles,
        style_order: None,
    }
}
