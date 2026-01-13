use std::collections::BTreeMap;

use crate::{
    ExtractStyleProp,
    css_utils::{CssToStyleResult, css_to_style_literal},
    extract_style::{
        extract_font_face::ExtractFontFace, extract_import::ExtractImport,
        extract_style_value::ExtractStyleValue,
    },
    extractor::{
        GlobalExtractResult, extract_style_from_expression::extract_style_from_expression,
    },
    utils::{get_string_by_literal_expression, get_string_by_property_key},
};
use css::{
    disassemble_property,
    optimize_multi_css_value::{check_multi_css_optimize, optimize_mutli_css_value, wrap_url},
    style_selector::StyleSelector,
};
use oxc_ast::{
    AstBuilder,
    ast::{ArrayExpressionElement, Expression, ObjectPropertyKind},
};

pub fn extract_global_style_from_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    file: &str,
) -> GlobalExtractResult<'a> {
    let mut styles = vec![];

    if let Expression::ObjectExpression(obj) = expression {
        for p in obj.properties.iter_mut() {
            match p {
                ObjectPropertyKind::ObjectProperty(o) => {
                    if let Some(name) = get_string_by_property_key(&o.key) {
                        if name == "imports" {
                            if let Expression::ArrayExpression(arr) = &o.value {
                                for p in arr.elements.iter() {
                                    if let Expression::ObjectExpression(obj) = p.to_expression() {
                                        let mut url = None;
                                        let mut query = None;
                                        for p in obj.properties.iter() {
                                            if let ObjectPropertyKind::ObjectProperty(o) = p
                                                && let Some(ident) = o.key.as_expression()
                                                && let Some(ident) =
                                                    get_string_by_literal_expression(ident)
                                            {
                                                if ident == "url" {
                                                    url =
                                                        get_string_by_literal_expression(&o.value);
                                                } else if ident == "query" {
                                                    query =
                                                        get_string_by_literal_expression(&o.value);
                                                }
                                            }
                                        }
                                        if let Some(url) = url {
                                            styles.push(ExtractStyleProp::Static(
                                                ExtractStyleValue::Import(ExtractImport {
                                                    url: format!(
                                                        "\"{url}\"{}",
                                                        if let Some(query) = query {
                                                            format!(" {query}")
                                                        } else {
                                                            "".to_string()
                                                        }
                                                    ),
                                                    file: file.to_string(),
                                                }),
                                            ));
                                        }
                                    } else if !matches!(
                                        p.to_expression(),
                                        Expression::NumericLiteral(_)
                                    ) && let Some(url) =
                                        get_string_by_literal_expression(p.to_expression())
                                    {
                                        styles.push(ExtractStyleProp::Static(
                                            ExtractStyleValue::Import(ExtractImport {
                                                url,
                                                file: file.to_string(),
                                            }),
                                        ));
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
                                                            && let Some(property_name) = get_string_by_property_key(&o.key)
                                                            && let Some(s) = get_string_by_literal_expression(&o.value)
                                                        {
                                                            Some(
                                                                disassemble_property(&property_name)
                                                                    .iter()
                                                                    .map(|p| {
                                                                        let v = if check_multi_css_optimize(p) { optimize_mutli_css_value(&s) } else { s.clone() };
                                                                        if *p == "src" { (p.to_string(), wrap_url(&v)) } else { (p.to_string(), v) }
                                                                    })
                                                                    .collect::<Vec<_>>(),
                                                            )
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                    .flatten(),
                                            ),
                                            file: file.to_string(),
                                        })));
                                    } else if let ArrayExpressionElement::TemplateLiteral(t) = p {
                                        let css_styles = css_to_style_literal(t, 0, &None)
                                            .into_iter()
                                            .filter_map(|ex| {
                                                if let CssToStyleResult::Static(st) = ex {
                                                    Some(ExtractStyleValue::Static(st))
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect::<Vec<_>>();
                                        styles.push(ExtractStyleProp::Static(
                                            ExtractStyleValue::FontFace(ExtractFontFace {
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
                                            }),
                                        ));
                                    }
                                }
                            }
                        } else {
                            // Handle @layer property in globalStyle
                            // Extract the layer name if present in the style object
                            let mut layer_name: Option<String> = None;
                            if let Expression::ObjectExpression(style_obj) = &o.value {
                                for style_prop in style_obj.properties.iter() {
                                    if let ObjectPropertyKind::ObjectProperty(sp) = style_prop
                                        && let Some(prop_name) = get_string_by_property_key(&sp.key)
                                        && prop_name == "@layer"
                                    {
                                        layer_name = get_string_by_literal_expression(&sp.value);
                                        break;
                                    }
                                }
                            }

                            let extracted = extract_style_from_expression(
                                ast_builder,
                                None,
                                &mut o.value,
                                0,
                                &Some(StyleSelector::Global(
                                    if let Some(name) = name.strip_prefix("_") {
                                        StyleSelector::from(name).to_string().replace("&", "*")
                                    } else {
                                        name.to_string()
                                    },
                                    file.to_string(),
                                )),
                            );

                            // Filter out @layer property from styles and set layer on remaining styles
                            for mut style in extracted.styles {
                                if let ExtractStyleProp::Static(ExtractStyleValue::Static(
                                    ref mut st,
                                )) = style
                                {
                                    // Skip @layer property - it's not a CSS property
                                    if st.property() == "@layer" {
                                        continue;
                                    }
                                    // Set layer on other styles
                                    if let Some(ref layer) = layer_name {
                                        st.layer = Some(layer.clone());
                                    }
                                }
                                styles.push(style);
                            }
                        }
                    }
                }
                ObjectPropertyKind::SpreadProperty(o) => {
                    styles.extend(
                        extract_global_style_from_expression(
                            ast_builder,
                            o.argument.get_inner_expression_mut(),
                            file,
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
