use crate::{
    StyleProperty,
    extract_style::{
        ExtractStyleProperty, extract_css::ExtractCss, extract_dynamic_style::ExtractDynamicStyle,
        extract_import::ExtractImport, extract_static_style::ExtractStaticStyle,
    },
};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub enum ExtractStyleValue {
    Static(ExtractStaticStyle),
    Typography(String),
    Dynamic(ExtractDynamicStyle),
    Css(ExtractCss),
    Import(ExtractImport),
}

impl ExtractStyleValue {
    pub fn extract(&self) -> Option<StyleProperty> {
        match self {
            ExtractStyleValue::Static(style) => Some(style.extract()),
            ExtractStyleValue::Dynamic(style) => Some(style.extract()),
            ExtractStyleValue::Typography(typo) => {
                Some(StyleProperty::ClassName(format!("typo-{typo}")))
            }
            ExtractStyleValue::Css(_) | ExtractStyleValue::Import(_) => None,
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
    fn test_style_order() {
        let mut style =
            ExtractStyleValue::Static(ExtractStaticStyle::new("margin", "10px", 0, None));
        style.set_style_order(1);
        if let ExtractStyleValue::Static(style) = style {
            assert_eq!(style.style_order(), Some(1));
        }
        let mut style =
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("margin", 0, "10px", None));
        style.set_style_order(1);
        if let ExtractStyleValue::Dynamic(style) = style {
            assert_eq!(style.style_order(), Some(1));
        }
    }
}
