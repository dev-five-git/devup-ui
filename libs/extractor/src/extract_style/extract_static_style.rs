use css::{
    optimize_multi_css_value::{check_multi_css_optimize, optimize_mutli_css_value},
    optimize_value::optimize_value,
    sheet_to_classname,
    style_selector::{StyleSelector, optimize_selector},
};

use crate::{
    extract_style::{ExtractStyleProperty, constant::MAINTAIN_VALUE_PROPERTIES, style_property::StyleProperty},
    utils::{convert_value, gcd},
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
                if property == "aspect-ratio" && value.contains("/") {
                    if let [Ok(a), Ok(b)] = value.split('/').map(|v| v.trim().parse::<u32>()).collect::<Vec<_>>()[..] {
                        let gcd = gcd(a, b);
                        format!("{}/{}", a / gcd, b / gcd)
                    } else {
                        value.to_string()
                    }
                } else {
                    value.to_string()
                }
            } else {
                convert_value(value)
            }),
            property: property.to_string(),
            level,
            selector: selector.map(optimize_selector),
            style_order: None,
        }
    }

    pub fn new_basic(property: &str, value: &str, level: u8, selector: Option<StyleSelector>) -> Self {
        Self { value: optimize_value(&if MAINTAIN_VALUE_PROPERTIES.contains(property) { value.to_string() } else { convert_value(value) }), property: property.to_string(), level, selector, style_order: Some(0) }
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
    fn extract(&self, filename: Option<&str>) -> StyleProperty {
        let s = self.selector.clone().map(|s| s.to_string());
        let v = optimize_value(&if MAINTAIN_VALUE_PROPERTIES.contains(&self.property) { self.value.to_string() } else { convert_value(&self.value) });
        let v = if check_multi_css_optimize(&self.property) { optimize_mutli_css_value(&v) } else { v };
        StyleProperty::ClassName(sheet_to_classname(&self.property, self.level, Some(&v), s.as_deref(), self.style_order, filename))
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
