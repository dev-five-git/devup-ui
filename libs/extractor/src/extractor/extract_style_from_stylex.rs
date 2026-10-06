use crate::ExtractStyleProp;
use crate::extract_style::extract_dynamic_style::ExtractDynamicStyle;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::extract_style::extract_style_value::ExtractStyleValue;
use crate::stylex::{
    DecomposedStyle, SelectorPart, StylexIncludeRef, decompose_value_conditions,
    dynamic_number_suffix, is_include_call_static, normalize_stylex_property, stylex_value,
};
use css::optimize_value::optimize_value;
use css::style_selector::StyleSelector;
use oxc_ast::ast::{
    Argument, ArrowFunctionExpression, BindingPattern, Expression, ObjectExpression,
    ObjectPropertyKind, SpreadElement,
};
use oxc_span::GetSpan;
use rustc_hash::FxHashMap;

use crate::utils::{
    build_time_error, get_str_by_property_key, get_string_by_property_key, key_error,
    readable_code, runtime_value_error, spread_error,
};

/// Construct a static style directly — bypass `convert_value()` to avoid devup-ui
/// spacing transformations. `StyleX` values are raw CSS, only `optimize_value()`.
fn raw_static_style<'a>(
    property: String,
    value: &str,
    selector: Option<StyleSelector>,
) -> ExtractStyleProp<'a> {
    ExtractStyleProp::Static(ExtractStyleValue::Static(ExtractStaticStyle {
        property,
        value: optimize_value(value).into_owned(),
        level: 0,
        selector,
        style_order: None,
        layer: None,
        theme_token_resolution: Default::default(),
        naming: css::Naming::Own,
        counter_owner: crate::sparse_sites::counter_owner(),
        origin: crate::style_origin::current(),
    }))
}

/// Flatten an object literal of literal-valued properties into kebab-cased CSS
/// declarations, the shape `positionTry` and `viewTransitionClass` bodies take.
pub fn extract_stylex_declarations(
    api: &str,
    object: &ObjectExpression<'_>,
    errors: &mut Vec<(u32, String)>,
) -> Vec<(String, String)> {
    let mut declarations = vec![];
    for property in &object.properties {
        let property = match property {
            ObjectPropertyKind::ObjectProperty(property) => property,
            ObjectPropertyKind::SpreadProperty(spread) => {
                errors.push(spread_error(api, spread));
                continue;
            }
        };
        let Some(name) = get_str_by_property_key(&property.key) else {
            errors.push(key_error(api, &property.key));
            continue;
        };
        let name = normalize_stylex_property(name.as_ref());
        match stylex_value(&name, &property.value) {
            Some(value) => declarations.push((name, optimize_value(&value).into_owned())),
            None => errors.push((
                property.value.span().start,
                runtime_value_error(api, &readable_code(&property.value)),
            )),
        }
    }
    declarations
}

/// Resolve a `vars.key` member access against the contracts `stylex.defineVars()`
/// produced, yielding the `var(--x)` reference the value compiles to.
fn var_reference<'v>(
    value: &Expression<'_>,
    var_refs: &'v FxHashMap<String, String>,
) -> Option<&'v str> {
    let Expression::StaticMemberExpression(member) = value else {
        return None;
    };
    let Expression::Identifier(object) = &member.object else {
        return None;
    };
    var_refs
        .get(&format!("{}.{}", object.name, member.property.name))
        .map(String::as_str)
}

/// Shorthand CSS properties that trigger a `StyleX` specificity warning.
/// Promoted from an 18-element `&[&str]` linear `.contains` scan to a
/// module-level `phf::Set` for an O(1) membership probe per `create()` property.
static SHORTHAND_PROPERTIES: phf::Set<&'static str> = phf::phf_set! {
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
};

type Leaf<'l> = dyn Fn(&str, &Expression<'_>) -> Option<String> + 'l;

