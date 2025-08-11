pub mod class_map;
mod constant;
pub mod debug;
pub mod is_special_property;
pub mod optimize_multi_css_value;
pub mod optimize_value;
pub mod rm_css_comment;
mod selector_separator;
pub mod style_selector;
pub mod utils;

use std::hash::{DefaultHasher, Hash, Hasher};

use crate::class_map::GLOBAL_CLASS_MAP;
use crate::constant::{COLOR_HASH, F_SPACE_RE, GLOBAL_STYLE_PROPERTY, ZERO_RE};
use crate::debug::is_debug;
use crate::optimize_value::optimize_value;
use crate::style_selector::StyleSelector;
use crate::utils::to_kebab_case;

pub fn merge_selector(class_name: &str, selector: Option<&StyleSelector>) -> String {
    if let Some(selector) = selector {
        match selector {
            StyleSelector::Selector(value) => value.replace("&", &format!(".{class_name}")),
            StyleSelector::Media { selector: s, .. } => {
                if let Some(s) = s {
                    s.replace("&", &format!(".{class_name}"))
                } else {
                    format!(".{class_name}")
                }
            }
            StyleSelector::Global(v, _) => v.to_string(),
        }
    } else {
        format!(".{class_name}")
    }
}

pub fn disassemble_property(property: &str) -> Vec<String> {
    GLOBAL_STYLE_PROPERTY
        .get(property)
        .map(|v| match v.len() {
            1 => vec![v[0].to_string()],
            _ => v.iter().map(|v| v.to_string()).collect(),
        })
        .unwrap_or_else(|| {
            vec![if (property.starts_with("Webkit")
                && property.len() > 6
                && property.chars().nth(6).unwrap().is_uppercase())
                || (property.starts_with("Moz")
                    && property.len() > 3
                    && property.chars().nth(3).unwrap().is_uppercase())
                || (property.starts_with("ms")
                    && property.len() > 2
                    && property.chars().nth(2).unwrap().is_uppercase())
            {
                format!("-{}", to_kebab_case(property))
            } else {
                to_kebab_case(property)
            }]
        })
}

pub fn keyframes_to_keyframes_name(keyframes: &str) -> String {
    if is_debug() {
        format!("k-{keyframes}")
    } else {
        let key = format!("k-{keyframes}");
        let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
        map.get(&key).map(|v| format!("k{v}")).unwrap_or_else(|| {
            let len = map.len();
            map.insert(key, len as i32);
            format!("k{}", map.len() - 1)
        })
    }
}

pub fn sheet_to_classname(
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: Option<&str>,
    style_order: Option<u8>,
) -> String {
    if is_debug() {
        let selector = selector.unwrap_or_default().trim();
        format!(
            "{}-{}-{}-{}-{}",
            property.trim(),
            level,
            optimize_value(value.unwrap_or_default()),
            if selector.is_empty() {
                "".to_string()
            } else {
                let mut hasher = DefaultHasher::new();
                selector.hash(&mut hasher);
                hasher.finish().to_string()
            },
            style_order.unwrap_or(255)
        )
    } else {
        let key = format!(
            "{}-{}-{}-{}-{}",
            property.trim(),
            level,
            optimize_value(value.unwrap_or_default()),
            selector.unwrap_or_default().trim(),
            style_order.unwrap_or(255)
        );
        let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
        map.get(&key).map(|v| format!("d{v}")).unwrap_or_else(|| {
            let len = map.len();
            map.insert(key, len as i32);
            format!("d{}", map.len() - 1)
        })
    }
}

