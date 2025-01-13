use css::{convert_property, PropertyType};
use std::collections::{BTreeMap, HashSet};

trait ExtractStyle {
    fn extract(&self) -> String;
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct StyleSheetProperty {
    pub class_name: String,
    pub property: String,
    pub value: String,
    pub selector: Option<String>,
}

impl ExtractStyle for StyleSheetProperty {
    fn extract(&self) -> String {
        let selector = if let Some(selector) = &self.selector {
            format!(":{}", selector)
        } else {
            String::new()
        };
        match convert_property(self.property.as_str()) {
            PropertyType::Single(prop) => {
                format!(
                    ".{}{}{{{}:{}}}",
                    self.class_name, selector, prop, self.value
                )
            }
            PropertyType::Multi(multi) => format!(
                ".{}{}{{{}}}",
                self.class_name,
                selector,
                multi
                    .into_iter()
                    .map(|prop| format!("{}:{};", prop, self.value))
                    .collect::<Vec<String>>()
                    .join("")
            ),
        }
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
    /// level -> properties
    pub properties: BTreeMap<u8, HashSet<StyleSheetProperty>>,
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
            properties: BTreeMap::new(),
            css: HashSet::new(),
        }
    }

    pub fn add_property(
        &mut self,
        class_name: String,
        property: String,
        level: u8,
        value: String,
        selector: Option<String>,
    ) -> Option<String> {
        let prop = StyleSheetProperty {
            class_name,
            property,
            value,
            selector,
        };
        let css = prop.extract();
        if self.properties.entry(level).or_default().insert(prop) {
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
    pub fn create_css(&self, break_points: Vec<u16>) -> String {
        let mut css = String::new();
        for (level, props) in self.properties.iter() {
            let inner_css = props
                .iter()
                .map(|prop| prop.extract())
                .collect::<Vec<String>>()
                .join("");
            if *level == 0 {
                css.push_str(inner_css.as_str());
            } else {
                css.push_str(
                    format!(
                        "\n@media (min-width:{}px){{{}}}",
                        break_points
                            .iter()
                            .enumerate()
                            .find(|(idx, _)| (*idx as u8) == *level)
                            .map(|(_, bp)| *bp)
                            .unwrap_or_else(|| break_points.last().cloned().unwrap_or(0)),
                        inner_css
                    )
                    .as_str(),
                );
            }
        }
        for prop in self.css.iter() {
            css.push_str(&prop.extract());
        }
        css
    }
}