/// Extract styles from a `stylex.create()` call's argument (`ObjectExpression`).
///
/// Handles static string/number values (Phase 1) and value-level conditions (Phase 2);
/// what cannot be read at build time is reported in `errors`.
///
/// Returns a Vec of `(namespace_name, style_props, css_vars, include_refs)` tuples. Each namespace
/// corresponds to a top-level key in the `stylex.create({...})` argument.
#[allow(clippy::type_complexity)]
pub fn extract_stylex_namespace_styles<'a>(
    obj: &ObjectExpression<'_>,
    keyframe_names: &FxHashMap<String, String>,
    var_refs: &FxHashMap<String, String>,
    errors: &mut Vec<(u32, String)>,
) -> Vec<(
    String,
    Vec<ExtractStyleProp<'a>>,
    Option<Vec<(usize, String, &'static str)>>,
    Vec<StylexIncludeRef>,
)> {
    // A keyframes name or a `defineVars` member reads as the value it stands for
    let leaf = |property: &str, value: &Expression<'_>| {
        if let Expression::Identifier(ident) = value
            && let Some(name) = keyframe_names.get(ident.name.as_str())
        {
            return Some(name.clone());
        }
        var_reference(value, var_refs)
            .map(str::to_string)
            .or_else(|| stylex_value(property, value).map(std::borrow::Cow::into_owned))
    };

    let mut result = vec![];
    for prop in &obj.properties {
        let prop = match prop {
            ObjectPropertyKind::ObjectProperty(prop) => prop,
            ObjectPropertyKind::SpreadProperty(spread) => {
                errors.push(spread_error("stylex.create", spread));
                continue;
            }
        };
        let Some(ns_name) = get_string_by_property_key(&prop.key) else {
            errors.push(key_error("stylex.create", &prop.key));
            continue;
        };
        match &prop.value {
            Expression::ArrowFunctionExpression(arrow) => {
                let Some((styles, css_vars)) =
                    extract_stylex_dynamic_namespace(arrow, &leaf, errors)
                else {
                    errors.push((
                        prop.value.span().start,
                        build_time_error(
                            "stylex.create",
                            &readable_code(&prop.value),
                            "a dynamic style is an arrow function with plain parameters returning an object literal",
                        ),
                    ));
                    continue;
                };
                result.push((ns_name, styles, Some(css_vars), vec![]));
            }
            Expression::ObjectExpression(ns_obj) => {
                let (styles, include_refs) = extract_stylex_namespace(ns_obj, &leaf, errors);
                result.push((ns_name, styles, None, include_refs));
            }
            Expression::NullLiteral(_) => result.push((ns_name, vec![], None, vec![])),
            value => errors.push((
                value.span().start,
                build_time_error(
                    "stylex.create",
                    &readable_code(value),
                    "a namespace is an object of styles or an arrow function returning one",
                ),
            )),
        }
    }
    result
}

/// The styles and `include()` references of one static namespace
fn extract_stylex_namespace<'a>(
    namespace: &ObjectExpression<'_>,
    leaf: &Leaf<'_>,
    errors: &mut Vec<(u32, String)>,
) -> (Vec<ExtractStyleProp<'a>>, Vec<StylexIncludeRef>) {
    let mut styles = vec![];
    let mut include_refs = vec![];
    for style_prop in &namespace.properties {
        let style_prop = match style_prop {
            ObjectPropertyKind::ObjectProperty(style_prop) => style_prop,
            ObjectPropertyKind::SpreadProperty(spread) => {
                match include(spread) {
                    Some(Ok(include_ref)) => include_refs.push(include_ref),
                    Some(Err(error)) => errors.push(error),
                    None => errors.push(spread_error("stylex.create", spread)),
                }
                continue;
            }
        };
        let Some(prop_name) = get_str_by_property_key(&style_prop.key) else {
            errors.push(key_error("stylex.create", &style_prop.key));
            continue;
        };

        // Phase 2: pseudo-element / pseudo-class top-level keys
        if prop_name.starts_with(':') {
            let Expression::ObjectExpression(inner_obj) = &style_prop.value else {
                errors.push((
                    style_prop.value.span().start,
                    build_time_error(
                        "stylex.create",
                        &readable_code(&style_prop.value),
                        "a pseudo-class or pseudo-element key takes an object of styles",
                    ),
                ));
                continue;
            };
            let parent_selectors = [SelectorPart::Pseudo(prop_name.to_string())];
            for inner_prop in &inner_obj.properties {
                let inner_prop = match inner_prop {
                    ObjectPropertyKind::ObjectProperty(inner_prop) => inner_prop,
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        errors.push(spread_error("stylex.create", spread));
                        continue;
                    }
                };
                let Some(inner_name) = get_str_by_property_key(&inner_prop.key) else {
                    errors.push(key_error("stylex.create", &inner_prop.key));
                    continue;
                };
                push_decomposed(
                    &mut styles,
                    decompose_value_conditions(
                        &normalize_stylex_property(inner_name.as_ref()),
                        &inner_prop.value,
                        &parent_selectors,
                        leaf,
                        errors,
                    ),
                );
            }
            continue;
        }

        let css_property = normalize_stylex_property(prop_name.as_ref());
        if SHORTHAND_PROPERTIES.contains(css_property.as_str()) {
            eprintln!(
                "[stylex] WARNING: Shorthand property '{css_property}' may cause unexpected specificity issues. Consider using longhand properties (e.g., 'marginTop', 'paddingLeft')."
            );
        }
        push_decomposed(
            &mut styles,
            decompose_value_conditions(&css_property, &style_prop.value, &[], leaf, errors),
        );
    }
    (styles, include_refs)
}

