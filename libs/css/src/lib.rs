pub mod class_map;
mod constant;
pub mod debug;
pub mod file_map;
pub mod is_special_property;
mod num_to_nm_base;
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
use crate::file_map::get_file_num_by_filename;
use crate::num_to_nm_base::num_to_nm_base;
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

pub fn keyframes_to_keyframes_name(keyframes: &str, filename: Option<&str>) -> String {
    if is_debug() {
        format!("k-{keyframes}")
    } else {
        let key = format!("k-{keyframes}");
        let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
        let filename = filename.unwrap_or_default().to_string();
        let class_num = map
            .entry(filename.to_string())
            .or_default()
            .get(&key)
            .map(|v| num_to_nm_base(*v).to_string())
            .unwrap_or_else(|| {
                let m = map.entry(filename.to_string()).or_default();
                let len = m.len();
                m.insert(key, len);
                num_to_nm_base(len).to_string()
            });
        if !filename.is_empty() {
            format!(
                "{}-{}",
                num_to_nm_base(get_file_num_by_filename(&filename)),
                class_num
            )
        } else {
            class_num
        }
    }
}

pub fn sheet_to_classname(
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: Option<&str>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> String {
    // base style
    let filename = if style_order == Some(0) {
        None
    } else {
        filename
    };
    if is_debug() {
        let selector = selector.unwrap_or_default().trim();
        format!(
            "{}-{}-{}-{}-{}{}",
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
            style_order.unwrap_or(255),
            filename
                .map(|v| format!("-{}", get_file_num_by_filename(v)))
                .unwrap_or_default(),
        )
    } else {
        let key = format!(
            "{}-{}-{}-{}-{}{}",
            property.trim(),
            level,
            optimize_value(value.unwrap_or_default()),
            selector.unwrap_or_default().trim(),
            style_order.unwrap_or(255),
            filename
                .map(|v| format!("-{}", get_file_num_by_filename(v)))
                .unwrap_or_default(),
        );
        let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
        let filename = filename.map(|v| v.to_string()).unwrap_or_default();
        let clas_num = map
            .entry(filename.to_string())
            .or_default()
            .get(&key)
            .map(|v| num_to_nm_base(*v))
            .unwrap_or_else(|| {
                let m = map.entry(filename.to_string()).or_default();
                let len = m.len();
                m.insert(key, len);
                num_to_nm_base(len)
            });
        if !filename.is_empty() {
            format!(
                "{}-{}",
                num_to_nm_base(get_file_num_by_filename(&filename)),
                clas_num
            )
        } else {
            clas_num
        }
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
        map.entry("".to_string())
            .or_default()
            .get(&key)
            .map(|v| format!("--{}", num_to_nm_base(*v)))
            .unwrap_or_else(|| {
                let m = map.entry("".to_string()).or_default();
                let len = m.len();
                m.insert(key, len);
                format!("--{}", num_to_nm_base(len))
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
        assert_eq!(sheet_to_variable_name("background", 0, None), "--a");
        assert_eq!(
            sheet_to_variable_name("background", 0, Some("hover")),
            "--b"
        );
        assert_eq!(sheet_to_variable_name("background", 1, None), "--c");
        assert_eq!(
            sheet_to_variable_name("background", 1, Some("hover")),
            "--d"
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
            sheet_to_classname("background", 0, Some("red"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), Some("hover"), None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("background", 1, None, None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("background", 1, None, Some("hover"), None, None),
            "d"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None, None),
            sheet_to_classname("background", 0, None, None, None, None)
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None, None),
            sheet_to_classname("background", 0, Some("red"), None, None, None),
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None, None),
            sheet_to_classname("  background  ", 0, Some("  red  "), None, None, None),
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), None, None, None),
            sheet_to_classname("  background  ", 0, Some("red;"), None, None, None),
        );
        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("rgba(255, 0, 0,    0.5)"),
                None,
                None,
                None
            ),
            sheet_to_classname("background", 0, Some("rgba(255,0,0,0.5)"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("rgba(255, 0, 0,    0.5)"),
                None,
                None,
                None
            ),
            sheet_to_classname("background", 0, Some("rgba(255,0,0,.5)"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("rgba(255, 0, 0,    0.5)"),
                None,
                None,
                None
            ),
            sheet_to_classname("background", 0, Some("#FF000080"), None, None, None),
        );

        {
            let map = GLOBAL_CLASS_MAP.lock().unwrap();
            assert_eq!(
                map.get("").unwrap().get("background-0-#FF000080--255"),
                Some(&2)
            );
        }
        assert_eq!(
            sheet_to_classname("background", 0, Some("#fff"), None, None, None),
            sheet_to_classname("  background  ", 0, Some("#FFF"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffff"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFF"), None, None, None),
        );

        {
            let map = GLOBAL_CLASS_MAP.lock().unwrap();
            assert_eq!(map.get("").unwrap().get("background-0-#FFF--255"), Some(&3));
        }

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffff"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFFFFF"), None, None, None),
        );

        assert_eq!(
            sheet_to_classname("background", 0, Some("#ffffffAA"), None, None, None),
            sheet_to_classname("background", 0, Some("#FFFFFFaa"), None, None, None),
        );

        {
            let map = GLOBAL_CLASS_MAP.lock().unwrap();
            assert_eq!(
                map.get("").unwrap().get("background-0-#FFFA--255"),
                Some(&4)
            );
        }
        assert_eq!(
            sheet_to_classname(
                "background",
                0,
                Some("color-mix(in srgb,var(--primary) 80%,    #000 20%)"),
                None,
                None,
                None
            ),
            sheet_to_classname(
                "background",
                0,
                Some("color-mix(in srgb,    var(--primary) 80%, #000000 20%)"),
                None,
                None,
                None
            ),
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("background", 0, None, None, Some(1), None),
            "b"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname("width", 0, Some("0px"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0em"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0rem"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0vh"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0%"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0dvh"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0dvw"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0vw"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("width", 0, Some("0"), None, None, None),
            "a"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0px red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0% red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0em red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0rem red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0vh red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0vw red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0dvh red"), None, None, None),
            "b"
        );
        assert_eq!(
            sheet_to_classname("border", 0, Some("solid 0dvw red"), None, None, None),
            "b"
        );

        assert_eq!(
            sheet_to_classname("test", 0, Some("0px 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0em 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0rem 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0vh 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0vw 0"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0dvh 0"), None, None, None),
            "c"
        );

        assert_eq!(
            sheet_to_classname("test", 0, Some("0 0vh"), None, None, None),
            "c"
        );
        assert_eq!(
            sheet_to_classname("test", 0, Some("0 0vw"), None, None, None),
            "c"
        );

        reset_class_map();
        assert_eq!(
            sheet_to_classname(
                "transition",
                0,
                Some("all 0.3s ease-in-out"),
                None,
                None,
                None
            ),
            "a"
        );
        assert_eq!(
            sheet_to_classname(
                "transition",
                0,
                Some("all .3s ease-in-out"),
                None,
                None,
                None
            ),
            "a"
        );
    }

    #[test]
    #[serial]
    fn test_debug_sheet_to_classname() {
        set_debug(true);
        assert_eq!(
            sheet_to_classname("background", 0, None, None, None, None),
            "background-0---255"
        );
        assert_eq!(
            sheet_to_classname("background", 0, Some("red"), Some("hover"), None, None),
            "background-0-red-12448419602614487988-255"
        );
        assert_eq!(
            sheet_to_classname("background", 1, None, None, None, None),
            "background-1---255"
        );
        assert_eq!(
            sheet_to_classname("background", 1, Some("red"), Some("hover"), None, None),
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
        map.insert("".to_string(), HashMap::new());
        map.get_mut("")
            .unwrap()
            .insert("background-0-rgba(255,0,0,0.5)-".to_string(), 1);
        set_class_map(map);
        assert_eq!(get_class_map().len(), 1);
    }

    #[test]
    #[serial]
    fn test_keyframes_to_keyframes_name() {
        reset_class_map();
        set_debug(false);
        assert_eq!(keyframes_to_keyframes_name("spin", None), num_to_nm_base(0));
        assert_eq!(keyframes_to_keyframes_name("spin", None), num_to_nm_base(0));
        assert_eq!(
            keyframes_to_keyframes_name("spin2", None),
            num_to_nm_base(1)
        );
        reset_class_map();
        set_debug(true);
        assert_eq!(keyframes_to_keyframes_name("spin", None), "k-spin");
        assert_eq!(keyframes_to_keyframes_name("spin1", None), "k-spin1");
    }
}
