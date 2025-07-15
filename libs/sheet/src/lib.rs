pub mod theme;

use crate::theme::Theme;
use css::{PropertyType, StyleSelector, convert_property, merge_selector};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering::Equal;
use std::collections::{BTreeMap, BTreeSet, HashSet};

trait ExtractStyle {
    fn extract(&self) -> String;
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleSheetProperty {
    pub class_name: String,
    pub property: String,
    pub value: String,
    pub selector: Option<StyleSelector>,
}
impl PartialOrd for StyleSheetProperty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StyleSheetProperty {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self.selector.is_some(), other.selector.is_some()) {
            (true, true) => match self.selector.cmp(&other.selector) {
                Equal => match self.property.cmp(&other.property) {
                    Equal => self.value.cmp(&other.value),
                    val => val,
                },
                val => val,
            },
            (false, false) => match self.property.cmp(&other.property) {
                Equal => self.value.cmp(&other.value),
                prop => prop,
            },
            (a, b) => a.cmp(&b),
        }
    }
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

static VAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\w+").unwrap());

fn convert_theme_variable_value(value: &str) -> String {
    if value.contains("$") {
        VAR_RE
            .replace_all(value, |caps: &regex::Captures| {
                format!("var(--{})", &caps[0][1..])
            })
            .to_string()
    } else {
        value.to_string()
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct StyleSheetCss {
    pub class_name: String,
    pub css: String,
}

impl ExtractStyle for StyleSheetCss {
    fn extract(&self) -> String {
        format!(".{}{{{}}}", self.class_name, self.css)
    }
}

type PropertyMap = BTreeMap<u8, BTreeMap<u8, HashSet<StyleSheetProperty>>>;

fn deserialize_btree_map_u8<'de, D>(deserializer: D) -> Result<PropertyMap, D::Error>
where
    D: Deserializer<'de>,
{
    let mut result = BTreeMap::new();
    for (key, value) in
        BTreeMap::<String, BTreeMap<String, HashSet<StyleSheetProperty>>>::deserialize(
            deserializer,
        )?
    {
        let mut tmp_map = BTreeMap::new();

        for (key, value) in value.into_iter() {
            tmp_map.insert(key.parse().map_err(Error::custom)?, value);
        }

        result.insert(key.parse().map_err(Error::custom)?, tmp_map);
    }

    Ok(result)
}
#[derive(Default, Deserialize, Serialize, Debug)]
pub struct StyleSheet {
    #[serde(deserialize_with = "deserialize_btree_map_u8")]
    pub properties: PropertyMap,
    pub css: HashSet<StyleSheetCss>,
    #[serde(default)]
    pub imports: BTreeSet<String>,
    #[serde(skip)]
    pub theme: Theme,
}

impl StyleSheet {
    pub fn add_property(
        &mut self,
        class_name: &str,
        property: &str,
        level: u8,
        value: &str,
        selector: Option<&StyleSelector>,
        style_order: Option<u8>,
    ) -> bool {
        self.properties
            .entry(style_order.unwrap_or(255))
            .or_default()
            .entry(level)
            .or_default()
            .insert(StyleSheetProperty {
                class_name: class_name.to_string(),
                property: property.to_string(),
                value: value.to_string(),
                selector: selector.cloned(),
            })
    }

    pub fn add_import(&mut self, import: &str) {
        self.imports.insert(import.to_string());
    }

    pub fn add_css(&mut self, class_name: &str, css: &str) -> bool {
        self.css.insert(StyleSheetCss {
            class_name: class_name.to_string(),
            css: css.to_string(),
        })
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn create_css(&self) -> String {
        let mut css = self.theme.to_css();

        for import in self.imports.iter() {
            css.push_str(&format!("@import \"{}\";", import));
        }

        // order
        for (_, map) in self.properties.iter() {
            for (level, props) in map.iter() {
                let (mut global_props, rest): (Vec<_>, Vec<_>) = props
                    .iter()
                    .partition(|prop| matches!(prop.selector, Some(StyleSelector::Global(_))));
                global_props.sort();
                let (mut medias, mut sorted_props): (Vec<&StyleSheetProperty>, Vec<_>) = rest
                    .iter()
                    .partition(|prop| matches!(prop.selector, Some(StyleSelector::Media(_))));
                sorted_props.sort();
                medias.sort();
                let medias = {
                    let mut map = BTreeMap::new();
                    for prop in medias {
                        if let Some(StyleSelector::Media(media)) = &prop.selector {
                            map.entry(media).or_insert_with(Vec::new).push(prop);
                        }
                    }
                    map
                };

                let break_point = if *level == 0 {
                    None
                } else {
                    Some(
                        self.theme
                            .breakpoints
                            .iter()
                            .enumerate()
                            .find(|(idx, _)| (*idx as u8) == *level)
                            .map(|(_, bp)| *bp)
                            .unwrap_or_else(|| self.theme.breakpoints.last().cloned().unwrap_or(0)),
                    )
                };

                if !global_props.is_empty() {
                    let inner_css = global_props
                        .into_iter()
                        .map(ExtractStyle::extract)
                        .collect::<Vec<String>>()
                        .join("");
                    css.push_str(
                        if let Some(break_point) = break_point {
                            format!("\n@media (min-width:{break_point}px){{{inner_css}}}")
                        } else {
                            inner_css
                        }
                        .as_str(),
                    );
                }

                if !sorted_props.is_empty() {
                    let inner_css = sorted_props
                        .into_iter()
                        .map(ExtractStyle::extract)
                        .collect::<Vec<String>>()
                        .join("");
                    css.push_str(
                        if let Some(break_point) = break_point {
                            format!("\n@media (min-width:{break_point}px){{{inner_css}}}")
                        } else {
                            inner_css
                        }
                        .as_str(),
                    );
                }
                for (media, props) in medias {
                    let inner_css = props
                        .into_iter()
                        .map(ExtractStyle::extract)
                        .collect::<Vec<String>>()
                        .join("");
                    css.push_str(
                        if let Some(break_point) = break_point {
                            format!(
                                "\n@media (min-width:{break_point}px) and {media}{{{inner_css}}}"
                            )
                        } else {
                            format!("\n@media {}{{{}}}", media, inner_css.as_str())
                        }
                        .as_str(),
                    );
                }
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
        assert_eq!(convert_theme_variable_value("1px"), "1px");
        assert_eq!(convert_theme_variable_value("$var"), "var(--var)");

        assert_eq!(
            convert_theme_variable_value("$var $var"),
            "var(--var) var(--var)"
        );

        assert_eq!(
            convert_theme_variable_value("1px solid $red"),
            "1px solid var(--red)"
        );
    }

    #[test]
    fn test_create_css_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, None);
        sheet.add_property("test", "background", 1, "some", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "border", 0, "1px solid", None, None);
        sheet.add_property("test", "border-color", 0, "red", None, None);
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
            Some(&"hover".into()),
            None,
        );
        sheet.add_property("test", "background-color", 1, "some", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, None);
        sheet.add_property(
            "test",
            "background-color",
            1,
            "some",
            Some(&"hover".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, None);
        sheet.add_property("test", "background", 1, "some", None, None);
        assert_debug_snapshot!(sheet.create_css());
    }
    #[test]
    fn test_create_css_with_basic_sort_test() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background-color", 1, "red", None, Some(0));
        sheet.add_property("test", "background", 1, "some", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "border", 0, "1px solid", None, None);
        sheet.add_property("test", "border-color", 0, "red", None, Some(0));
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "display", 0, "flex", None, Some(0));
        sheet.add_property("test", "display", 0, "block", None, None);
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
            Some(&"hover".into()),
            None,
        );
        sheet.add_property("test", "background-color", 1, "some", None, Some(0));
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "display", 0, "flex", None, Some(0));
        sheet.add_property("test", "display", 0, "none", None, None);
        sheet.add_property("test", "display", 2, "flex", None, None);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_create_css() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 1, "40px", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_css("test", "display:flex;");
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 2, "40px", None, None);
        sheet.add_property("test", "my", 2, "40px", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "bg", 0, "red", Some(&"hover".into()), None);
        sheet.add_property("test", "bg", 0, "blue", Some(&"active".into()), None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "bg",
            0,
            "red",
            Some(&StyleSelector::from("groupFocusVisible")),
            None,
        );
        sheet.add_property(
            "test",
            "bg",
            0,
            "blue",
            Some(&StyleSelector::from("groupFocusVisible")),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "bg",
            0,
            "red",
            Some(&StyleSelector::from("groupFocusVisible")),
            None,
        );
        sheet.add_property(
            "test",
            "bg",
            0,
            "blue",
            Some(&StyleSelector::from("hover")),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "bg", 0, "red", Some(&"*:hover &".into()), None);
        sheet.add_property(
            "test",
            "bg",
            0,
            "blue",
            Some(&StyleSelector::from("groupFocusVisible")),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "bg",
            0,
            "red",
            Some(&["themeDark", "hover"].into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "bg",
            0,
            "red",
            Some(&["wrong", "hover"].into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "bg",
            0,
            "red",
            Some(&"*[disabled='true'] &:hover".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "bg",
            0,
            "red",
            Some(&"&[disabled='true']".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "bg",
            0,
            "red",
            Some(&"&[disabled='true'], &[disabled='true']".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_style_order_create_css() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 0, "40px", None, Some(1));
        sheet.add_property("test", "mx", 1, "40px", None, Some(1));
        sheet.add_property("test", "mx", 1, "44px", None, Some(1));
        sheet.add_property("test", "mx", 1, "50px", None, Some(2));
        sheet.add_property("test", "mx", 1, "60px", None, None);
        sheet.add_property("test", "mx", 0, "70px", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "bg", 0, "red", None, Some(3));
        sheet.add_property("test", "bg", 0, "blue", None, Some(17));
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn wrong_breakpoint() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 10, "40px", None, None);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_selector_with_prefix() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 1, "40px", Some(&"groupHover".into()), None);
        sheet.add_property("test", "mx", 2, "50px", Some(&"groupHover".into()), None);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_theme_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 0, "40px", Some(&"themeDark".into()), None);
        sheet.add_property("test", "my", 0, "40px", Some(&"themeDark".into()), None);
        sheet.add_property("test", "mx", 0, "50px", Some(&"themeLight".into()), None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 0, "50px", Some(&"themeLight".into()), None);
        sheet.add_property("test", "mx", 0, "41px", None, None);
        sheet.add_property("test", "mx", 0, "51px", Some(&"themeLight".into()), None);
        sheet.add_property("test", "mx", 0, "42px", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "mx",
            0,
            "50px",
            Some(&["themeLight", "active"].into()),
            None,
        );
        sheet.add_property(
            "test",
            "mx",
            0,
            "50px",
            Some(&["themeLight", "hover"].into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_print_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 0, "40px", Some(&"print".into()), None);
        sheet.add_property("test", "my", 0, "40px", Some(&"print".into()), None);

        sheet.add_property("test", "mx", 1, "40px", Some(&"print".into()), None);
        sheet.add_property("test", "my", 1, "40px", Some(&"print".into()), None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "mx", 0, "40px", Some(&"print".into()), None);
        sheet.add_property("test", "my", 0, "40px", None, None);

        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_deserialize() {
        {
            let sheet: StyleSheet = serde_json::from_str(
                r##"{
            "properties": {
                "255": {
                    "0": [
                        {
                            "className": "test",
                            "property": "mx",
                            "value": "40px",
                            "selector": null,
                            "basic": false
                        }
                    ]
                }
            },
            "css": [],
            "theme": {
                "breakPoints": [
                    640,
                    768,
                    1024,
                    1280
                ],
                "colors": {
                    "black": "#000",
                    "white": "#fff"
                },
                "typography": {}
            }
        }"##,
            )
            .unwrap();
            assert_debug_snapshot!(sheet);
        }

        {
            let sheet: Result<StyleSheet, _> = serde_json::from_str(
                r##"{
            "properties": {
                "wrong": [
                    {
                        "className": "test",
                        "property": "mx",
                        "value": "40px",
                        "selector": null,
                        "basic": false
                    }
                ]
            },
            "css": [],
            "theme": {
                "breakPoints": [
                    640,
                    768,
                    1024,
                    1280
                ],
                "colors": {
                    "black": "#000",
                    "white": "#fff"
                },
                "typography": {}
            }
        }"##,
            );
            assert!(sheet.is_err());
        }
    }

    #[test]
    fn test_create_css_with_global_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Global("div".to_string())),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&StyleSelector::Global("div".to_string())),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();

        sheet.add_property(
            "test",
            "background-color",
            2,
            "blue",
            Some(&StyleSelector::Global("div".to_string())),
            None,
        );
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&StyleSelector::Global("div".to_string())),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "blue",
            Some(&StyleSelector::Global("div".to_string())),
            Some(0),
        );
        sheet.add_property(
            "test",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Global("div".to_string())),
            Some(255),
        );
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_create_css_with_imports() {
        let mut sheet = StyleSheet::default();
        sheet.add_import("@devup-ui/core/css/global.css");
        sheet.add_import("@devup-ui/core/css/global2.css");
        sheet.add_import("@devup-ui/core/css/global3.css");
        sheet.add_import("@devup-ui/core/css/global4.css");
        assert_debug_snapshot!(sheet.create_css());
    }
}
