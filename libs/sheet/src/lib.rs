pub mod theme;

use crate::theme::Theme;
use css::{convert_property, merge_selector, PropertyType, StyleSelector};
use std::cmp::Ordering::{Greater, Less};
use std::collections::{BTreeMap, HashSet};

trait ExtractStyle {
    fn extract(&self) -> String;
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct StyleSheetProperty {
    pub class_name: String,
    pub property: String,
    pub value: String,
    pub selector: Option<StyleSelector>,
    pub basic: bool,
}

impl ExtractStyle for StyleSheetProperty {
    fn extract(&self) -> String {
        match convert_property(self.property.as_str()) {
            PropertyType::Single(prop) => {
                format!(
                    "{}{{{}:{}}}",
                    merge_selector(&self.class_name, self.selector.as_ref()),
                    prop,
                    convert_theme_variable_value(&self.value)
                )
            }
            PropertyType::Multi(multi) => format!(
                "{}{{{}}}",
                merge_selector(&self.class_name, self.selector.as_ref()),
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

#[derive(Default)]
pub struct StyleSheet {
    /// level -> properties
    pub properties: BTreeMap<u8, HashSet<StyleSheetProperty>>,
    pub css: HashSet<StyleSheetCss>,
    pub theme: Theme,
    theme_declaration: String,
}

impl StyleSheet {
    pub fn add_property(
        &mut self,
        class_name: &str,
        property: &str,
        level: u8,
        value: &str,
        selector: Option<&StyleSelector>,
        basic: bool,
    ) -> bool {
        let prop = StyleSheetProperty {
            class_name: class_name.to_string(),
            property: property.to_string(),
            value: value.to_string(),
            selector: selector.cloned(),
            basic,
        };
        self.properties.entry(level).or_default().insert(prop)
    }

    pub fn add_css(&mut self, class_name: &str, css: &str) -> bool {
        let prop = StyleSheetCss {
            class_name: class_name.to_string(),
            css: css.to_string(),
        };
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
            let mut sorted_props = props.iter().collect::<Vec<_>>();
            sorted_props.sort_by(|a, b| {
                if a.basic == b.basic {
                    match (a.selector.is_some(), b.selector.is_some()) {
                        (true, false) => Greater,
                        (false, true) => Less,
                        (true, true) => {
                            if a.selector == b.selector {
                                if a.property == b.property {
                                    a.value.cmp(&b.value)
                                } else {
                                    a.property.cmp(&b.property)
                                }
                            } else {
                                a.selector.cmp(&b.selector)
                            }
                        }
                        (false, false) => {
                            if a.property == b.property {
                                a.value.cmp(&b.value)
                            } else {
                                a.property.cmp(&b.property)
                            }
                        }
                    }
                } else {
                    b.basic.cmp(&a.basic)
                }
            });

            let inner_css = sorted_props
                .into_iter()
                .map(ExtractStyle::extract)
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
    use insta::assert_debug_snapshot;

    #[test]
    fn test_convert_theme_variable_value() {
        assert_eq!(convert_theme_variable_value(&"1px".to_string()), "1px");
        assert_eq!(
            convert_theme_variable_value(&"$var".to_string()),
            "var(--var)"
        );
    }

    #[test]
    fn test_create_css_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, false);
        sheet.add_property("test", "background", 1, "some", None, false);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "border", 0, "1px solid", None, false);
        sheet.add_property("test", "border-color", 0, "red", None, false);
        assert_debug_snapshot!(sheet.create_css());
    }
    #[test]
    fn test_create_css_with_selector_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&StyleSelector::Postfix("hover".to_string())),
            false,
        );
        sheet.add_property("test", "background-color", 1, "some", None, false);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, false);
        sheet.add_property(
            "test",
            "background-color",
            1,
            "some",
            Some(&StyleSelector::Postfix("hover".to_string())),
            false,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, false);
        sheet.add_property("test", "background", 1, "some", None, false);
        assert_debug_snapshot!(sheet.create_css());
    }
    #[test]
    fn test_create_css_with_basic_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, true);
        sheet.add_property("test", "background", 1, "some", None, false);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "border", 0, "1px solid", None, false);
        sheet.add_property("test", "border-color", 0, "red", None, true);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "display", 0, "flex", None, true);
        sheet.add_property("test", "display", 0, "block", None, false);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_create_css_with_selector_and_basic_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&StyleSelector::Postfix("hover".to_string())),
            false,
        );
        sheet.add_property("test", "background-color", 1, "some", None, true);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "display", 0, "flex", None, true);
        sheet.add_property("test", "display", 0, "none", None, false);
        sheet.add_property("test", "display", 2, "flex", None, false);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_create_css() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 1, "40px", None, false);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_css("test", "display:flex;");
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 2, "40px", None, false);
        sheet.add_property("test", "my", 2, "40px", None, false);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn wrong_breakpoint() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 10, "40px", None, false);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_selector_with_prefix() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 1, "40px", Some(&"groupHover".into()), false);
        sheet.add_property("test", "mx", 2, "50px", Some(&"groupHover".into()), false);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_theme_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 0, "40px", Some(&"themeDark".into()), false);
        sheet.add_property("test", "my", 0, "40px", Some(&"themeDark".into()), false);
        sheet.add_property("test", "mx", 0, "50px", Some(&"themeLight".into()), false);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 0, "50px", Some(&"themeLight".into()), false);
        sheet.add_property("test", "mx", 0, "41px", None, false);
        sheet.add_property("test", "mx", 0, "51px", Some(&"themeLight".into()), false);
        sheet.add_property("test", "mx", 0, "42px", None, false);
        assert_debug_snapshot!(sheet.create_css());
    }
}
