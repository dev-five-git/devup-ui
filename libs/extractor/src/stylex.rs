use std::borrow::Cow;

use css::keyframes_to_keyframes_name;
use css::style_selector::{AtRuleKind, StyleSelector};
use oxc_ast::ast::{Expression, ObjectPropertyKind};

use crate::utils::{
    get_string_by_literal_expression, get_string_by_property_key, js_number_literal,
};

/// Which `StyleX` function a named import refers to
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StylexFunction {
    Create,
    Props,
    Attrs,
    Keyframes,
    DefineVars,
    CreateTheme,
    CreateThemeContract,
    DefineConsts,
    PositionTry,
    ViewTransitionClass,
}

impl StylexFunction {
    #[must_use]
    pub fn from_export_name(value: &str) -> Option<Self> {
        match value {
            "create" => Some(StylexFunction::Create),
            "props" => Some(StylexFunction::Props),
            "attrs" => Some(StylexFunction::Attrs),
            "keyframes" => Some(StylexFunction::Keyframes),
            "defineVars" => Some(StylexFunction::DefineVars),
            "createTheme" => Some(StylexFunction::CreateTheme),
            "createThemeContract" => Some(StylexFunction::CreateThemeContract),
            "defineConsts" => Some(StylexFunction::DefineConsts),
            "positionTry" => Some(StylexFunction::PositionTry),
            "viewTransitionClass" => Some(StylexFunction::ViewTransitionClass),
            _ => None,
        }
    }
}

/// The custom property `stylex.defineVars()` in `filename` declares for `key`;
/// a module importing it computes the same name
#[must_use]
pub fn define_vars_variable(filename: &str, key: &str, split_filename: Option<&str>) -> String {
    format!(
        "--{}",
        keyframes_to_keyframes_name(&format!("sxv-{filename}-{key}"), split_filename)
    )
}

/// The class `stylex.createTheme()` in `filename` applies to `contract`, the
/// name that module binds the contract to
#[must_use]
pub fn create_theme_class(filename: &str, contract: &str, split_filename: Option<&str>) -> String {
    keyframes_to_keyframes_name(&format!("sxt-{filename}-{contract}"), split_filename)
}

#[must_use]
pub fn css_variable_block(selector: &str, assignments: &[(String, String)]) -> String {
    let mut css = String::new();
    css.push_str(selector);
    css.push('{');
    for (name, value) in assignments {
        css.push_str(name);
        css.push(':');
        css.push_str(value);
        css.push(';');
    }
    css.push('}');
    css
}

/// Check if a call expression is `stylex.firstThatWorks()` or named `firstThatWorks()`.
pub fn is_first_that_works_call(callee: &Expression) -> bool {
    // stylex.firstThatWorks(...)
    if let Expression::StaticMemberExpression(member) = callee
        && member.property.name.as_str() == "firstThatWorks"
    {
        return true;
    }
    // firstThatWorks(...) (named import)
    if let Expression::Identifier(ident) = callee
        && ident.name.as_str() == "firstThatWorks"
    {
        return true;
    }
    false
}

/// Check if a call expression is `stylex.include()` or named `include()`.
/// This is a static check that does NOT require access to the visitor.
pub fn is_include_call_static(callee: &Expression) -> bool {
    if let Expression::StaticMemberExpression(member) = callee
        && member.property.name.as_str() == "include"
    {
        return true;
    }
    if let Expression::Identifier(ident) = callee
        && ident.name.as_str() == "include"
    {
        return true;
    }
    false
}

/// A reference to a stylex.include(base.member) call found inside `stylex.create()`.
#[derive(Debug, Clone)]
pub struct StylexIncludeRef {
    pub var_name: String,
    pub member_name: String,
}

/// Check if a call expression is `stylex.types.X()` or `types.X()` (type wrapper).
pub fn is_types_call(callee: &Expression) -> bool {
    if let Expression::StaticMemberExpression(member) = callee {
        // stylex.types.X(...)
        if let Expression::StaticMemberExpression(inner) = &member.object {
            return inner.property.name.as_str() == "types";
        }
        // types.X(...) (named import)
        if let Expression::Identifier(ident) = &member.object {
            return ident.name.as_str() == "types";
        }
    }
    false
}

/// Convert camelCase CSS property name to kebab-case.
/// `StyleX` uses standard CSS properties only — NO devup-ui shorthand expansion.
pub fn normalize_stylex_property(name: &str) -> String {
    css::utils::to_kebab_case(name).into_owned()
}

