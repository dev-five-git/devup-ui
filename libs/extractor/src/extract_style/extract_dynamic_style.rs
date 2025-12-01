use css::{
    optimize_value::optimize_value,
    sheet_to_classname, sheet_to_variable_name,
    style_selector::{StyleSelector, optimize_selector},
};

use crate::extract_style::{ExtractStyleProperty, style_property::StyleProperty};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractDynamicStyle {
    /// property
    property: String,
    /// responsive
    level: u8,
    identifier: String,

    /// selector
    selector: Option<StyleSelector>,

    pub(super) style_order: Option<u8>,
}

impl ExtractDynamicStyle {
    /// create a new ExtractDynamicStyle
    pub fn new(
        property: &str,
        level: u8,
        identifier: &str,
        selector: Option<StyleSelector>,
    ) -> Self {
        Self {
            property: property.to_string(),
            level,
            identifier: optimize_value(identifier),
            selector: selector.map(optimize_selector),
            style_order: None,
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
    }
}
