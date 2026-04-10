use std::fmt::{Debug, Formatter};

use css::{
    optimize_value::optimize_value,
    sheet_to_classname, sheet_to_variable_name,
    style_selector::{StyleSelector, optimize_selector},
};

use crate::extract_style::{ExtractStyleProperty, style_property::StyleProperty};

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
        s.finish()
    }
}

/// Strip ` !important` from a dynamic style identifier, returning the cleaned
/// identifier and whether `!important` was found.
///
/// Handles JS expression code produced by `expression_to_code`, where the
/// `!important` text appears before a closing delimiter (backtick, quote) or
/// at the very end of the string.
fn strip_important(identifier: &str) -> (String, bool) {
    for str_symbol in ["", "`", "\"", "'"] {
        let suffix = format!(" !important{str_symbol}");
        if identifier.ends_with(&suffix) {
            let base = &identifier[..identifier.len() - suffix.len()];
            return (format!("{base}{str_symbol}"), true);
        }
    }
    (identifier.to_string(), false)
}

impl ExtractDynamicStyle {
    /// create a new ExtractDynamicStyle
    pub fn new(
        property: &str,
        level: u8,
        identifier: &str,
        selector: Option<StyleSelector>,
    ) -> Self {
        let optimized = optimize_value(identifier);
        let (identifier, important) = strip_important(&optimized);
        Self {
            property: property.to_string(),
            level,
            identifier,
            selector: selector.map(optimize_selector),
            style_order: None,
            important,
        }
    }

    pub fn property(&self) -> &str {
        self.property.as_str()
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn selector(&self) -> Option<&StyleSelector> {
        self.selector.as_ref()
    }

    pub fn identifier(&self) -> &str {
        self.identifier.as_str()
    }

    pub fn style_order(&self) -> Option<u8> {
        self.style_order
    }

    pub fn important(&self) -> bool {
        self.important
    }
}

impl ExtractStyleProperty for ExtractDynamicStyle {
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        let selector = self.selector.clone().map(|s| s.to_string());
        StyleProperty::Variable {
            class_name: sheet_to_classname(
                self.property.as_str(),
                self.level,
                None,
                selector.as_deref(),
                self.style_order,
                filename,
            ),
            variable_name: sheet_to_variable_name(
                self.property.as_str(),
                self.level,
                selector.as_deref(),
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
    fn test_strip_important_plain() {
        let (id, important) = strip_important("color");
        assert_eq!(id, "color");
        assert!(!important);
    }

    #[test]
    fn test_strip_important_template_literal() {
        // Template literal: `${color} !important`
        let (id, important) = strip_important("`${color} !important`");
        assert_eq!(id, "`${color}`");
        assert!(important);
    }

    #[test]
    fn test_strip_important_double_quote() {
        let (id, important) = strip_important("\"red !important\"");
        assert_eq!(id, "\"red\"");
        assert!(important);
    }

    #[test]
    fn test_strip_important_single_quote() {
        let (id, important) = strip_important("'red !important'");
        assert_eq!(id, "'red'");
        assert!(important);
    }

    #[test]
    fn test_strip_important_bare() {
        let (id, important) = strip_important("something !important");
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
