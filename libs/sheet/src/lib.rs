pub mod theme;

use crate::theme::Theme;
use css::{merge_selector, style_selector::StyleSelector};
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

#[derive(Debug, Hash, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleSheetKeyframes {
    pub name: String,
    pub keyframes: BTreeMap<String, BTreeSet<StyleSheetProperty>>,
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
        format!(
            "{}{{{}:{}}}",
            merge_selector(&self.class_name, self.selector.as_ref()),
            self.property,
            convert_theme_variable_value(&self.value)
        )
    }
}

static VAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\w+").unwrap());
static INTERFACE_KEY_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z_$][a-zA-Z0-9_$]*$").unwrap());

fn convert_interface_key(key: &str) -> String {
    if INTERFACE_KEY_RE.is_match(key) {
        key.to_string()
    } else {
        format!("[`{}`]", key.replace("`", "\\`"))
    }
}

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

#[derive(Debug, Hash, Eq, PartialEq, Deserialize, Serialize, Ord, PartialOrd)]
pub struct StyleSheetCss {
    pub css: String,
}

impl ExtractStyle for StyleSheetCss {
    fn extract(&self) -> String {
        self.css.clone()
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
    pub css: BTreeMap<String, BTreeSet<StyleSheetCss>>,
    #[serde(default)]
    pub keyframes: BTreeMap<String, BTreeMap<String, Vec<(String, String)>>>,
    #[serde(default)]
    pub global_css_files: BTreeSet<String>,
    #[serde(default)]
    pub imports: BTreeMap<String, BTreeSet<String>>,
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
        // register global css file for cache
        if let Some(StyleSelector::Global(_, file)) = selector {
            self.global_css_files.insert(file.clone());
        }

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

    pub fn add_import(&mut self, file: &str, import: &str) {
        self.global_css_files.insert(file.to_string());
        self.imports
            .entry(file.to_string())
            .or_default()
            .insert(import.to_string());
    }

    pub fn add_css(&mut self, file: &str, css: &str) -> bool {
        self.global_css_files.insert(file.to_string());
        self.css
            .entry(file.to_string())
            .or_default()
            .insert(StyleSheetCss {
                css: css.to_string(),
            })
    }

    pub fn add_keyframes(
        &mut self,
        name: &str,
        keyframes: BTreeMap<String, Vec<(String, String)>>,
    ) -> bool {
        let map = self.keyframes.entry(name.to_string()).or_default();
        if map == &keyframes {
            return false;
        }
        map.clear();
        map.extend(keyframes);
        true
    }

    pub fn rm_global_css(&mut self, file: &str) -> bool {
        if !self.global_css_files.contains(file) {
            return false;
        }
        self.global_css_files.remove(file);
        self.css.remove(file);
        for map in self.properties.values_mut() {
            for props in map.values_mut() {
                props.retain(|prop| {
                    if let Some(StyleSelector::Global(_, f)) = prop.selector.as_ref() {
                        f != file
                    } else {
                        true
                    }
                });
            }
        }
        true
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn create_interface(
        &self,
        package_name: &str,
        color_interface_name: &str,
        typography_interface_name: &str,
        theme_interface_name: &str,
    ) -> String {
        let mut color_keys = HashSet::new();
        let mut typography_keys = HashSet::new();
        let mut theme_keys = HashSet::new();
        for color_theme in self.theme.colors.values() {
            color_theme.0.keys().for_each(|key| {
                color_keys.insert(key.clone());
            });
        }
        self.theme.typography.keys().for_each(|key| {
            typography_keys.insert(key.clone());
        });

        self.theme.colors.keys().for_each(|key| {
            theme_keys.insert(key.clone());
        });

        if color_keys.is_empty() && typography_keys.is_empty() {
            String::new()
        } else {
            format!(
                "import \"{}\";declare module \"{}\"{{interface {}{{{}}}interface {}{{{}}}interface {}{{{}}}}}",
                package_name,
                package_name,
                color_interface_name,
                color_keys
                    .into_iter()
                    .map(|key| format!("{}:null;", convert_interface_key(&format!("${key}"))))
                    .collect::<String>(),
                typography_interface_name,
                typography_keys
                    .into_iter()
                    .map(|key| format!("{}:null;", convert_interface_key(&key)))
                    .collect::<String>(),
                theme_interface_name,
                theme_keys
                    .into_iter()
                    // key to pascal
                    .map(|key| format!("{}:null;", convert_interface_key(&key)))
                    .collect::<String>()
            )
        }
    }

    pub fn create_css(&self) -> String {
        let mut css = self
            .imports
            .values()
            .flatten()
            .map(|import| format!("@import \"{import}\";"))
            .collect::<String>();
        css.push_str(&self.theme.to_css());

        for (name, map) in self.keyframes.iter() {
            css.push_str(&format!(
                "@keyframes {name}{{{}}}",
                map.iter()
                    .map(|(key, props)| format!(
                        "{key}{{{}}}",
                        props
                            .iter()
                            .map(|(key, value)| format!("{key}:{value}"))
                            .collect::<Vec<String>>()
                            .join(";")
                    ))
                    .collect::<String>()
            ));
        }

        for (_, _css) in self.css.iter() {
            for _css in _css.iter() {
                css.push_str(&_css.extract());
            }
        }

        // order
        for (_, map) in self.properties.iter() {
            for (level, props) in map.iter() {
                let (mut global_props, rest): (Vec<_>, Vec<_>) = props
                    .iter()
                    .partition(|prop| matches!(prop.selector, Some(StyleSelector::Global(_, _))));
                global_props.sort();
                let (mut medias, mut sorted_props): (Vec<&StyleSheetProperty>, Vec<_>) =
                    rest.iter().partition(|prop| {
                        matches!(
                            prop.selector,
                            Some(StyleSelector::Media {
                                query: _,
                                selector: _
                            })
                        )
                    });
                sorted_props.sort();
                medias.sort();
                let medias = {
                    let mut map = BTreeMap::new();
                    for prop in medias {
                        if let Some(StyleSelector::Media { query, .. }) = &prop.selector {
                            map.entry(query).or_insert_with(Vec::new).push(prop);
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
                    // 같은 selector 끼리 모아서 CSS를 생성해야 함
                    use std::collections::BTreeMap;
                    let mut selector_map: BTreeMap<_, Vec<_>> = BTreeMap::new();
                    for prop in global_props {
                        if let Some(StyleSelector::Global(selector, _)) = &prop.selector {
                            selector_map.entry(selector.clone()).or_default().push(prop);
                        }
                    }
                    let mut inner_css = String::new();
                    for (selector, props) in selector_map {
                        // props는 같은 selector를 가진 property들의 Vec
                        inner_css.push_str(&format!(
                            "{}{{{}}}",
                            selector,
                            props
                                .into_iter()
                                .map(|prop| format!("{}:{}", prop.property, prop.value))
                                .collect::<Vec<String>>()
                                .join(";")
                        ));
                    }
                    css.push_str(
                        if let Some(break_point) = break_point {
                            format!("@media(min-width:{break_point}px){{{inner_css}}}")
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
                        .collect::<String>();
                    css.push_str(
                        if let Some(break_point) = break_point {
                            format!("@media(min-width:{break_point}px){{{inner_css}}}")
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
                        .collect::<String>();
                    css.push_str(
                        if let Some(break_point) = break_point {
                            format!("@media(min-width:{break_point}px)and {media}{{{inner_css}}}")
                        } else {
                            format!(
                                "@media{}{{{}}}",
                                if media.starts_with("(") {
                                    media.clone()
                                } else {
                                    format!(" {media}")
                                },
                                inner_css.as_str()
                            )
                        }
                        .as_str(),
                    );
                }
            }
        }
        css
    }
}

#[cfg(test)]
mod tests {
    use crate::theme::{ColorTheme, Typography};

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
        sheet.add_property("test", "margin", 1, "40px", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_css("test.tsx", "div {display:flex;}");
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "margin", 2, "40px", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background", 0, "red", Some(&"hover".into()), None);
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&"active".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&StyleSelector::from("group-focus-visible")),
            None,
        );
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&StyleSelector::from("group-focus-visible")),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&StyleSelector::from("group-focus-visible")),
            None,
        );
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&StyleSelector::from("hover")),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"*:hover &".into()),
            None,
        );
        sheet.add_property(
            "test",
            "background",
            0,
            "blue",
            Some(&StyleSelector::from("group-focus-visible")),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&["theme-dark", "hover"].into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&["wrong", "hover"].into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"*[disabled='true'] &:hover".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"&[disabled='true']".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background",
            0,
            "red",
            Some(&"&[disabled='true'], &[disabled='true']".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_reset_global_css() {
        let mut sheet = StyleSheet::default();
        sheet.add_css("test.tsx", "div {display:flex;}");
        sheet.add_css("test2.tsx", "div {display:flex;}");
        assert_debug_snapshot!(sheet.create_css());

        sheet.rm_global_css("test.tsx");

        assert_debug_snapshot!(sheet.create_css());

        sheet.rm_global_css("wrong.tsx");

        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_style_order_create_css() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "margin-left", 0, "40px", None, Some(1));
        sheet.add_property("test", "margin-right", 0, "40px", None, Some(1));

        sheet.add_property("test", "margin-left", 1, "40px", None, Some(1));
        sheet.add_property("test", "margin-right", 1, "40px", None, Some(1));
        sheet.add_property("test", "margin-left", 1, "44px", None, Some(1));
        sheet.add_property("test", "margin-right", 1, "44px", None, Some(1));
        sheet.add_property("test", "margin-left", 1, "40px", None, Some(1));
        sheet.add_property("test", "margin-right", 1, "44px", None, Some(1));
        sheet.add_property("test", "margin-left", 1, "44px", None, Some(1));
        sheet.add_property("test", "margin-right", 1, "44px", None, Some(1));
        sheet.add_property("test", "margin-left", 1, "50px", None, Some(2));
        sheet.add_property("test", "margin-right", 1, "50px", None, Some(2));
        sheet.add_property("test", "margin-left", 1, "60px", None, None);
        sheet.add_property("test", "margin-right", 1, "60px", None, None);
        sheet.add_property("test", "margin-left", 0, "70px", None, None);
        sheet.add_property("test", "margin-right", 0, "70px", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "background", 0, "red", None, Some(3));
        sheet.add_property("test", "background", 0, "blue", None, Some(17));
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn wrong_breakpoint() {
        let mut sheet = StyleSheet::default();
        sheet.add_property("test", "margin-left", 10, "40px", None, None);
        sheet.add_property("test", "margin-right", 10, "40px", None, None);
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_selector_with_prefix() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            1,
            "40px",
            Some(&"group-hover".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            1,
            "40px",
            Some(&"group-hover".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-left",
            2,
            "50px",
            Some(&"group-hover".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            2,
            "50px",
            Some(&"group-hover".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_theme_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "40px",
            Some(&"theme-dark".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "40px",
            Some(&"theme-dark".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-top",
            0,
            "40px",
            Some(&"theme-dark".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-bottom",
            0,
            "40px",
            Some(&"theme-dark".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "50px",
            Some(&"theme-light".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "50px",
            Some(&"theme-light".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "50px",
            Some(&"theme-light".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "50px",
            Some(&"theme-light".into()),
            None,
        );
        sheet.add_property("test", "margin-left", 0, "41px", None, None);
        sheet.add_property("test", "margin-right", 0, "41px", None, None);
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "51px",
            Some(&"theme-light".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "51px",
            Some(&"theme-light".into()),
            None,
        );
        sheet.add_property("test", "margin-left", 0, "42px", None, None);
        sheet.add_property("test", "margin-right", 0, "42px", None, None);
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "50px",
            Some(&["theme-light", "active"].into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "50px",
            Some(&["theme-light", "active"].into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "50px",
            Some(&["theme-light", "hover"].into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "50px",
            Some(&["theme-light", "hover"].into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_print_selector() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "40px",
            Some(&"print".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "40px",
            Some(&"print".into()),
            None,
        );
        sheet.add_property("test", "margin-top", 0, "40px", Some(&"print".into()), None);
        sheet.add_property(
            "test",
            "margin-bottom",
            0,
            "40px",
            Some(&"print".into()),
            None,
        );

        sheet.add_property(
            "test",
            "margin-left",
            1,
            "40px",
            Some(&"print".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            1,
            "40px",
            Some(&"print".into()),
            None,
        );
        sheet.add_property("test", "margin-top", 1, "40px", Some(&"print".into()), None);
        sheet.add_property(
            "test",
            "margin-bottom",
            1,
            "40px",
            Some(&"print".into()),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-left",
            0,
            "40px",
            Some(&"print".into()),
            None,
        );
        sheet.add_property(
            "test",
            "margin-right",
            0,
            "40px",
            Some(&"print".into()),
            None,
        );
        sheet.add_property("test", "margin-top", 0, "40px", None, None);
        sheet.add_property("test", "margin-bottom", 0, "40px", None, None);

        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_selector_with_query() {
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "margin-top",
            0,
            "40px",
            Some(&StyleSelector::Media {
                query: "(min-width: 1024px)".to_string(),
                selector: Some("&:hover".to_string()),
            }),
            None,
        );
        sheet.add_property(
            "test",
            "margin-bottom",
            0,
            "40px",
            Some(&StyleSelector::Media {
                query: "(min-width: 1024px)".to_string(),
                selector: Some("&:hover".to_string()),
            }),
            None,
        );

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
            "css": {},
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
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());
        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();

        sheet.add_property(
            "test",
            "background-color",
            2,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            None,
        );
        sheet.add_property(
            "test",
            "background-color",
            1,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            None,
        );
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
        );
        sheet.add_property(
            "test",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(255),
        );
        assert_debug_snapshot!(sheet.create_css());

        sheet.add_property(
            "test",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test2.tsx".to_string(),
            )),
            Some(255),
        );

        sheet.add_property(
            "test2",
            "background-color",
            0,
            "red",
            Some(&StyleSelector::Selector("&:hover".to_string())),
            Some(255),
        );

        sheet.rm_global_css("test.tsx");
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            1,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
        );
        sheet.add_property(
            "test",
            "color",
            1,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
        );

        assert_debug_snapshot!(sheet.create_css());

        sheet.rm_global_css("test.tsx");
        assert_debug_snapshot!(sheet.create_css());

        let mut sheet = StyleSheet::default();
        sheet.add_property(
            "test",
            "background-color",
            0,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
        );
        sheet.add_property(
            "test",
            "color",
            0,
            "blue",
            Some(&StyleSelector::Global(
                "div".to_string(),
                "test2.tsx".to_string(),
            )),
            Some(0),
        );

        assert_debug_snapshot!(sheet.create_css());

        sheet.rm_global_css("test.tsx");
        assert_debug_snapshot!(sheet.create_css());
    }

    #[test]
    fn test_create_css_with_imports() {
        {
            let mut sheet = StyleSheet::default();
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            sheet.add_import("test2.tsx", "@devup-ui/core/css/global2.css");
            sheet.add_import("test3.tsx", "@devup-ui/core/css/global3.css");
            sheet.add_import("test4.tsx", "@devup-ui/core/css/global4.css");
            assert_debug_snapshot!(sheet.create_css());
        }
        {
            let mut sheet = StyleSheet::default();
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            sheet.add_import("test.tsx", "@devup-ui/core/css/new-global.css");
            assert_debug_snapshot!(sheet.create_css());
        }
        {
            let mut sheet = StyleSheet::default();
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            sheet.add_import("test.tsx", "@devup-ui/core/css/global.css");
            assert_debug_snapshot!(sheet.create_css());
        }
    }

    #[test]
    fn test_get_theme_interface() {
        let sheet = StyleSheet::default();
        assert_eq!(
            sheet.create_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            ""
        );

        let mut sheet = StyleSheet::default();
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        theme.add_color_theme("dark", color_theme);
        sheet.set_theme(theme);
        assert_eq!(
            sheet.create_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            "import \"package\";declare module \"package\"{interface ColorInterface{$primary:null;}interface TypographyInterface{}interface ThemeInterface{dark:null;}}"
        );

        // test wrong case
        let mut sheet = StyleSheet::default();
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("(primary)", "#000");
        theme.add_color_theme("dark", color_theme);
        theme.add_typography(
            "prim``ary",
            vec![Some(Typography::new(
                Some("Arial".to_string()),
                Some("16px".to_string()),
                Some("400".to_string()),
                Some("1.5".to_string()),
                Some("0.5".to_string()),
            ))],
        );
        sheet.set_theme(theme);
        assert_eq!(
            sheet.create_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "ThemeInterface"
            ),
            "import \"package\";declare module \"package\"{interface ColorInterface{[`$(primary)`]:null;}interface TypographyInterface{[`prim\\`\\`ary`]:null;}interface ThemeInterface{dark:null;}}"
        );
    }

    #[test]
    fn test_keyframes() {
        let mut sheet = StyleSheet::default();
        let mut keyframes: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();

        let mut from_props = BTreeSet::new();
        from_props.insert(StyleSheetProperty {
            class_name: String::from("test"),
            property: String::from("opacity"),
            value: String::from("0"),
            selector: None,
        });
        keyframes.insert(
            String::from("from"),
            vec![(String::from("opacity"), String::from("0"))],
        );

        let mut to_props = BTreeSet::new();
        to_props.insert(StyleSheetProperty {
            class_name: String::from("test"),
            property: String::from("opacity"),
            value: String::from("1"),
            selector: None,
        });
        keyframes.insert(
            String::from("to"),
            vec![(String::from("opacity"), String::from("1"))],
        );

        sheet.add_keyframes("fadeIn", keyframes);
        let past = sheet.create_css();
        assert_debug_snapshot!(past);

        let mut keyframes: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut from_props = BTreeSet::new();
        from_props.insert(StyleSheetProperty {
            class_name: String::from("test"),
            property: String::from("opacity"),
            value: String::from("0"),
            selector: None,
        });
        keyframes.insert(
            String::from("from"),
            vec![(String::from("opacity"), String::from("0"))],
        );

        let mut to_props = BTreeSet::new();
        to_props.insert(StyleSheetProperty {
            class_name: String::from("test"),
            property: String::from("opacity"),
            value: String::from("1"),
            selector: None,
        });
        keyframes.insert(
            String::from("to"),
            vec![(String::from("opacity"), String::from("1"))],
        );

        sheet.add_keyframes("fadeIn", keyframes);

        let now = sheet.create_css();
        assert_debug_snapshot!(now);
        assert_eq!(past, now);
    }
}
