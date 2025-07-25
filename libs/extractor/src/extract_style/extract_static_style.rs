use css::{optimize_value::optimize_value, sheet_to_classname, style_selector::StyleSelector};

use crate::{
    extract_style::{
        ExtractStyleProperty, constant::MAINTAIN_VALUE_PROPERTIES, style_property::StyleProperty,
    },
    utils::convert_value,
};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractStaticStyle {
    /// property
    pub property: String,
    /// fixed value
    pub value: String,
    /// responsive level
    pub level: u8,
    /// selector
    pub selector: Option<StyleSelector>,
    /// None is inf, 0 is first, 1 is second, etc
    pub style_order: Option<u8>,
}

impl ExtractStaticStyle {
    /// create a new ExtractStaticStyle
    pub fn new(property: &str, value: &str, level: u8, selector: Option<StyleSelector>) -> Self {
        Self {
            value: optimize_value(&if MAINTAIN_VALUE_PROPERTIES.contains(property) {
                value.to_string()
            } else {
                convert_value(value)
            }),
            property: property.to_string(),
            level,
            selector,
            style_order: None,
        }
    }

    pub fn new_basic(
        property: &str,
        value: &str,
        level: u8,
        selector: Option<StyleSelector>,
    ) -> Self {
        Self {
            value: optimize_value(&if MAINTAIN_VALUE_PROPERTIES.contains(property) {
                value.to_string()
            } else {
                convert_value(value)
            }),
            property: property.to_string(),
            level,
            selector,
            style_order: Some(0),
        }
    }

    pub fn property(&self) -> &str {
        self.property.as_str()
    }

    pub fn value(&self) -> &str {
        self.value.as_str()
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn selector(&self) -> Option<&StyleSelector> {
        self.selector.as_ref()
    }

    pub fn style_order(&self) -> Option<u8> {
        self.style_order
    }
}

impl ExtractStyleProperty for ExtractStaticStyle {
    fn extract(&self) -> StyleProperty {
        let s = self.selector.clone().map(|s| s.to_string());
        StyleProperty::ClassName(sheet_to_classname(
            self.property.as_str(),
            self.level,
            Some(&optimize_value(
                &if MAINTAIN_VALUE_PROPERTIES.contains(self.property.as_str()) {
                    self.value.to_string()
                } else {
                    convert_value(self.value.as_str())
                },
            )),
            s.as_deref(),
            self.style_order,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_static_style() {
        let style = ExtractStaticStyle::new("color", "red", 0, None);
        assert_eq!(style.property(), "color");
        assert_eq!(style.value(), "red");
        assert_eq!(style.level(), 0);
        assert_eq!(style.selector(), None);
        assert_eq!(style.style_order(), None);
    }
}