fn push_decomposed(styles: &mut Vec<ExtractStyleProp<'_>>, decomposed: Vec<DecomposedStyle>) {
    for style in decomposed {
        if let Some(value) = style.value {
            styles.push(raw_static_style(style.property, &value, style.selector));
        }
    }
}

/// `...stylex.include(base.member)`: `None` when the spread is not an
/// `include()` call
fn include(spread: &SpreadElement<'_>) -> Option<Result<StylexIncludeRef, (u32, String)>> {
    let Expression::CallExpression(call) = &spread.argument else {
        return None;
    };
    if !is_include_call_static(&call.callee) {
        return None;
    }
    if let Some(Expression::StaticMemberExpression(member)) =
        call.arguments.first().and_then(Argument::as_expression)
        && let Expression::Identifier(ident) = &member.object
    {
        return Some(Ok(StylexIncludeRef {
            var_name: ident.name.to_string(),
            member_name: member.property.name.to_string(),
            offset: spread.span.start,
        }));
    }
    Some(Err((
        spread.span.start,
        build_time_error(
            "stylex.include",
            &readable_code(&spread.argument),
            "it takes a namespace such as `styles.base`",
        ),
    )))
}

/// Extract styles from a dynamic `StyleX` namespace (arrow function).
/// Returns (`styles_for_css`, `css_vars`) where `css_vars` maps `param_index` to a CSS variable
/// name and the unit a number passed for it gets; `None` when the function is
/// not an arrow with plain parameters returning an object literal.
#[allow(clippy::type_complexity)]
fn extract_stylex_dynamic_namespace<'a>(
    arrow: &ArrowFunctionExpression<'_>,
    leaf: &Leaf<'_>,
    errors: &mut Vec<(u32, String)>,
) -> Option<(
    Vec<ExtractStyleProp<'a>>,
    Vec<(usize, String, &'static str)>,
)> {
    // 1. Extract parameter names
    let mut param_names = vec![];
    for param in &arrow.params.items {
        let BindingPattern::BindingIdentifier(ident) = &param.pattern else {
            return None;
        };
        param_names.push(ident.name.to_string());
    }

    // 2. Get body ObjectExpression from expression body: (x) => ({ ... })
    let Expression::ObjectExpression(body_obj) = arrow.body.as_expression()?.without_parentheses()
    else {
        return None;
    };

    // 3. Process each property
    let mut styles = vec![];
    let mut css_vars = vec![];

    for prop in &body_obj.properties {
        let prop = match prop {
            ObjectPropertyKind::ObjectProperty(prop) => prop,
            ObjectPropertyKind::SpreadProperty(spread) => {
                errors.push(spread_error("stylex.create", spread));
                continue;
            }
        };
        let Some(prop_name) = get_string_by_property_key(&prop.key) else {
            errors.push(key_error("stylex.create", &prop.key));
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
            let param_name = &param_names[param_idx];
            let style = ExtractDynamicStyle::new(&css_property, 0, param_name, None)
                .at(prop.value.span().start);
            css_vars.push((
                param_idx,
                style.variable_name(),
                dynamic_number_suffix(&css_property),
            ));
            styles.push(ExtractStyleProp::Static(ExtractStyleValue::Dynamic(style)));
            continue;
        }
        match leaf(&css_property, &prop.value) {
            Some(value) => styles.push(raw_static_style(css_property, &value, None)),
            None => errors.push((
                prop.value.span().start,
                build_time_error(
                    "stylex.create",
                    &readable_code(&prop.value),
                    "a dynamic style's value is one of its parameters or a static value; compute it before passing it",
                ),
            )),
        }
    }

    Some((styles, css_vars))
}
