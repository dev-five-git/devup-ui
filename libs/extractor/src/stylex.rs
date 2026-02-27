use css::style_selector::{AtRuleKind, StyleSelector};
use oxc_ast::ast::{Expression, ObjectPropertyKind};

use crate::utils::{get_string_by_literal_expression, get_string_by_property_key};

/// Which StyleX function a named import refers to
#[derive(Debug, Clone, PartialEq)]
pub enum StylexFunction {
    Create,
    Props,
    Keyframes,
    FirstThatWorks,
    DefineVars,
    CreateTheme,
    Include,
}

/// Check if a call expression is stylex.firstThatWorks() or named firstThatWorks().
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

/// Check if a call expression is stylex.include() or named include().
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

/// A reference to a stylex.include(base.member) call found inside stylex.create().
#[derive(Debug, Clone)]
pub struct StylexIncludeRef {
    pub var_name: String,
    pub member_name: String,
}

/// Check if a call expression is stylex.types.X() or types.X() (type wrapper).
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
/// StyleX uses standard CSS properties only — NO devup-ui shorthand expansion.
pub fn normalize_stylex_property(name: &str) -> String {
    css::utils::to_kebab_case(name)
}

/// Intermediate selector parts collected during recursion.
#[derive(Debug, Clone)]
pub enum SelectorPart {
    /// Pseudo-class or pseudo-element, e.g. ":hover", "::placeholder"
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

/// Information about a dynamic StyleX namespace (arrow function in stylex.create())
#[derive(Debug, Clone)]
pub struct StylexDynamicInfo {
    /// Combined class name string for all properties (static + dynamic)
    pub class_name: String,
    /// Maps (param_index, css_variable_name) for each dynamic property
    pub css_vars: Vec<(usize, String)>,
}

/// A StyleX namespace entry — either static or dynamic (arrow function)
#[derive(Debug, Clone)]
pub enum StylexNamespaceValue {
    /// Static namespace: just a className string
    Static(String),
    /// Dynamic namespace (from arrow function): className + CSS variable mappings
    Dynamic(StylexDynamicInfo),
}

/// Decompose a StyleX value-level condition object into flat (css_value, selector) tuples.
///
/// StyleX allows values to be objects with condition keys:
/// ```js
/// { color: { default: 'red', ':hover': 'blue', '@media (max-width:600px)': 'green' } }
/// ```
///
/// This recursively walks the value tree and returns flat tuples of (value_or_none, selector).
/// `None` value means null/no CSS emitted (but tracked for atomic override).
pub fn decompose_value_conditions(
    css_property: &str,
    value: &Expression,
    parent_selectors: &[SelectorPart],
) -> Vec<DecomposedStyle> {
    // String literal → leaf
    if let Some(s) = get_string_by_literal_expression(value) {
        return vec![DecomposedStyle {
            property: css_property.to_string(),
            value: Some(s),
            selector: compose_selectors(parent_selectors),
        }];
    }

    // NullLiteral → tracked but no CSS
    if matches!(value, Expression::NullLiteral(_)) {
        return vec![DecomposedStyle {
            property: css_property.to_string(),
            value: None,
            selector: compose_selectors(parent_selectors),
        }];
    }

    // CallExpression: firstThatWorks() → multiple fallback values with current selectors
    if let Expression::CallExpression(call) = value
        && is_first_that_works_call(&call.callee)
    {
        let mut results = vec![];
        for arg in call.arguments.iter().rev() {
            let arg_expr = arg.to_expression();
            if let Some(s) = get_string_by_literal_expression(arg_expr) {
                results.push(DecomposedStyle {
                    property: css_property.to_string(),
                    value: Some(s),
                    selector: compose_selectors(parent_selectors),
                });
            }
        }
        return results;
    }

    // CallExpression: types.*() → extract inner value, pass through selectors
    if let Expression::CallExpression(call) = value
        && is_types_call(&call.callee)
        && !call.arguments.is_empty()
    {
        let inner = call.arguments[0].to_expression();
        if let Some(s) = get_string_by_literal_expression(inner) {
            return vec![DecomposedStyle {
                property: css_property.to_string(),
                value: Some(s),
                selector: compose_selectors(parent_selectors),
            }];
        }
        return vec![];
    }

    // ObjectExpression → recurse into condition keys
    let Expression::ObjectExpression(obj) = value else {
        return vec![];
    };

    let mut results = vec![];

    for prop in obj.properties.iter() {
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

/// Compose a list of selector parts into a single `StyleSelector`.
fn compose_selectors(parts: &[SelectorPart]) -> Option<StyleSelector> {
    if parts.is_empty() {
        return None;
    }

    let pseudos: Vec<&str> = parts
        .iter()
        .filter_map(|p| match p {
            SelectorPart::Pseudo(s) => Some(s.as_str()),
            SelectorPart::AtRule { .. } => None,
        })
        .collect();

    let at_rules: Vec<(AtRuleKind, &str)> = parts
        .iter()
        .filter_map(|p| match p {
            SelectorPart::AtRule { kind, query } => Some((*kind, query.as_str())),
            SelectorPart::Pseudo(_) => None,
        })
        .collect();

    let pseudo_str = if pseudos.is_empty() {
        None
    } else {
        Some(format!("&{}", pseudos.join("")))
    };

    if at_rules.is_empty() {
        pseudo_str.map(StyleSelector::Selector)
    } else {
        let (kind, query) = at_rules.last().expect("at_rules is non-empty");
        Some(StyleSelector::At {
            kind: *kind,
            query: query.to_string(),
            selector: pseudo_str,
        })
    }
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
}
