use crate::extract_style::{ExtractStyleProperty, extract_css::ExtractCss, extract_dynamic_style::ExtractDynamicStyle, extract_font_face::ExtractFontFace, extract_import::ExtractImport, extract_keyframes::ExtractKeyframes, extract_static_style::ExtractStaticStyle, style_property::StyleProperty};

#[derive(Debug, PartialEq, Clone, Eq, Hash, Ord, PartialOrd)]
pub enum ExtractStyleValue {
    Static(ExtractStaticStyle),
    Typography(String),
    Dynamic(ExtractDynamicStyle),
    Css(ExtractCss),
    Import(ExtractImport),
    FontFace(ExtractFontFace),
    Keyframes(ExtractKeyframes),
}

impl ExtractStyleValue {
    pub fn extract(&self, filename: Option<&str>) -> Option<StyleProperty> {
        match self {
            ExtractStyleValue::Static(style) => Some(style.extract(filename)),
            ExtractStyleValue::Dynamic(style) => Some(style.extract(filename)),
            ExtractStyleValue::Keyframes(keyframes) => Some(keyframes.extract(filename)),
            ExtractStyleValue::Typography(typo) => Some(StyleProperty::ClassName(format!("typo-{typo}"))),
            ExtractStyleValue::Css(_) | ExtractStyleValue::Import(_) | ExtractStyleValue::FontFace(_) => None,
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
        let mut style = ExtractStyleValue::Static(ExtractStaticStyle::new("margin", "10px", 0, None));
        style.set_style_order(1);
        if let ExtractStyleValue::Static(style) = style {
            assert_eq!(style.style_order(), Some(1));
        }
        let mut style = ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("margin", 0, "10px", None));
        style.set_style_order(1);
        if let ExtractStyleValue::Dynamic(style) = style {
            assert_eq!(style.style_order(), Some(1));
        }
    }
    #[test]
    fn test_extract() {
        let style = ExtractStaticStyle::new("margin", "10px", 0, None);
        let value = ExtractStyleValue::Static(style);
        let extracted = value.extract(None);
        assert!(matches!(extracted, Some(StyleProperty::ClassName(_))));

        let style = ExtractDynamicStyle::new("margin", 0, "10px", None);
        let value = ExtractStyleValue::Dynamic(style);
        let extracted = value.extract(None);
        assert!(matches!(extracted, Some(StyleProperty::Variable { .. })));

        let keyframes = ExtractKeyframes::default();
        let value = ExtractStyleValue::Keyframes(keyframes);
        let extracted = value.extract(None);
        assert!(matches!(extracted, Some(StyleProperty::ClassName(_))));

        let value = ExtractStyleValue::Typography("body1".to_string());
        let extracted = value.extract(None);
        assert!(matches!(extracted, Some(StyleProperty::ClassName(_))));

        let value = ExtractStyleValue::Css(ExtractCss { css: "".to_string(), file: "".to_string() });
        assert!(value.extract(None).is_none());

        let value = ExtractStyleValue::Import(ExtractImport { url: "".to_string(), file: "".to_string() });
        assert!(value.extract(None).is_none());
    }
}
