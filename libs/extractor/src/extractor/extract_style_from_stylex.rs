use crate::ExtractStyleProp;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::extract_style::extract_style_value::ExtractStyleValue;
use crate::stylex::assignments::is_final_assignment;
use crate::stylex::{
    DecomposedStyle, DynamicNamespace, SelectorPart, StylexIncludeRef, StylexResolver,
    decompose_value_conditions, is_first_that_works_call, is_include_call_static, is_types_call,
    normalize_stylex_property, stylex_value,
};
use css::style_selector::StyleSelector;
use oxc_ast::ast::{
    Argument, Expression, ObjectExpression, ObjectPropertyKind, PropertyKind, SpreadElement,
};

mod dynamic;
use oxc_span::GetSpan;
use rustc_hash::FxHashMap;

use crate::utils::{
    build_time_error, get_str_by_property_key, get_string_by_property_key, key_error,
    readable_code, runtime_value_error, spread_error,
};

/// Construct a static style directly — bypass `convert_value()` to avoid devup-ui
/// spacing transformations. `StyleX` values are raw CSS, only `optimize_value()`.
pub(crate) fn raw_static_style<'a>(
    property: String,
    value: &str,
    selector: Option<StyleSelector>,
) -> ExtractStyleProp<'a> {
    let value = crate::stylex::transitions::css_value(&property, value).into_owned();
    ExtractStyleProp::Static(ExtractStyleValue::Static(ExtractStaticStyle {
        property,
        value,
        level: 0,
        selector,
        style_order: None,
        layer: None,
        theme_token_resolution: Default::default(),
    }))
}

