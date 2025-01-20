use crate::utils::convert_value;
use crate::StyleProperty;
use css::{css_to_classname, sheet_to_classname, sheet_to_variable_name};
use once_cell::sync::Lazy;
use std::collections::HashSet;

#[derive(Debug, PartialEq, Clone)]
pub struct ExtractStaticStyle {
    /// property
    property: String,
    /// fixed value
    value: String,
    /// responsive level
    level: u8,
    /// selector
    selector: Option<String>,
    /// basic, if the value is true then css created by this style will be added to the first
    basic: bool,
}

static MAINTAIN_VALUE_PROPERTIES: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set = HashSet::<String>::new();
    set.insert("opacity".to_string());
    set.insert("flex".to_string());
    set.insert("zIndex".to_string());
    set.insert("fontWeight".to_string());
    set.insert("scale".to_string());
    set
});

impl ExtractStaticStyle {
    /// create a new ExtractStaticStyle
    pub fn new(property: &str, value: &str, level: u8, selector: Option<&str>) -> Self {
        Self {
            value: if MAINTAIN_VALUE_PROPERTIES.contains(property) {
                value.to_string()
            } else {
                convert_value(value)
            },
            property: property.to_string(),
            level,
            selector: selector.map(|s| s.to_string()),
            basic: false,
        }
    }

    pub fn new_basic(property: &str, value: &str, level: u8, selector: Option<&str>) -> Self {
        Self {
            value: if MAINTAIN_VALUE_PROPERTIES.contains(property) {
                value.to_string()
            } else {
                convert_value(value)
            },
            property: property.to_string(),
            level,
            selector: selector.map(|s| s.to_string()),
            basic: true,
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

    pub fn selector(&self) -> Option<&str> {
        self.selector.as_deref()
    }

    pub fn basic(&self) -> bool {
        self.basic
    }
}

pub trait ExtractStyleProperty {
    /// extract style properties
    fn extract(&self) -> StyleProperty;
}

impl ExtractStyleProperty for ExtractStaticStyle {
    fn extract(&self) -> StyleProperty {
        StyleProperty::ClassName(sheet_to_classname(
            self.property.as_str(),
            self.level,
            Some(convert_value(self.value.as_str()).as_str()),
            if let Some(selector) = &self.selector {
                Some(selector.as_str())
            } else {
                None
            },
        ))
    }
}
#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
pub struct ExtractDynamicStyle {
    /// property
    property: String,
    /// responsive
    level: u8,
    identifier: String,

    /// selector
    selector: Option<String>,
}

impl ExtractDynamicStyle {
    /// create a new ExtractDynamicStyle
    pub fn new(property: &str, level: u8, identifier: &str, selector: Option<&str>) -> Self {
        Self {
            property: property.to_string(),
            level,
            identifier: identifier.to_string(),
            selector: selector.map(|s| s.to_string()),
        }
    }

    pub fn property(&self) -> &str {
        self.property.as_str()
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn selector(&self) -> Option<&str> {
        self.selector.as_deref()
    }

    pub fn identifier(&self) -> &str {
        self.identifier.as_str()
    }
}

impl ExtractStyleProperty for ExtractDynamicStyle {
    fn extract(&self) -> StyleProperty {
        StyleProperty::Variable {
            class_name: sheet_to_classname(
                self.property.as_str(),
                self.level,
                None,
                if let Some(selector) = &self.selector {
                    Some(selector.as_str())
                } else {
                    None
                },
            ),
            variable_name: sheet_to_variable_name(
                self.property.as_str(),
                self.level,
                if let Some(selector) = &self.selector {
                    Some(selector.as_str())
                } else {
                    None
                },
            ),
            identifier: self.identifier.clone(),
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
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
}