/// Properties `StyleX` leaves unitless when given a number.
static UNITLESS_NUMBER_PROPERTIES: phf::Set<&'static str> = phf::phf_set! {
    "webkit-line-clamp",
    "animation-iteration-count",
    "aspect-ratio",
    "border-image-outset",
    "border-image-slice",
    "border-image-width",
    "counter-set",
    "counter-reset",
    "column-count",
    "flex",
    "flex-grow",
    "flex-shrink",
    "flex-order",
    "grid-row",
    "grid-row-start",
    "grid-row-end",
    "grid-column",
    "grid-column-start",
    "grid-column-end",
    "grid-area",
    "font-size-adjust",
    "font-weight",
    "hyphenate-limit-chars",
    "line-clamp",
    "line-height",
    "mask-border-outset",
    "mask-border-slice",
    "mask-border-width",
    "opacity",
    "order",
    "orphans",
    "tab-size",
    "widows",
    "z-index",
    "fill-opacity",
    "flood-opacity",
    "rotate",
    "scale",
    "shape-image-threshold",
    "stop-opacity",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-miterlimit",
    "stroke-opacity",
    "stroke-width",
    "math-depth",
    "zoom",
};

/// Properties whose numbers `StyleX` reads as milliseconds.
static TIME_PROPERTIES: phf::Set<&'static str> = phf::phf_set! {
    "animation-delay",
    "animation-duration",
    "transition-delay",
    "transition-duration",
    "voice-duration",
};

/// Properties whose dynamic values `StyleX` gives a unit when they are numbers.
static LENGTH_PROPERTIES: phf::Set<&'static str> = phf::phf_set! {
    "background-position-x",
    "background-position-y",
    "block-size",
    "border-block-end-width",
    "border-block-start-width",
    "border-block-width",
    "border-vertical-width",
    "border-bottom-left-radius",
    "border-bottom-right-radius",
    "border-bottom-width",
    "border-end-end-radius",
    "border-end-start-radius",
    "border-inline-end-width",
    "border-end-width",
    "border-inline-start-width",
    "border-start-width",
    "border-inline-width",
    "border-horizontal-width",
    "border-left-width",
    "border-right-width",
    "border-spacing",
    "border-start-end-radius",
    "border-start-start-radius",
    "border-top-left-radius",
    "border-top-right-radius",
    "border-top-width",
    "bottom",
    "column-gap",
    "column-rule-width",
    "column-width",
    "contain-intrinsic-block-size",
    "contain-intrinsic-height",
    "contain-intrinsic-inline-size",
    "contain-intrinsic-width",
    "flex-basis",
    "font-size",
    "font-smooth",
    "height",
    "inline-size",
    "inset-block-end",
    "inset-block-start",
    "inset-inline-end",
    "inset-inline-start",
    "left",
    "letter-spacing",
    "margin-block-end",
    "margin-block-start",
    "margin-bottom",
    "margin-inline-end",
    "margin-end",
    "margin-inline-start",
    "margin-start",
    "margin-left",
    "margin-right",
    "margin-top",
    "max-block-size",
    "max-height",
    "max-inline-size",
    "max-width",
    "min-block-size",
    "min-height",
    "min-inline-size",
    "min-width",
    "offset-distance",
    "outline-offset",
    "outline-width",
    "overflow-clip-margin",
    "padding-block-end",
    "padding-block-start",
    "padding-bottom",
    "padding-inline-end",
    "padding-end",
    "padding-inline-start",
    "padding-start",
    "padding-left",
    "padding-right",
    "padding-top",
    "perspective",
    "right",
    "row-gap",
    "scroll-margin-block-end",
    "scroll-margin-block-start",
    "scroll-margin-bottom",
    "scroll-margin-inline-end",
    "scroll-margin-inline-start",
    "scroll-margin-left",
    "scroll-margin-right",
    "scroll-margin-top",
    "scroll-padding-block-end",
    "scroll-padding-block-start",
    "scroll-padding-bottom",
    "scroll-padding-inline-end",
    "scroll-padding-inline-start",
    "scroll-padding-left",
    "scroll-padding-right",
    "scroll-padding-top",
    "scroll-snap-margin-bottom",
    "scroll-snap-margin-left",
    "scroll-snap-margin-right",
    "scroll-snap-margin-top",
    "shape-margin",
    "tab-size",
    "text-decoration-thickness",
    "text-indent",
    "text-underline-offset",
    "top",
    "transform-origin",
    "translate",
    "vertical-align",
    "width",
    "word-spacing",
    "border",
    "border-block",
    "border-block-end",
    "border-block-start",
    "border-bottom",
    "border-left",
    "border-radius",
    "border-right",
    "border-top",
    "border-width",
    "column-rule",
    "contain-intrinsic-size",
    "gap",
    "inset",
    "inset-block",
    "inset-inline",
    "margin",
    "margin-block",
    "margin-vertical",
    "margin-inline",
    "margin-horizontal",
    "offset",
    "outline",
    "padding",
    "padding-block",
    "padding-vertical",
    "padding-inline",
    "padding-horizontal",
    "scroll-margin",
    "scroll-margin-block",
    "scroll-margin-inline",
    "scroll-padding",
    "scroll-padding-block",
    "scroll-padding-inline",
    "scroll-snap-margin",
};

