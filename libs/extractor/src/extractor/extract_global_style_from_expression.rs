use std::borrow::Cow;

use crate::{
    ExtractStyleProp,
    css_utils::{CssToStyleResult, css_to_style_literal},
    extract_style::{
        extract_font_face::ExtractFontFace, extract_import::ExtractImport,
        extract_style_value::ExtractStyleValue,
    },
    extractor::{
        GlobalExtractResult,
        extract_style_from_expression::{
            LiteralHandling, at_rule_record_kind, extract_style_from_expression, misplaced,
            place_in_layer, unreadable, unreadable_key, yield_typography,
        },
    },
    utils::{
        SELECTOR_NAME, get_str_by_property_key, get_string_by_literal_expression,
        get_string_by_property_key, unwrap_syntax_only_mut,
    },
};
use css::{
    at_rule::{media_shorthand_query, split_at_rule_key},
    disassemble_property,
    optimize_multi_css_value::{check_multi_css_optimize, optimize_multi_css_value, wrap_url},
    style_selector::{AtRule, AtRuleKind, StyleSelector, is_selector_name},
    utils::to_kebab_case,
};
use oxc_ast::{
    ast::{ArrayExpressionElement, Expression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::GetSpan;

pub fn extract_global_style_from_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    file: &str,
) -> GlobalExtractResult<'a> {
    let mut styles = vec![];
    collect_global_styles(ast_builder, expression, file, &[], &mut styles);
    GlobalExtractResult {
        styles,
        style_order: None,
    }
}

/// A top-level at-rule key wrapping a selector map: `'@media print'` or a
/// media shorthand such as `_print` / `_motionReduce`.
fn global_at_rule_key(name: &str) -> Option<AtRule> {
    let (kind, query) = split_at_rule_key(name).or_else(|| {
        let query = media_shorthand_query(name.strip_prefix('_')?)?;
        Some((AtRuleKind::Media, query))
    })?;
    Some(AtRule {
        kind,
        query: query.to_string(),
    })
}

