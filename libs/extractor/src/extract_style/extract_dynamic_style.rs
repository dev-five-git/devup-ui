use std::fmt::{Debug, Formatter};

use css::{
    sheet_to_classname, sheet_to_variable_name,
    style_selector::{StyleSelector, optimize_selector},
};

use crate::extract_style::{
    ExtractStyleProperty, extract_static_style::ExtractStaticStyle, style_property::StyleProperty,
};

/// The variable an element sets to override a style it writes before a spread
/// the build cannot read, and the value the style keeps while it is unset
#[derive(PartialEq, Clone, Eq, Hash, Ord, PartialOrd, Debug)]
struct Override {
    variable: String,
    fallback: String,
}

#[derive(PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractDynamicStyle {
    /// property
    property: String,
    /// responsive
    level: u8,
    identifier: String,

    /// selector
    selector: Option<StyleSelector>,

    pub(super) style_order: Option<u8>,

    /// Whether the value had `!important` that was stripped from the identifier
    important: bool,

    pub(crate) layer: Option<String>,

    /// Set when the style is a static value a runtime spread may override
    overridable: Option<Override>,
}

impl Debug for ExtractDynamicStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("ExtractDynamicStyle");
        s.field("property", &self.property)
            .field("level", &self.level)
            .field("identifier", &self.identifier)
            .field("selector", &self.selector)
            .field("style_order", &self.style_order);
        if self.important {
            s.field("important", &self.important);
        }
        if let Some(layer) = &self.layer {
            s.field("layer", layer);
        }
        if let Some(overridable) = &self.overridable {
            s.field("overridable", overridable);
        }
        s.finish()
    }
}

/// `(closing symbol, " !important" + closing symbol)` probe pairs for `strip_important`.
const IMPORTANT_SUFFIXES: [(&str, &str); 4] = [
    ("", " !important"),
    ("`", " !important`"),
    ("\"", " !important\""),
    ("'", " !important'"),
];

/// Strip ` !important` from a dynamic style identifier, returning the cleaned
/// identifier and whether `!important` was found.
///
/// Takes the identifier by value so the overwhelmingly common no-match path can
/// move the owned string straight through without allocating a fresh copy.
///
/// Handles JS expression code produced by `expression_to_code`, where the
/// `!important` text appears before a closing delimiter (backtick, quote) or
/// at the very end of the string.
fn strip_important(identifier: String) -> (String, bool) {
    // Overwhelmingly common case: no `!important` anywhere. A single substring
    // scan short-circuits before the 4-probe `strip_suffix` loop (each of which
    // scans from the string end). Byte-identical: any string that would match a
    // suffix necessarily contains `!important`, so no match is ever skipped.
    if !identifier.contains("!important") {
        return (identifier, false);
    }
    for (str_symbol, suffix) in IMPORTANT_SUFFIXES {
        if let Some(base) = identifier.strip_suffix(suffix) {
            // The bare-identifier case (`str_symbol == ""`, the most common) needs
            // no formatting machinery — a plain owned copy of `base` suffices.
            if str_symbol.is_empty() {
                return (base.to_string(), true);
            }
            return (format!("{base}{str_symbol}"), true);
        }
    }
    // No `!important` suffix — move the owned string through unchanged (zero alloc).
    (identifier, false)
}

/// `identifier`, the code the element runs, without the statement's semicolon;
/// only a template holding the whole CSS value loses the `;` ending it. The
/// code is not optimized as CSS, as that would rewrite the literals it passes on
fn runtime_code(identifier: &str) -> String {
    let code = identifier.trim();
    let code = code.strip_suffix(';').unwrap_or(code);
    match code
        .strip_prefix('`')
        .and_then(|code| code.strip_suffix('`'))
        .filter(|value| value.ends_with(';'))
    {
        Some(value) => format!("`{}`", value.trim_end_matches(';')),
        None => code.to_string(),
    }
}

/// `key` as the characters of a variable name, each other character by its code
fn escaped(key: &str) -> String {
    key.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_string()
            } else {
                format!("_u{:04x}_", u32::from(c))
            }
        })
        .collect()
}

impl ExtractDynamicStyle {
    /// create a new `ExtractDynamicStyle`
    pub fn new(
        property: &str,
        level: u8,
        identifier: &str,
        selector: Option<StyleSelector>,
    ) -> Self {
        // `optimize_value` returns `Cow`; `strip_important` takes ownership (and
        // the struct stores the `String`), so materialize the owned form here.
        let (identifier, important) = strip_important(runtime_code(identifier));
        Self {
            property: property.to_string(),
            level,
            identifier,
            selector: selector.map(optimize_selector),
            style_order: None,
            important,
            layer: None,
            overridable: None,
        }
    }