/// The unit `StyleX` appends to a number on `property`.
fn number_suffix(property: &str) -> &'static str {
    if UNITLESS_NUMBER_PROPERTIES.contains(property) || property.starts_with("--") {
        ""
    } else if TIME_PROPERTIES.contains(property) {
        "ms"
    } else {
        "px"
    }
}

/// The unit a dynamic value on `property` gets when it is a number at runtime.
pub fn dynamic_number_suffix(property: &str) -> &'static str {
    if TIME_PROPERTIES.contains(property) || LENGTH_PROPERTIES.contains(property) {
        number_suffix(property)
    } else {
        ""
    }
}

/// A literal `StyleX` value as CSS text: a number gets the unit `StyleX` gives it.
pub fn stylex_value<'a>(property: &str, value: &Expression<'a>) -> Option<Cow<'a, str>> {
    js_number_literal(value).map_or_else(
        || get_string_by_literal_expression(value),
        |number| {
            // `+ 0.0` turns `-0` into `0`, as JS prints it.
            let rounded = (number * 10_000.0).round() / 10_000.0 + 0.0;
            Some(Cow::Owned(format!("{rounded}{}", number_suffix(property))))
        },
    )
}

/// Intermediate selector parts collected during recursion.
#[derive(Debug, Clone)]
pub enum SelectorPart {
    /// Pseudo-class or pseudo-element, e.g. ":hover", "`::placeholder`"
    Pseudo(String),
    /// At-rule condition, e.g. @media (max-width: 600px)
    AtRule { kind: AtRuleKind, query: String },
}

/// A single decomposed style entry from a value-level condition object.
#[derive(Debug)]
pub struct DecomposedStyle {
    pub property: String,
    /// `None` means null (no CSS emitted, tracked for atomic override).
    pub value: Option<String>,
    pub selector: Option<StyleSelector>,
}

