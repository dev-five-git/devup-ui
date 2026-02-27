use crate::ExtractStyleProp;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::extract_style::extract_style_value::ExtractStyleValue;
use crate::stylex::{
    SelectorPart, decompose_value_conditions, format_number, is_unitless_property,
    normalize_stylex_property,
};
use crate::utils::{get_number_by_literal_expression, get_string_by_literal_expression};
use css::optimize_value::optimize_value;
use oxc_ast::AstBuilder;
use oxc_ast::ast::{Expression, ObjectPropertyKind};

use crate::utils::get_string_by_property_key;

/// Extract styles from a `stylex.create()` call's argument (ObjectExpression).
///
/// Handles static string/number values (Phase 1) and value-level conditions (Phase 2).
///
/// Returns a Vec of `(namespace_name, style_props)` pairs. Each namespace
/// corresponds to a top-level key in the `stylex.create({...})` argument.
pub fn extract_stylex_namespace_styles<'a>(
    _ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
) -> Vec<(String, Vec<ExtractStyleProp<'a>>)> {
    let Expression::ObjectExpression(obj) = expression else {
        return vec![];
    };

    let mut result = vec![];

    for prop in obj.properties.iter() {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            continue;
        };

        let Some(ns_name) = get_string_by_property_key(&prop.key) else {
            continue;
        };

        let Expression::ObjectExpression(ns_obj) = &prop.value else {
            // Non-object namespace value (e.g., null): push empty styles
            result.push((ns_name, vec![]));
            continue;
        };

        let mut styles = vec![];

        for style_prop in ns_obj.properties.iter() {
            let ObjectPropertyKind::ObjectProperty(style_prop) = style_prop else {
                continue;
            };

            let Some(prop_name) = get_string_by_property_key(&style_prop.key) else {
                continue;
            };

            // Phase 2: pseudo-element / pseudo-class top-level keys
            if prop_name.starts_with("::") || prop_name.starts_with(':') {
                let Expression::ObjectExpression(inner_obj) = &style_prop.value else {
                    continue;
                };
                for inner_prop in inner_obj.properties.iter() {
                    let ObjectPropertyKind::ObjectProperty(inner_prop) = inner_prop else {
                        continue;
                    };
                    let Some(inner_name) = get_string_by_property_key(&inner_prop.key) else {
                        continue;
                    };
                    let inner_css_property = normalize_stylex_property(&inner_name);
                    let parent_selectors = vec![SelectorPart::Pseudo(prop_name.clone())];
                    for decomposed in decompose_value_conditions(
                        &inner_css_property,
                        &inner_prop.value,
                        &parent_selectors,
                    ) {
                        if let Some(css_value) = decomposed.value {
                            styles.push(ExtractStyleProp::Static(ExtractStyleValue::Static(
                                ExtractStaticStyle {
                                    property: decomposed.property,
                                    value: optimize_value(&css_value),
                                    level: 0,
                                    selector: decomposed.selector,
                                    style_order: None,
                                    layer: None,
                                },
                            )));
                        }
                    }
                }
                continue;
            }

            let css_property = normalize_stylex_property(&prop_name);

            // Phase 1: static string/number values
            let css_value = if let Some(s) = get_string_by_literal_expression(&style_prop.value) {
                s
            } else if let Some(n) = get_number_by_literal_expression(&style_prop.value) {
                if is_unitless_property(&css_property) || n == 0.0 {
                    format_number(n)
                } else {
                    format!("{}px", format_number(n))
                }
            } else if matches!(&style_prop.value, Expression::ObjectExpression(_)) {
                // Phase 2: value-level conditions
                for decomposed in decompose_value_conditions(&css_property, &style_prop.value, &[])
                {
                    if let Some(css_value) = decomposed.value {
                        styles.push(ExtractStyleProp::Static(ExtractStyleValue::Static(
                            ExtractStaticStyle {
                                property: decomposed.property,
                                value: optimize_value(&css_value),
                                level: 0,
                                selector: decomposed.selector,
                                style_order: None,
                                layer: None,
                            },
                        )));
                    }
                }
                continue;
            } else {
                // Skip NullLiteral, dynamic values, etc.
                continue;
            };

            // Construct directly — bypass convert_value() to avoid devup-ui
            // spacing transformations. StyleX values are raw CSS, only optimize_value().
            styles.push(ExtractStyleProp::Static(ExtractStyleValue::Static(
                ExtractStaticStyle {
                    property: css_property,
                    value: optimize_value(&css_value),
                    level: 0,
                    selector: None,
                    style_order: None,
                    layer: None,
                },
            )));
        }

        result.push((ns_name, styles));
    }

    result
}
