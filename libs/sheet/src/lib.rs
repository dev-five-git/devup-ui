pub mod theme;

use crate::theme::Theme;
use css::{convert_property, to_kebab_case, PropertyType};
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
            format!(":{}", to_kebab_case(selector))
        } else {
            String::new()
        };
        match convert_property(self.property.as_str()) {
            PropertyType::Single(prop) => {
                format!(
                    ".{}{}{{{}:{}}}",
                    self.class_name,
                    selector,
                    prop,
                    convert_theme_variable_value(&self.value)
                )
            }
            PropertyType::Multi(multi) => format!(
                ".{}{}{{{}}}",
                self.class_name,
                selector,
                multi
                    .into_iter()
                    .map(|prop| format!("{}:{};", prop, convert_theme_variable_value(&self.value)))
                    .collect::<Vec<String>>()
                    .join("")
            ),
        }
    }
}

fn convert_theme_variable_value(value: &String) -> String {
    if let Some(value) = value.strip_prefix("$") {
        format!("var(--{})", value)
    } else {
        value.to_string()
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
    pub theme: Theme,
    theme_declaration: String,
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
            theme_declaration: String::new(),
            theme: Theme::new(),
        }
    }

    pub fn add_property(
        &mut self,
        class_name: &str,
        property: &str,
        level: u8,
        value: &str,
        selector: Option<&str>,
    ) -> bool {
        let prop = StyleSheetProperty {
            class_name: class_name.to_string(),
            property: property.to_string(),
            value: value.to_string(),
            selector: selector.map(|s| s.to_string()),
        };
        self.properties.entry(level).or_default().insert(prop)
    }

    pub fn add_css(&mut self, class_name: String, css: String) -> bool {
        let prop = StyleSheetCss { class_name, css };
        self.css.insert(prop)
    }

    pub fn set_theme(&mut self, theme: Theme) {
        let mut theme_declaration = String::new();
        theme_declaration.push_str(theme.to_css().as_str());
        self.theme = theme;
        self.theme_declaration = theme_declaration;
    }

    pub fn create_css(&self) -> String {
        let mut css = self.theme_declaration.clone();
        for (level, props) in self.properties.iter() {
            // If has a selector property, move it to the back
            let (mut select_props, other_props): (Vec<_>, Vec<_>) =
                props.iter().partition(|prop| prop.selector.is_some());
            let mut sorted_props = other_props;
            sorted_props.append(&mut select_props);

            let inner_css = sorted_props
                .into_iter()
                .map(|prop| prop.extract())
                .collect::<Vec<String>>()
                .join("");
            if *level == 0 {
                css.push_str(inner_css.as_str());
            } else {
                css.push_str(
                    format!(
                        "\n@media (min-width:{}px){{{}}}",
                        self.theme
                            .break_points
                            .iter()
                            .enumerate()
                            .find(|(idx, _)| (*idx as u8) == *level)
                            .map(|(_, bp)| *bp)
                            .unwrap_or_else(|| self
                                .theme
                                .break_points
                                .last()
                                .cloned()
                                .unwrap_or(0)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_theme_variable_value() {
        assert_eq!(convert_theme_variable_value(&"1px".to_string()), "1px");
        assert_eq!(
            convert_theme_variable_value(&"$var".to_string()),
            "var(--var)"
        );
    }
}