/// Information about a dynamic `StyleX` namespace (arrow function in `stylex.create()`)
#[derive(Debug, Clone)]
pub struct StylexDynamicInfo {
    /// Combined class name string for all properties (static + dynamic)
    pub class_name: String,
    /// (`param_index`, `css_variable_name`, unit for a number) for each dynamic property
    pub css_vars: Vec<(usize, String, &'static str)>,
}

/// A `StyleX` namespace entry — either static or dynamic (arrow function)
#[derive(Debug, Clone)]
pub enum StylexNamespaceValue {
    /// Static namespace: just a className string
    Static(String),
    /// Dynamic namespace (from arrow function): className + CSS variable mappings
    Dynamic(StylexDynamicInfo),
}

/// Decompose a `StyleX` value-level condition object into flat (`css_value`, selector) tuples.
///
/// `StyleX` allows values to be objects with condition keys:
/// ```js
/// { color: { default: 'red', ':hover': 'blue', '@media (max-width:600px)': 'green' } }
/// ```
///
/// This recursively walks the value tree and returns flat tuples of (`value_or_none`, selector).
/// `None` value means null/no CSS emitted (but tracked for atomic override).
pub fn decompose_value_conditions(
    css_property: &str,
    value: &Expression,
    parent_selectors: &[SelectorPart],
) -> Vec<DecomposedStyle> {
    // String or number literal → leaf
    if let Some(s) = stylex_value(css_property, value) {
        return decomposed_leaf(css_property, Some(s.into_owned()), parent_selectors)
            .into_iter()
            .collect();
    }

    // NullLiteral → tracked but no CSS
    if matches!(value, Expression::NullLiteral(_)) {
        return decomposed_leaf(css_property, None, parent_selectors)
            .into_iter()
            .collect();
    }

    // CallExpression: firstThatWorks() → multiple fallback values with current selectors
    if let Expression::CallExpression(call) = value
        && is_first_that_works_call(&call.callee)
    {
        let mut results = vec![];
        for arg in call.arguments.iter().rev() {
            if let Some(arg_expr) = arg.as_expression()
                && let Some(s) = stylex_value(css_property, arg_expr)
            {
                results.extend(decomposed_leaf(
                    css_property,
                    Some(s.into_owned()),
                    parent_selectors,
                ));
            }
        }
        return results;
    }

    // CallExpression: types.*() → extract inner value, pass through selectors
    if let Expression::CallExpression(call) = value
        && is_types_call(&call.callee)
        && let Some(inner) = call.arguments.first().and_then(|arg| arg.as_expression())
    {
        if let Some(s) = stylex_value(css_property, inner) {
            return decomposed_leaf(css_property, Some(s.into_owned()), parent_selectors)
                .into_iter()
                .collect();
        }
        return vec![];
    }

    // ObjectExpression → recurse into condition keys
    let Expression::ObjectExpression(obj) = value else {
        return vec![];
    };

    let mut results = vec![];

    for prop in &obj.properties {
        let ObjectPropertyKind::ObjectProperty(prop) = prop else {
            continue;
        };
        let Some(key) = get_string_by_property_key(&prop.key) else {
            continue;
        };

        if key == "default" {
            results.extend(decompose_value_conditions(
                css_property,
                &prop.value,
                parent_selectors,
            ));
        } else if key.starts_with("::") || key.starts_with(':') {
            let mut new_selectors = parent_selectors.to_vec();
            new_selectors.push(SelectorPart::Pseudo(key));
            results.extend(decompose_value_conditions(
                css_property,
                &prop.value,
                &new_selectors,
            ));
        } else if let Some((kind, query)) = parse_at_rule_key(&key) {
            let mut new_selectors = parent_selectors.to_vec();
            new_selectors.push(SelectorPart::AtRule { kind, query });
            results.extend(decompose_value_conditions(
                css_property,
                &prop.value,
                &new_selectors,
            ));
        }
    }

    results
}

/// A leaf style under `parent_selectors`, or `None` when those conditions can
/// never match together (e.g. `@media print` inside `@media screen`).
fn decomposed_leaf(
    css_property: &str,
    value: Option<String>,
    parent_selectors: &[SelectorPart],
) -> Option<DecomposedStyle> {
    let mut pseudo_str: Option<String> = None;
    for p in parent_selectors {
        if let SelectorPart::Pseudo(s) = p {
            pseudo_str
                .get_or_insert_with(|| String::from("&"))
                .push_str(s);
        }
    }

    // Every enclosing at-rule applies, outermost first.
    let mut selector = pseudo_str.map(StyleSelector::Selector);
    for p in parent_selectors {
        if let SelectorPart::AtRule { kind, query } = p {
            selector = Some(StyleSelector::nest_at_rule(
                selector.as_ref(),
                *kind,
                query,
            )?);
        }
    }

    Some(DecomposedStyle {
        property: css_property.to_string(),
        value,
        selector,
    })
}

/// Parse an at-rule key like `"@media (max-width: 600px)"` into kind + query.
fn parse_at_rule_key(key: &str) -> Option<(AtRuleKind, String)> {
    key.strip_prefix("@media")
        .map(|q| (AtRuleKind::Media, q.trim().to_string()))
        .or_else(|| {
            key.strip_prefix("@supports")
                .map(|q| (AtRuleKind::Supports, q.trim().to_string()))
        })
        .or_else(|| {
            key.strip_prefix("@container")
                .map(|q| (AtRuleKind::Container, q.trim().to_string()))
        })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_stylex_property() {
        assert_eq!(
            normalize_stylex_property("backgroundColor"),
            "background-color"
        );
        assert_eq!(normalize_stylex_property("fontSize"), "font-size");
        assert_eq!(normalize_stylex_property("color"), "color");
        assert_eq!(normalize_stylex_property("zIndex"), "z-index");
    }

    #[test]
    fn test_decompose_folds_every_at_rule() {
        let allocator = oxc_allocator::Allocator::default();
        let source = "({ default: 'red', '@media print': { ':hover': { '@media (prefers-reduced-motion: reduce)': 'blue', '@media screen': 'green' } } })";
        let program = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts())
            .parse()
            .program;
        let oxc_ast::ast::Statement::ExpressionStatement(statement) = &program.body[0] else {
            panic!("expected expression statement");
        };

        let value = statement.expression.without_parentheses();
        let styles: Vec<_> = decompose_value_conditions("color", value, &[])
            .into_iter()
            .map(|style| (style.value, style.selector.map(|s| s.to_string())))
            .collect();
        assert_eq!(
            styles,
            vec![
                (Some("red".to_string()), None),
                (
                    Some("blue".to_string()),
                    Some("@media print and (prefers-reduced-motion:reduce) &:hover".to_string())
                ),
            ]
        );
    }
}
