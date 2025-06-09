use crate::StyleProperty;
use crate::utils::convert_value;
use css::{
    StyleSelector, css_to_classname, optimize_value, sheet_to_classname, sheet_to_variable_name,
    short_to_long,
};
use phf::phf_set;

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractStaticStyle {
    /// property
    property: String,
    /// fixed value
    value: String,
    /// responsive level
    level: u8,
    /// selector
    selector: Option<StyleSelector>,
    /// None is inf, 0 is first, 1 is second, etc
    style_order: Option<u8>,
}

static MAINTAIN_VALUE_PROPERTIES: phf::Set<&str> = phf_set! {
    "opacity",
    "flex",
    "zIndex",
    "lineClamp",
    "fontWeight",
    "scale",
    "aspectRatio",
    "flexGrow",
    "flexShrink",
    "order",
    "gridColumn",
    "gridColumnStart",
    "gridColumnEnd",
    "gridRow",
    "gridRowStart",
    "gridRowEnd",
    "animationIterationCount"
};

impl ExtractStaticStyle {
    /// create a new ExtractStaticStyle
    pub fn new(property: &str, value: &str, level: u8, selector: Option<StyleSelector>) -> Self {
        Self {
            value: optimize_value(&if MAINTAIN_VALUE_PROPERTIES.contains(property) {
                value.to_string()
            } else {
                convert_value(value)
            }),
            property: short_to_long(property),
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

pub trait ExtractStyleProperty {
    /// extract style properties
    fn extract(&self) -> StyleProperty;
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
#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractCss {
    /// css code
    pub css: String,
}

impl ExtractStyleProperty for ExtractCss {
    /// hashing css code to class name
    fn extract(&self) -> StyleProperty {
        StyleProperty::ClassName(css_to_classname(self.css.as_str()))
    }
}

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub struct ExtractDynamicStyle {
    /// property
    property: String,
    /// responsive
    level: u8,
    identifier: String,

    /// selector
    selector: Option<StyleSelector>,

    style_order: Option<u8>,
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
            property: short_to_long(property),
            level,
            identifier: identifier.to_string(),
            selector,
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
    fn extract(&self) -> StyleProperty {
        let selector = self.selector.clone().map(|s| s.to_string());
        StyleProperty::Variable {
            class_name: sheet_to_classname(
                self.property.as_str(),
                self.level,
                None,
                selector.as_deref(),
                self.style_order,
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
#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub enum ExtractStyleValue {
    Static(ExtractStaticStyle),
    Typography(String),
    Dynamic(ExtractDynamicStyle),
    Css(ExtractCss),
}

impl ExtractStyleValue {
    pub fn extract(&self) -> StyleProperty {
        match self {
            ExtractStyleValue::Static(style) => style.extract(),
            ExtractStyleValue::Dynamic(style) => style.extract(),
            ExtractStyleValue::Css(css) => css.extract(),
            ExtractStyleValue::Typography(typo) => {
                StyleProperty::ClassName(format!("typo-{}", typo))
            }
        }
    }
    pub fn set_style_order(&mut self, order: u8) {
        match self {
            ExtractStyleValue::Static(style) => {
                if style.style_order.is_none() {
                    style.style_order = Some(order);
                }
            }
            ExtractStyleValue::Dynamic(style) => {
                style.style_order = Some(order);
            }
            _ => {}
        }
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
    }

    #[test]
    fn test_extract_dynamic_style() {
        let style = ExtractDynamicStyle::new("color", 0, "primary", None);
        assert_eq!(style.property(), "color");
        assert_eq!(style.level(), 0);
        assert_eq!(style.selector(), None);
        assert_eq!(style.identifier(), "primary");
    }

    #[test]
    fn test_extract_basic_static_style() {
        let style = ExtractStaticStyle::new_basic("color", "red", 0, None);
        assert_eq!(style.property(), "color");
        assert_eq!(style.value(), "red");
        assert_eq!(style.level(), 0);
        assert_eq!(style.selector(), None);
    }
}