    /// `style` as a stylesheet rule that gives way to `identifier`, the code
    /// an element sets on a variable only the prop `key` it is written as
    /// reads, so every breakpoint of that prop shares it and no other prop
    /// setting the same property can reach it
    pub fn overridable(style: &ExtractStaticStyle, identifier: &str, key: &str) -> Self {
        let (fallback, important) = strip_important(style.value.clone());
        Self {
            property: style.property.clone(),
            level: style.level,
            identifier: identifier.to_string(),
            selector: None,
            style_order: style.style_order,
            important,
            layer: style.layer.clone(),
            overridable: Some(Override {
                variable: sheet_to_variable_name(
                    &format!("{}-spread-{}", style.property, escaped(key)),
                    0,
                    None,
                ),
                fallback,
            }),
        }
    }

    /// Give way to what `read` reads over the value set now, as a spread
    /// written after it replaces it: `read` is given that value
    pub fn overridden_by(&mut self, read: impl FnOnce(&str) -> String) {
        self.identifier = read(&self.identifier);
    }

    /// The value `var()` falls back to while no override is set
    pub fn fallback(&self) -> Option<&str> {
        self.overridable
            .as_ref()
            .map(|overridable| overridable.fallback.as_str())
    }

    pub const fn property(&self) -> &str {
        self.property.as_str()
    }

    pub const fn level(&self) -> u8 {
        self.level
    }

    pub const fn selector(&self) -> Option<&StyleSelector> {
        self.selector.as_ref()
    }

    pub const fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    pub const fn style_order(&self) -> Option<u8> {
        self.style_order
    }

    pub const fn important(&self) -> bool {
        self.important
    }

    pub fn layer(&self) -> Option<&str> {
        self.layer.as_deref()
    }
}

impl ExtractStyleProperty for ExtractDynamicStyle {
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        let selector = super::class_selector(self.selector.as_ref(), self.layer());
        // What the rule falls back to is part of the class, as another
        // fallback is another rule
        let rule = self.overridable.as_ref().map(|overridable| {
            let important = if self.important { " !important" } else { "" };
            format!(
                "var({},{}){important}",
                overridable.variable, overridable.fallback
            )
        });
        StyleProperty::Variable {
            class_name: sheet_to_classname(
                self.property.as_str(),
                self.level,
                rule.as_deref(),
                selector.as_deref(),
                self.style_order,
                filename,
            ),
            variable_name: self.overridable.as_ref().map_or_else(
                || sheet_to_variable_name(self.property.as_str(), self.level, selector.as_deref()),
                |overridable| overridable.variable.clone(),
            ),
            identifier: self.identifier.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dynamic_style() {
        let style = ExtractDynamicStyle::new("color", 0, "primary", None);
        assert_eq!(style.property(), "color");
        assert_eq!(style.level(), 0);
        assert_eq!(style.selector(), None);
        assert_eq!(style.identifier(), "primary");
        assert_eq!(style.style_order(), None);
        assert!(!style.important());
    }

    #[test]
    fn test_escaped_keeps_names_and_codes_every_other_character() {
        assert_eq!(escaped("background-color"), "background-color");
        assert_eq!(escaped("a_b c"), "a_u005f_b_u0020_c");
    }

    #[test]
    fn test_strip_important_plain() {
        let (id, important) = strip_important("color".to_string());
        assert_eq!(id, "color");
        assert!(!important);
    }

    #[test]
    fn test_debug_dynamic_style() {
        let style = ExtractDynamicStyle::new("color", 2, "value", None);

        assert_eq!(
            format!("{style:?}"),
            "ExtractDynamicStyle { property: \"color\", level: 2, identifier: \"value\", selector: None, style_order: None }"
        );
    }

    #[test]
    fn test_strip_important_when_not_a_suffix() {
        let (id, important) = strip_important("color !important fallback".to_string());

        assert_eq!(id, "color !important fallback");
        assert!(!important);
    }

    #[test]
    fn test_strip_important_template_literal() {
        // Template literal: `${color} !important`
        let (id, important) = strip_important("`${color} !important`".to_string());
        assert_eq!(id, "`${color}`");
        assert!(important);
    }

    #[test]
    fn test_strip_important_double_quote() {
        let (id, important) = strip_important("\"red !important\"".to_string());
        assert_eq!(id, "\"red\"");
        assert!(important);
    }

    #[test]
    fn test_strip_important_single_quote() {
        let (id, important) = strip_important("'red !important'".to_string());
        assert_eq!(id, "'red'");
        assert!(important);
    }

    #[test]
    fn test_strip_important_bare() {
        let (id, important) = strip_important("something !important".to_string());
        assert_eq!(id, "something");
        assert!(important);
    }

    #[test]
    fn test_dynamic_style_with_important() {
        let style = ExtractDynamicStyle::new("background", 0, "`${color} !important`", None);
        assert_eq!(style.property(), "background");
        assert_eq!(style.identifier(), "`${color}`");
        assert!(style.important());
    }
}