/// Why `value` is no declaration value: a genuine `StyleX` helper is read only
/// inside the API that takes it
pub(crate) fn declaration_error(
    api: &str,
    value: &Expression<'_>,
    resolver: StylexResolver<'_>,
) -> String {
    let code = readable_code(value);
    match value {
        Expression::CallExpression(call)
            if is_first_that_works_call(&call.callee, resolver)
                || is_include_call_static(&call.callee, resolver)
                || is_types_call(&call.callee, resolver) =>
        {
            build_time_error(
                api,
                &code,
                "its values must be literals; `firstThatWorks()`, `include()` and `types` are read only inside `stylex.create()` and `stylex.defineVars()`",
            )
        }
        _ => runtime_value_error(api, &code),
    }
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
/// Returns a Vec of `(namespace_name, style_props, css_vars, include_refs, key_groups)` tuples,
/// `key_groups` giving each top-level key of a static namespace and how many of its
/// styles it gives. Each namespace
/// corresponds to a top-level key in the `stylex.create({...})` argument.
#[allow(clippy::type_complexity)]
pub fn extract_stylex_namespace_styles<'a>(
    obj: &ObjectExpression<'_>,
    keyframe_names: &FxHashMap<String, String>,
    var_refs: &FxHashMap<String, String>,
    errors: &mut Vec<(u32, String)>,
    resolver: StylexResolver<'_>,
) -> Vec<(
    String,
    Vec<ExtractStyleProp<'a>>,
    Option<DynamicNamespace>,
    Vec<StylexIncludeRef>,
    Vec<(String, usize)>,
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
        if prop.method || prop.kind != PropertyKind::Init {
            errors.push((
                prop.span.start,
                build_time_error(
                    "stylex.create",
                    "method/getter/setter namespace",
                    "use a plain namespace object or a synchronous expression-bodied arrow",
                ),
            ));
            continue;
        }
        match &prop.value {
            Expression::ArrowFunctionExpression(arrow) => {
                let Some((styles, css_vars)) = dynamic::extract(arrow, &leaf, errors) else {
                    continue;
                };
                result.push((ns_name, styles, Some(css_vars), vec![], vec![]));
            }
            Expression::ObjectExpression(ns_obj) => {
                let (styles, include_refs, groups) =
                    extract_stylex_namespace(ns_obj, &leaf, errors, resolver);
                result.push((ns_name, styles, None, include_refs, groups));
            }
            Expression::NullLiteral(_) => result.push((ns_name, vec![], None, vec![], vec![])),
            Expression::FunctionExpression(function) => errors.push((
                function.span.start,
                build_time_error(
                    "stylex.create",
                    if function.generator {
                        "generator function expression"
                    } else if function.r#async {
                        "async function expression"
                    } else {
                        "function expression"
                    },
                    "use a synchronous expression-bodied arrow with plain scalar parameters",
                ),
            )),
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

/// The styles and `include()` references of one static namespace, with each
/// top-level key and how many of the styles it gives
#[allow(clippy::type_complexity)]
fn extract_stylex_namespace<'a>(
    namespace: &ObjectExpression<'_>,
    leaf: &Leaf<'_>,
    errors: &mut Vec<(u32, String)>,
    resolver: StylexResolver<'_>,
) -> (
    Vec<ExtractStyleProp<'a>>,
    Vec<StylexIncludeRef>,
    Vec<(String, usize)>,
) {
    let mut styles = vec![];
    let mut include_refs = vec![];
    let mut groups = vec![];
    for (index, style_prop) in namespace.properties.iter().enumerate() {
        let style_prop = match style_prop {
            ObjectPropertyKind::ObjectProperty(style_prop) => style_prop,
            ObjectPropertyKind::SpreadProperty(spread) => {
                match include(spread, resolver) {
                    Some(Ok(mut include_ref)) => {
                        include_ref.before_group = groups.len();
                        include_refs.push(include_ref);
                    }
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
        let final_assignment = is_final_assignment(&prop_name, &namespace.properties[index + 1..]);

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
            let before = styles.len();
            let mut has_entries = false;
            for (inner_index, inner_prop) in inner_obj.properties.iter().enumerate() {
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
                let decomposed = decompose_value_conditions(
                    &normalize_stylex_property(inner_name.as_ref()),
                    &inner_prop.value,
                    &parent_selectors,
                    leaf,
                    errors,
                    resolver,
                );
                if final_assignment
                    && is_final_assignment(&inner_name, &inner_obj.properties[inner_index + 1..])
                {
                    has_entries |= !decomposed.is_empty();
                    push_decomposed(&mut styles, decomposed);
                }
            }
            if has_entries {
                groups.push((prop_name.to_string(), styles.len() - before));
            }
            continue;
        }

        let css_property = normalize_stylex_property(prop_name.as_ref());
        if SHORTHAND_PROPERTIES.contains(css_property.as_str()) {
            eprintln!(
                "[stylex] WARNING: Shorthand property '{css_property}' may cause unexpected specificity issues. Consider using longhand properties (e.g., 'marginTop', 'paddingLeft')."
            );
        }
        let before = styles.len();
        let decomposed = decompose_value_conditions(
            &css_property,
            &style_prop.value,
            &[],
            leaf,
            errors,
            resolver,
        );
        if final_assignment && !decomposed.is_empty() {
            push_decomposed(&mut styles, decomposed);
            groups.push((prop_name.to_string(), styles.len() - before));
        }
    }
    (styles, include_refs, groups)
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
fn include(
    spread: &SpreadElement<'_>,
    resolver: StylexResolver<'_>,
) -> Option<Result<StylexIncludeRef, (u32, String)>> {
    let Expression::CallExpression(call) = &spread.argument else {
        return None;
    };
    if !is_include_call_static(&call.callee, resolver) {
        return None;
    }
    if let Err(error) = crate::stylex::validation::validate_helper_call(call, resolver) {
        return Some(Err(error));
    }
    if let Some(Expression::StaticMemberExpression(member)) =
        call.arguments.first().and_then(Argument::as_expression)
        && let Expression::Identifier(ident) = &member.object
    {
        return Some(Ok(StylexIncludeRef {
            var_name: ident.name.to_string(),
            member_name: member.property.name.to_string(),
            offset: call.span.start,
            before_group: 0,
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