pub fn sheet_to_variable_name(property: &str, level: u8, selector: Option<&str>) -> String {
    if is_debug() {
        let selector = selector.unwrap_or_default().trim();
        format!(
            "--{}-{}-{}",
            property,
            level,
            if selector.is_empty() {
                "".to_string()
            } else {
                let mut hasher = DefaultHasher::new();
                selector.hash(&mut hasher);
                hasher.finish().to_string()
            }
        )
    } else {
        let key = format!(
            "{}-{}-{}",
            property,
            level,
            selector.unwrap_or_default().trim()
        );
        let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
        map.get(&key).map(|v| format!("--d{v}")).unwrap_or_else(|| {
            let len = map.len();
            map.insert(key, len as i32);
            format!("--d{}", map.len() - 1)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        class_map::{get_class_map, reset_class_map, set_class_map},
        debug::set_debug,
    };

    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_sheet_to_variable_name() {
        reset_class_map();
        set_debug(false);
        assert_eq!(sheet_to_variable_name("background", 0, None), "--d0");
        assert_eq!(
            sheet_to_variable_name("background", 0, Some("hover")),
            "--d1"
        );
        assert_eq!(sheet_to_variable_name("background", 1, None), "--d2");
        assert_eq!(
            sheet_to_variable_name("background", 1, Some("hover")),
            "--d3"
        );
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_variable_name() {
        set_debug(true);
        assert_eq!(
            sheet_to_variable_name("background", 0, None),
            "--background-0-"
        );
        assert_eq!(
            sheet_to_variable_name("background", 0, Some("hover")),
            "--background-0-12448419602614487988"
        );
        assert_eq!(
            sheet_to_variable_name("background", 1, None),
            "--background-1-"
        );
        assert_eq!(
            sheet_to_variable_name("background", 1, Some("hover")),
            "--background-1-12448419602614487988"
        );
    }

    #[test]
    #[serial]
    fn test_sheet_to_classname() {
        set_debug(false);
        reset_class_map();
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None),
            "d0"
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), Some("hover"), None),
            "d1"
        );
        assert_eq!(sheet_to_classname("background", 1, None, None, None), "d2");
        assert_eq!(
            sheet_to_classname("background", 1, None, Some("hover"), None),
            "d3"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None),
            sheet_to_classname("background", 0, None, None, None)
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None),
            sheet_to_classname("background", 0, Some("red"), None, None),
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None),
            sheet_to_classname("  background  ", 0, Some("  red  "), None, None),
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None),
            sheet_to_classname("  background  ", 0, Some("red;"), None, None),
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("rgba(255, 0, 0,    0.5)"), None, None),
            sheet_to_classname("background", 0, Some("rgba(255,0,0,0.5)"), None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("rgba(255, 0, 0,    0.5)"), None, None),
            sheet_to_classname("background", 0, Some("rgba(255,0,0,.5)"), None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("rgba(255, 0, 0,    0.5)"), None, None),
            sheet_to_classname("background", 0, Some("#FF000080"), None, None),
        );

        {
            let map = GLOBAL_CLASS_MAP.lock().unwrap();
            assert_eq!(map.get("background-0-#FF000080--255"), Some(&2));
        }
        assert_eq!(
            sheet_to_classname("background", 0, Some("#fff"), None, None),
            sheet_to_classname("  background  ", 0, Some("#FFF"), None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffff"), None, None),
            sheet_to_classname("background", 0, Some("#FFF"), None, None),
        );

        {
            let map = GLOBAL_CLASS_MAP.lock().unwrap();
            assert_eq!(map.get("background-0-#FFF--255"), Some(&3));
        }

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffff"), None, None),
            sheet_to_classname("background", 0, Some("#FFFFFF"), None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffffAA"), None, None),
            sheet_to_classname("background", 0, Some("#FFFFFFaa"), None, None),
        );

        {
            let map = GLOBAL_CLASS_MAP.lock().unwrap();
            assert_eq!(map.get("background-0-#FFFA--255"), Some(&4));
        }
        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("color-mix(in srgb,var(--primary) 80%,    #000 20%)"),
                None,
                None
            ),
            sheet_to_classname(
                "background",
                0,
                Some("color-mix(in srgb,    var(--primary) 80%, #000000 20%)"),
                None,
                None
            ),
        );

        reset_class_map();
        assert_eq!(sheet_to_classname("background", 0, None, None, None), "d0");
        assert_eq!(
            sheet_to_classname("background", 0, None, None, Some(1)),
            "d1"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("width", 0, Some("0px"), None, None),
            "d0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0em"), None, None),
            "d0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0rem"), None, None),
            "d0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0vh"), None, None),
            "d0"
        );
        assert_eq!(sheet_to_classname("width", 0, Some("0%"), None, None), "d0");
        assert_eq!(
            sheet_to_classname("width", 0, Some("0dvh"), None, None),
            "d0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0dvw"), None, None),
            "d0"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0vw"), None, None),
            "d0"
        );
        assert_eq!(sheet_to_classname("width", 0, Some("0"), None, None), "d0");
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0px red"), None, None),
            "d1"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0% red"), None, None),
            "d1"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0em red"), None, None),
            "d1"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0rem red"), None, None),
            "d1"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0vh red"), None, None),
            "d1"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0vw red"), None, None),
            "d1"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0dvh red"), None, None),
            "d1"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0dvw red"), None, None),
            "d1"
        );

        assert_eq!(
            sheet_to_classname("test", 0, Some("0px 0"), None, None),
            "d2"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0em 0"), None, None),
            "d2"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0rem 0"), None, None),
            "d2"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0vh 0"), None, None),
            "d2"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0vw 0"), None, None),
            "d2"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0dvh 0"), None, None),
            "d2"
        );

        assert_eq!(
            sheet_to_classname("test", 0, Some("0 0vh"), None, None),
            "d2"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0 0vw"), None, None),
            "d2"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("transition", 0, Some("all 0.3s ease-in-out"), None, None),
            "d0"
        );
        assert_eq!(
            sheet_to_classname("transition", 0, Some("all .3s ease-in-out"), None, None),
            "d0"
        );
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_classname() {
        set_debug(true);
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None),
            "background-0---255"
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), Some("hover"), None),
            "background-0-red-12448419602614487988-255"
        );
        assert_eq!(
            sheet_to_classname("background", 1, None, None, None),
            "background-1---255"
        );
        assert_eq!(
            sheet_to_classname("background", 1, Some("red"), Some("hover"), None),
            "background-1-red-12448419602614487988-255"
        );
    }

    #[test]
    fn test_merge_selector() {
        assert_eq!(merge_selector("cls", Some(&"hover".into())), ".cls:hover");
        assert_eq!(
            merge_selector("cls", Some(&"placeholder".into())),
            ".cls::placeholder"
        );
        assert_eq!(
            merge_selector("cls", Some(&"theme-dark".into())),
            ":root[data-theme=dark] .cls"
        );
        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::Selector(
                    ":root[data-theme=dark]:hover &".to_string(),
                )),
            ),
            ":root[data-theme=dark]:hover .cls"
        );
        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::Selector(
                    ":root[data-theme=dark]::placeholder &".to_string()
                )),
            ),
            ":root[data-theme=dark]::placeholder .cls"
        );

        assert_eq!(
            merge_selector("cls", Some(&["theme-dark", "hover"].into()),),
            ":root[data-theme=dark] .cls:hover"
        );
        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::Media {
                    query: "print".to_string(),
                    selector: None
                })
            ),
            ".cls"
        );

        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::Media {
                    query: "print".to_string(),
                    selector: Some("&:hover".to_string())
                })
            ),
            ".cls:hover"
        );

        assert_eq!(
            merge_selector(
                "cls",
                Some(&StyleSelector::Global(
                    "&".to_string(),
                    "file.ts".to_string()
                ))
            ),
            "&"
        );
    }

    #[test]
    #[serial]
    fn test_set_class_map() {
        let mut map = HashMap::new();
        map.insert("background-0-rgba(255,0,0,0.5)-".to_string(), 1);
        set_class_map(map);
        assert_eq!(get_class_map().len(), 1);
    }

    #[test]
    #[serial]
    fn test_keyframes_to_keyframes_name() {
        reset_class_map();
        set_debug(false);
        assert_eq!(keyframes_to_keyframes_name("spin"), "k0");
        assert_eq!(keyframes_to_keyframes_name("spin"), "k0");
        assert_eq!(keyframes_to_keyframes_name("spin2"), "k1");
        reset_class_map();
        set_debug(true);
        assert_eq!(keyframes_to_keyframes_name("spin"), "k-spin");
        assert_eq!(keyframes_to_keyframes_name("spin1"), "k-spin1");
    }
}
