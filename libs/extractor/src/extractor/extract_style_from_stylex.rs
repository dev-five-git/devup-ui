use crate::ExtractStyleProp;
use crate::extract_style::extract_dynamic_style::ExtractDynamicStyle;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::extract_style::extract_style_value::ExtractStyleValue;
use crate::stylex::{
    SelectorPart, StylexIncludeRef, decompose_value_conditions, is_first_that_works_call,
    is_include_call_static, is_types_call, normalize_stylex_property,
};
use crate::utils::get_string_by_literal_expression;
use css::optimize_value::optimize_value;
use css::sheet_to_variable_name;
use oxc_ast::AstBuilder;
use oxc_ast::ast::{BindingPattern, Expression, ObjectPropertyKind, Statement};
use rustc_hash::FxHashMap;

use crate::utils::get_string_by_property_key;

/// Extract styles from a `stylex.create()` call's argument (ObjectExpression).
///
/// Handles static string/number values (Phase 1) and value-level conditions (Phase 2).
///
/// Returns a Vec of `(namespace_name, style_props, css_vars, include_refs)` tuples. Each namespace
/// corresponds to a top-level key in the `stylex.create({...})` argument.
#[allow(clippy::type_complexity)]
pub fn extract_stylex_namespace_styles<'a>(
    _ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    keyframe_names: &FxHashMap<String, String>,
) -> Vec<(
    String,
    Vec<ExtractStyleProp<'a>>,
    Option<Vec<(usize, String)>>,
    Vec<StylexIncludeRef>,
)> {
    let Expression::ObjectExpression(obj) = expression else {
        return vec![];
    };

    let mut result = vec![];

    for prop in obj.properties.iter() {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            // Phase 4c: Spread not supported at namespace level
            if matches!(prop, ObjectPropertyKind::SpreadProperty(_)) {
                eprintln!(
                    "[stylex] ERROR: Object spread is not allowed at the namespace level of stylex.create()."
                );
            }
            continue;
        };

        let Some(ns_name) = get_string_by_property_key(&prop.key) else {
            // Phase 4c: Computed namespace keys not supported
            if prop.computed {
                eprintln!(
                    "[stylex] ERROR: Computed namespace keys are not allowed in stylex.create()."
                );
            }
            continue;
        };

        // Phase 4b: Arrow function (dynamic namespace)
        if let Expression::ArrowFunctionExpression(arrow) = &prop.value {
            if let Some((styles, css_vars)) =
                extract_stylex_dynamic_namespace(arrow, keyframe_names)
            {
                result.push((ns_name, styles, Some(css_vars), vec![]));
            } else {
                result.push((ns_name, vec![], None, vec![]));
            }
            continue;
        }

        let Expression::ObjectExpression(ns_obj) = &prop.value else {
            // Non-object namespace value (e.g., null): push empty styles
            result.push((ns_name, vec![], None, vec![]));
            continue;
        };

        let mut styles = vec![];
        let mut include_refs = vec![];

        for style_prop in ns_obj.properties.iter() {
            let ObjectPropertyKind::ObjectProperty(style_prop) = style_prop else {
                // Check for stylex.include() spread
                if let ObjectPropertyKind::SpreadProperty(spread) = style_prop
                    && let Expression::CallExpression(call) = &spread.argument
                    && is_include_call_static(&call.callee)
                    && !call.arguments.is_empty()
                {
                    // Parse include(base.member)
                    if let Expression::StaticMemberExpression(member) =
                        call.arguments[0].to_expression()
                        && let Expression::Identifier(ident) = &member.object
                    {
                        include_refs.push(StylexIncludeRef {
                            var_name: ident.name.to_string(),
                            member_name: member.property.name.to_string(),
                        });
                    }
                } else if matches!(style_prop, ObjectPropertyKind::SpreadProperty(_)) {
                    eprintln!(
                        "[stylex] ERROR: Object spread is not allowed in stylex.create() namespaces. Define all properties explicitly."
                    );
                }
                continue;
            };

            let Some(prop_name) = get_string_by_property_key(&style_prop.key) else {
                // Phase 4c: Computed property keys not supported
                if style_prop.computed {
                    eprintln!(
                        "[stylex] ERROR: Computed property keys are not allowed in stylex.create(). Use static string keys instead."
                    );
                }
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

            // Phase 4c: Warn about CSS shorthand properties
            const SHORTHAND_PROPERTIES: &[&str] = &[
                "margin",
                "padding",
                "background",
                "border",
                "font",
                "outline",
                "overflow",
                "flex",
                "grid",
                "gap",
                "border-radius",
                "border-color",
                "border-style",
                "border-width",
                "margin-inline",
                "margin-block",
                "padding-inline",
                "padding-block",
            ];
            if SHORTHAND_PROPERTIES.contains(&css_property.as_str()) {
                eprintln!(
                    "[stylex] WARNING: Shorthand property '{}' may cause unexpected specificity issues. Consider using longhand properties (e.g., 'marginTop', 'paddingLeft').",
                    css_property
                );
            }

            // Phase 4a: Resolve keyframe variable references (e.g., animationName: fadeIn)
            if let Expression::Identifier(ident) = &style_prop.value
                && let Some(anim_name) = keyframe_names.get(ident.name.as_str())
            {
                styles.push(ExtractStyleProp::Static(ExtractStyleValue::Static(
                    ExtractStaticStyle {
                        property: css_property,
                        value: optimize_value(anim_name),
                        level: 0,
                        selector: None,
                        style_order: None,
                        layer: None,
                    },
                )));
                continue;
            }

            // Phase 1: static string/number values
            let css_value = if let Some(s) = get_string_by_literal_expression(&style_prop.value) {
                s
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
            } else if let Expression::CallExpression(call) = &style_prop.value
                && is_first_that_works_call(&call.callee)
            {
                // firstThatWorks('a', 'b', 'c'): last arg is least preferred, first is most preferred.
                // CSS fallback: output in reverse order (least preferred first, most preferred last).
                for arg in call.arguments.iter().rev() {
                    let arg_expr = arg.to_expression();
                    if let Some(s) = get_string_by_literal_expression(arg_expr) {
                        styles.push(ExtractStyleProp::Static(ExtractStyleValue::Static(
                            ExtractStaticStyle {
                                property: css_property.clone(),
                                value: optimize_value(&s),
                                level: 0,
                                selector: None,
                                style_order: None,
                                layer: None,
                            },
                        )));
                    }
                }
                continue;
            } else if let Expression::CallExpression(call) = &style_prop.value
                && is_types_call(&call.callee)
                && !call.arguments.is_empty()
            {
                // stylex.types.length('100px') → extract inner value '100px'
                let inner = call.arguments[0].to_expression();
                let css_value = if let Some(s) = get_string_by_literal_expression(inner) {
                    s
                } else {
                    continue; // Can't resolve inner value
                };
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
                continue;
            } else {
                // Phase 4c: Non-static values in create() are not supported
                if !matches!(&style_prop.value, Expression::NullLiteral(_)) {
                    eprintln!(
                        "[stylex] ERROR: Non-static value for property '{}' in stylex.create(). Only string literals, numbers, null, objects (conditions), firstThatWorks(), types.*(), and arrow functions are allowed.",
                        prop_name
                    );
                }
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

        result.push((ns_name, styles, None, include_refs));
    }

    result
}

/// Extract styles from a dynamic StyleX namespace (arrow function).
/// Returns (styles_for_css, css_vars) where css_vars maps param_index to CSS variable name.
#[allow(clippy::type_complexity)]
fn extract_stylex_dynamic_namespace<'a>(
    arrow: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    keyframe_names: &FxHashMap<String, String>,
) -> Option<(Vec<ExtractStyleProp<'a>>, Vec<(usize, String)>)> {
    // 1. Extract parameter names
    let param_names: Vec<String> = arrow
        .params
        .items
        .iter()
        .filter_map(|param| {
            if let BindingPattern::BindingIdentifier(ident) = &param.pattern {
                Some(ident.name.to_string())
            } else {
                None
            }
        })
        .collect();

    if param_names.is_empty() {
        return None;
    }

    // 2. Get body ObjectExpression from expression body: (x) => ({ ... })
    if !arrow.expression {
        return None;
    }
    // Expression arrow body: Oxc always wraps in ExpressionStatement.
    // Unwrap ParenthesizedExpression since Oxc preserves parens for `(x) => ({...})`.
    let body_expr = arrow.body.statements.first().and_then(|stmt| {
        if let Statement::ExpressionStatement(e) = stmt {
            Some(&e.expression)
        } else {
            None
        }
    })?;
    let inner = if let Expression::ParenthesizedExpression(paren) = body_expr {
        &paren.expression
    } else {
        body_expr
    };
    let Expression::ObjectExpression(body_obj) = inner else {
        return None;
    };

    // 3. Process each property
    let mut styles = vec![];
    let mut css_vars = vec![];

    for prop in body_obj.properties.iter() {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            continue;
        };

        let Some(prop_name) = get_string_by_property_key(&prop.key) else {
            continue;
        };
        let css_property = normalize_stylex_property(&prop_name);

        // Check if value references a parameter (dynamic)
        let is_dynamic = if prop.shorthand {
            // Shorthand: { height } is equivalent to { height: height }
            param_names.iter().position(|p| p == &prop_name)
        } else if let Expression::Identifier(ident) = &prop.value {
            param_names.iter().position(|p| p == ident.name.as_str())
        } else {
            None
        };

        if let Some(param_idx) = is_dynamic {
            // Dynamic property: generate CSS variable
            let var_name = sheet_to_variable_name(&css_property, 0, None);
            css_vars.push((param_idx, var_name));
            let param_name = &param_names[param_idx];
            styles.push(ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                ExtractDynamicStyle::new(&css_property, 0, param_name, None),
            )));
        } else {
            // Static property: resolve keyframe references or literal values
            if let Expression::Identifier(ident) = &prop.value
                && let Some(anim_name) = keyframe_names.get(ident.name.as_str())
            {
                styles.push(ExtractStyleProp::Static(ExtractStyleValue::Static(
                    ExtractStaticStyle {
                        property: css_property,
                        value: optimize_value(anim_name),
                        level: 0,
                        selector: None,
                        style_order: None,
                        layer: None,
                    },
                )));
                continue;
            }
            let css_value = if let Some(s) = get_string_by_literal_expression(&prop.value) {
                s
            } else {
                continue;
            };
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
    }

    Some((styles, css_vars))
}
