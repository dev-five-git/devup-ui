use std::collections::HashSet;

trait ExtractStyle {
    fn extract(&self) -> String;
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct StyleSheetProperty {
    pub class_name: String,
    pub property: String,
    pub value: String,
    pub media: Option<String>,
}

impl ExtractStyle for StyleSheetProperty {
    fn extract(&self) -> String {
        format!(".{}{{{}:{}}}", self.class_name, self.property, self.value)
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct StyleSheetCss {
    pub class_name: String,
    pub css: String,
}

impl ExtractStyle for StyleSheetCss {
    fn extract(&self) -> String {
        format!(".{}{{{}}}", self.class_name, self.css)
    }
}

pub struct StyleSheet {
    pub properties: HashSet<StyleSheetProperty>,
    pub css: HashSet<StyleSheetCss>,
}

impl Default for StyleSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl StyleSheet {
    pub fn new() -> Self {
        Self {
            properties: HashSet::new(),
            css: HashSet::new(),
        }
    }

    pub fn add_property(
        &mut self,
        class_name: String,
        property: String,
        value: String,
    ) -> Option<String> {
        let prop = StyleSheetProperty {
            class_name,
            property,
            value,
            media: None,
        };
        let css = prop.extract();
        if self.properties.insert(prop) {
            Some(css)
        } else {
            None
        }
    }

    pub fn add_css(&mut self, class_name: String, css: String) -> Option<String> {
        let prop = StyleSheetCss { class_name, css };
        let css = prop.extract();
        if self.css.insert(prop) {
            Some(css)
        } else {
            None
        }
    }
}