/// Collect a `globalCss` selector map whose rules sit inside `at_rules`
/// (outermost first).
fn collect_global_styles<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    file: &str,
    at_rules: &[AtRule],
    styles: &mut Vec<ExtractStyleProp<'a>>,
) {
    let expression = unwrap_syntax_only_mut(expression);

    if let Expression::ObjectExpression(obj) = expression {
        for p in &mut obj.properties {
            match p {
                ObjectPropertyKind::ObjectProperty(o) => {
                    if let Some(name) = get_string_by_property_key(&o.key) {
                        if let Some(kind) = at_rule_record_kind(&name)
                            && let Expression::ObjectExpression(record) = &mut o.value
                        {
                            for entry in &mut record.properties {
                                if let ObjectPropertyKind::ObjectProperty(entry) = entry
                                    && let Some(query) = get_string_by_property_key(&entry.key)
                                {
                                    let mut nested = at_rules.to_vec();
                                    nested.push(AtRule { kind, query });
                                    collect_global_styles(
                                        ast_builder,
                                        &mut entry.value,
                                        file,
                                        &nested,
                                        styles,
                                    );
                                }
                            }
                        } else if let Some(layer) = name
                            .strip_prefix("@layer")
                            .filter(|rest| rest.starts_with(char::is_whitespace))
                            .map(str::trim)
                        {
                            if layer.is_empty()
                                || !layer.chars().all(|c| {
                                    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
                                })
                            {
                                styles.push(unreadable_key(&o.key, false));
                                continue;
                            }
                            // `'@layer name': { selector: rules }` puts the rules it holds in the layer
                            let mut layered = vec![];
                            collect_global_styles(
                                ast_builder,
                                &mut o.value,
                                file,
                                at_rules,
                                &mut layered,
                            );
                            place_in_layer(&mut layered, layer);
                            styles.extend(layered);
                        } else if let Some(at_rule) = global_at_rule_key(&name) {
                            let mut nested = at_rules.to_vec();
                            nested.push(at_rule);
                            collect_global_styles(ast_builder, &mut o.value, file, &nested, styles);
                        } else if name == "imports" {
                            if let Expression::ArrayExpression(arr) = &o.value {
                                for p in &arr.elements {
                                    // `...spread` elements carry no statically readable url.
                                    let Some(element) = p.as_expression() else {
                                        continue;
                                    };
                                    if let Expression::ObjectExpression(obj) = element {
                                        let mut url = None;
                                        let mut query = None;
                                        for p in &obj.properties {
                                            if let ObjectPropertyKind::ObjectProperty(o) = p
                                                && let Some(ident) = get_str_by_property_key(&o.key)
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
                                            // Build `"url"` (+ optional ` query`)
                                            // in one presized buffer instead of a
                                            // nested `format!` that allocates a
                                            // throwaway inner `String`.
                                            let mut import_url = String::with_capacity(
                                                url.len()
                                                    + 2
                                                    + query.as_ref().map_or(0, |q| q.len() + 1),
                                            );
                                            import_url.push('"');
                                            import_url.push_str(&url);
                                            import_url.push('"');
                                            if let Some(query) = query {
                                                import_url.push(' ');
                                                import_url.push_str(&query);
                                            }
                                            styles.push(ExtractStyleProp::Static(
                                                ExtractStyleValue::Import(ExtractImport {
                                                    url: import_url,
                                                    file: file.to_string(),
                                                }),
                                            ));
                                        }
                                    } else if !matches!(element, Expression::NumericLiteral(_))
                                        && let Some(url) = get_string_by_literal_expression(element)
                                    {
                                        styles.push(ExtractStyleProp::Static(
                                            ExtractStyleValue::Import(ExtractImport {
                                                url: url.into_owned(),
                                                file: file.to_string(),
                                            }),
                                        ));
                                    }
                                }
                            }
                        } else if name == "fontFaces" {
                            if let Expression::ArrayExpression(arr) = &o.value {
                                for p in &arr.elements {
                                    if let ArrayExpressionElement::ObjectExpression(o) = p {
                                        styles.push(ExtractStyleProp::Static(ExtractStyleValue::FontFace(ExtractFontFace {
                                            properties: o
                                                .properties
                                                .iter()
                                                .filter_map(|p| {
                                                        if let ObjectPropertyKind::ObjectProperty(o) = p
                                                            && let Some(property_name) = get_str_by_property_key(&o.key)
                                                            && let Some(s) = get_string_by_literal_expression(&o.value)
                                                        {
                                                            let it = disassemble_property(&property_name).map(|p| {
                                                                let v = if check_multi_css_optimize(&p) { optimize_multi_css_value(&s) } else { Cow::Borrowed(&*s) };
                                                                if p == "src" { (p.into_owned(), wrap_url(&v).into_owned()) } else { (p.into_owned(), v.into_owned()) }
                                                            });
                                                            Some(it.collect::<Vec<_>>())
                                                        } else {
                                                            None
                                                        }
                                                })
                                                .flatten()
                                                .collect(),
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
                                                properties: css_styles
                                                    .iter()
                                                    .filter_map(|p| {
                                                        if let ExtractStyleValue::Static(st) = p {
                                                            Some((
                                                                st.property().to_string(),
                                                                st.value().to_string(),
                                                            ))
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                    .collect(),
                                                file: file.to_string(),
                                            }),
                                        ));
                                    }
                                }
                            }
                        } else {
                            // Handle @layer property in globalStyle
                            // Extract the layer name if present in the style object
                            let layer_name = if let Expression::ObjectExpression(style_obj) = &o.value
                                && let Some(ObjectPropertyKind::ObjectProperty(sp)) = style_obj.properties.iter().find(|style_prop| matches!(style_prop, ObjectPropertyKind::ObjectProperty(s) if get_str_by_property_key(&s.key).as_deref() == Some("@layer")))
                            {
                                get_string_by_literal_expression(&sp.value)
                            } else {
                                None
                            };

                            let global = StyleSelector::Global(
                                if let Some(pseudo) = name.strip_prefix('_') {
                                    let pseudo = to_kebab_case(pseudo);
                                    if !is_selector_name(&pseudo) {
                                        styles.push(misplaced(
                                            o.key.span().start,
                                            name,
                                            SELECTOR_NAME,
                                        ));
                                        continue;
                                    }
                                    StyleSelector::from(pseudo.as_ref())
                                        .to_string()
                                        .replace('&', "*")
                                } else {
                                    name
                                },
                                file.to_string(),
                            );
                            // `None` when the enclosing at-rules can never match together.
                            let selector = at_rules.iter().try_fold(global, |selector, rule| {
                                StyleSelector::nest_at_rule(Some(&selector), rule.kind, &rule.query)
                            });
                            let extracted = selector
                                .map(|selector| {
                                    extract_style_from_expression(
                                        ast_builder,
                                        None,
                                        &mut o.value,
                                        0,
                                        &Some(selector),
                                        LiteralHandling::ExpandResponsiveThemeToken,
                                    )
                                })
                                .unwrap_or_default();

                            // `@layer` names the layer of every other declaration,
                            // responsive and nested ones included
                            let mut extracted = extracted.styles;
                            extracted.retain(|style| {
                                !matches!(style, ExtractStyleProp::Static(ExtractStyleValue::Static(st)) if st.property() == "@layer")
                            });
                            if let Some(layer) = layer_name {
                                place_in_layer(&mut extracted, &layer);
                            }
                            yield_typography(&mut extracted);
                            styles.extend(extracted);
                        }
                    } else {
                        styles.push(unreadable_key(&o.key, false));
                    }
                }
                ObjectPropertyKind::SpreadProperty(o) => {
                    collect_global_styles(
                        ast_builder,
                        o.argument.get_inner_expression_mut(),
                        file,
                        at_rules,
                        styles,
                    );
                }
            }
        }
    } else {
        styles.extend(unreadable(expression).styles);
    }
}
