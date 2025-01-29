use crate::StyleSelector::{Dual, Postfix, Prefix};
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::sync::Mutex;

static SELECTOR_ORDER_MAP: Lazy<HashMap<String, u8>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("disabled".to_string(), 5);
    map.insert("selected".to_string(), 4);
    map.insert("active".to_string(), 3);
    map.insert("focus".to_string(), 2);
    map.insert("hover".to_string(), 1);
    map
});

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub enum StyleSelector {
    Postfix(String),
    Prefix(String),
    Dual(String, String),
}

impl PartialOrd for StyleSelector {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StyleSelector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Postfix(a), Postfix(b)) => SELECTOR_ORDER_MAP
                .get(a)
                .unwrap_or(&0)
                .cmp(SELECTOR_ORDER_MAP.get(b).unwrap_or(&0)),
            (Prefix(a), Prefix(b)) => a.cmp(b),
            (Dual(p1, a), Dual(p2, b)) => {
                if p1 == p2 {
                    SELECTOR_ORDER_MAP
                        .get(a)
                        .unwrap_or(&0)
                        .cmp(SELECTOR_ORDER_MAP.get(b).unwrap_or(&0))
                } else {
                    p1.cmp(p2)
                }
            }
            (Postfix(_), _) => std::cmp::Ordering::Less,
            (Prefix(_), Postfix(_)) => std::cmp::Ordering::Greater,
            (Prefix(_), _) => std::cmp::Ordering::Less,
            (Dual(_, _), Postfix(_)) => std::cmp::Ordering::Greater,
            (Dual(_, _), Prefix(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl From<&str> for StyleSelector {
    fn from(value: &str) -> Self {
        if let Some(s) = value.strip_prefix("group") {
            Dual("*[role=group]".to_string(), to_kebab_case(s))
        } else if let Some(s) = value.strip_prefix("theme") {
            // first character should lower case
            Prefix(format!(
                ":root[data-theme={}{}]",
                s.chars().next().unwrap().to_ascii_lowercase(),
                &s[1..]
            ))
        } else {
            Postfix(to_kebab_case(value))
        }
    }
}

impl Display for StyleSelector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Postfix(value) => format!("-{}", value),
                Prefix(value) => format!("-{}-", value),
                Dual(prefix, postfix) => format!("-{}-{}", prefix, postfix),
            }
        )
    }
}

pub fn merge_selector(class_name: &str, selector: Option<&StyleSelector>) -> String {
    if let Some(selector) = selector {
        match selector {
            Postfix(postfix) => match get_selector_separator(postfix) {
                SelectorSeparator::Single => format!(".{}:{}", class_name, postfix),
                SelectorSeparator::Double => format!(".{}::{}", class_name, postfix),
            },
            Prefix(prefix) => format!("{} .{}", prefix, class_name),
            Dual(prefix, postfix) => match get_selector_separator(postfix) {
                SelectorSeparator::Single => format!("{}:{} .{}", prefix, postfix, class_name),
                SelectorSeparator::Double => format!("{}::{} .{}", prefix, postfix, class_name),
            },
        }
    } else {
        format!(".{}", class_name)
    }
}

pub enum SelectorSeparator {
    Single,
    Double,
}

static DOUBLE_SEPARATOR: Lazy<HashSet<&str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    for key in [
        "placeholder",
        "before",
        "after",
        "highlight",
        "view-transition",
        "view-transition-group",
        "view-transition-image-pair",
        "view-transition-new",
        "view-transition-old",
    ] {
        set.insert(key);
    }
    set
});

pub fn get_selector_separator(key: &str) -> SelectorSeparator {
    if DOUBLE_SEPARATOR.contains(key) {
        SelectorSeparator::Double
    } else {
        SelectorSeparator::Single
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyType {
    Single(String),
    Multi(Vec<String>),
}

impl From<&str> for PropertyType {
    fn from(value: &str) -> Self {
        value.to_string().into()
    }
}

impl From<String> for PropertyType {
    fn from(value: String) -> Self {
        PropertyType::Single(value)
    }
}

impl From<[&str; 2]> for PropertyType {
    fn from(value: [&str; 2]) -> Self {
        PropertyType::Multi(value.iter().map(|v| v.to_string()).collect())
    }
}

static GLOBAL_STYLE_PROPERTY: Lazy<HashMap<&str, PropertyType>> = Lazy::new(|| {
    let mut map = HashMap::new();

    for (key, value) in [
        ("bg", "background"),
        ("bgAttachment", "background-attachment"),
        ("bgClip", "background-clip"),
        ("bgColor", "background-color"),
        ("bgImage", "background-image"),
        ("bgOrigin", "background-origin"),
        ("bgPosition", "background-position"),
        ("bgPositionX", "background-position-x"),
        ("bgPositionY", "background-position-y"),
        ("bgRepeat", "background-repeat"),
        ("bgSize", "background-size"),
        ("animationDir", "animation-direction"),
        ("flexDir", "flex-direction"),
        ("pos", "position"),
        ("m", "margin"),
        ("mt", "margin-top"),
        ("mr", "margin-right"),
        ("mb", "margin-bottom"),
        ("ml", "margin-left"),
        ("p", "padding"),
        ("pt", "padding-top"),
        ("pr", "padding-right"),
        ("pb", "padding-bottom"),
        ("pl", "padding-left"),
        ("w", "width"),
        ("h", "height"),
        ("minW", "min-width"),
        ("minH", "min-height"),
        ("maxW", "max-width"),
        ("maxH", "max-height"),
    ] {
        map.insert(key, value.into());
    }

    for (key, value) in [
        ("mx", ["margin-left", "margin-right"]),
        ("my", ["margin-top", "margin-bottom"]),
        ("px", ["padding-left", "padding-right"]),
        ("py", ["padding-top", "padding-bottom"]),
        ("boxSize", ["width", "height"]),
    ] {
        map.insert(key, value.into());
    }
    map
});

static GLOBAL_CLASS_MAP: Lazy<Mutex<HashMap<String, i32>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// for test
pub fn reset_class_map() {
    let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
    map.clear();
}

pub fn to_kebab_case(value: &str) -> String {
    value
        .chars()
        .enumerate()
        .map(|(i, c)| {
            if c.is_uppercase() {
                if i == 0 {
                    c.to_ascii_lowercase().to_string()
                } else {
                    format!("-{}", c.to_ascii_lowercase())
                }
            } else {
                c.to_string()
            }
        })
        .collect()
}

pub fn to_camel_case(value: &str) -> String {
    value
        .split('-')
        .enumerate()
        .map(|(i, s)| {
            if i == 0 {
                s.to_string()
            } else {
                format!("{}{}", s[0..1].to_uppercase(), &s[1..])
            }
        })
        .collect()
}

pub fn convert_property(property: &str) -> PropertyType {
    GLOBAL_STYLE_PROPERTY
        .get(property)
        .cloned()
        .unwrap_or_else(|| to_kebab_case(property).into())
}

pub fn sort_to_long(property: &str) -> String {
    GLOBAL_STYLE_PROPERTY
        .get(property)
        .map(|v| match v {
            PropertyType::Single(value) => to_camel_case(value),
            PropertyType::Multi(_) => property.to_string(),
        })
        .unwrap_or_else(|| property.to_string())
}

pub fn sheet_to_classname(
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: Option<&str>,
) -> String {
    let key = format!(
        "{}-{}-{}-{}",
        property,
        level,
        value.unwrap_or(""),
        selector.unwrap_or("")
    );
    let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
    map.get(&key).map(|v| format!("d{}", v)).unwrap_or_else(|| {
        let len = map.len();
        map.insert(key, len as i32);
        format!("d{}", map.len() - 1)
    })
}

pub fn css_to_classname(css: &str) -> String {
    let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
    map.get(css).map(|v| format!("d{}", v)).unwrap_or_else(|| {
        let len = map.len();
        map.insert(css.to_string(), len as i32);
        format!("d{}", map.len() - 1)
    })
}

pub fn sheet_to_variable_name(property: &str, level: u8, selector: Option<&str>) -> String {
    let key = format!("{}-{}-{}", property, level, selector.unwrap_or(""));
    let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
    map.get(&key)
        .map(|v| format!("--d{}", v))
        .unwrap_or_else(|| {
            let len = map.len();
            map.insert(key, len as i32);
            format!("--d{}", map.len() - 1)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_sheet_to_variable_name() {
        reset_class_map();
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
    fn test_sheet_to_classname() {
        reset_class_map();
        assert_eq!(sheet_to_classname("background", 0, None, None), "d0");
        assert_eq!(
            sheet_to_classname("background", 0, Some("hover"), None),
            "d1"
        );
        assert_eq!(sheet_to_classname("background", 1, None, None), "d2");
        assert_eq!(
            sheet_to_classname("background", 1, Some("hover"), None),
            "d3"
        );
    }

    #[test]
    fn test_convert_property() {
        assert_eq!(
            convert_property("bg"),
            PropertyType::Single("background".to_string())
        );
        assert_eq!(
            convert_property("bgColor"),
            PropertyType::Single("background-color".to_string())
        );
        assert_eq!(
            convert_property("color"),
            PropertyType::Single("color".to_string())
        );
        assert_eq!(
            convert_property("m"),
            PropertyType::Single("margin".to_string())
        );
        assert_eq!(
            convert_property("mt"),
            PropertyType::Single("margin-top".to_string())
        );
        assert_eq!(
            convert_property("mr"),
            PropertyType::Single("margin-right".to_string())
        );
        assert_eq!(
            convert_property("mb"),
            PropertyType::Single("margin-bottom".to_string())
        );
        assert_eq!(
            convert_property("ml"),
            PropertyType::Single("margin-left".to_string())
        );
        assert_eq!(
            convert_property("p"),
            PropertyType::Single("padding".to_string())
        );
        assert_eq!(
            convert_property("pt"),
            PropertyType::Single("padding-top".to_string())
        );
        assert_eq!(
            convert_property("pr"),
            PropertyType::Single("padding-right".to_string())
        );
        assert_eq!(
            convert_property("pb"),
            PropertyType::Single("padding-bottom".to_string())
        );
        assert_eq!(
            convert_property("pl"),
            PropertyType::Single("padding-left".to_string())
        );
        assert_eq!(
            convert_property("w"),
            PropertyType::Single("width".to_string())
        );
        assert_eq!(
            convert_property("h"),
            PropertyType::Single("height".to_string())
        );
        assert_eq!(
            convert_property("minW"),
            PropertyType::Single("min-width".to_string())
        );
        assert_eq!(
            convert_property("minH"),
            PropertyType::Single("min-height".to_string())
        );
        assert_eq!(
            convert_property("maxW"),
            PropertyType::Single("max-width".to_string())
        );
        assert_eq!(
            convert_property("maxH"),
            PropertyType::Single("max-height".to_string())
        );
        assert_eq!(
            convert_property("mx"),
            PropertyType::Multi(vec!["margin-left".to_string(), "margin-right".to_string()])
        );
        assert_eq!(
            convert_property("my"),
            PropertyType::Multi(vec!["margin-top".to_string(), "margin-bottom".to_string()])
        );
        assert_eq!(
            convert_property("px"),
            PropertyType::Multi(vec![
                "padding-left".to_string(),
                "padding-right".to_string()
            ])
        );
        assert_eq!(
            convert_property("py"),
            PropertyType::Multi(vec![
                "padding-top".to_string(),
                "padding-bottom".to_string()
            ])
        );
    }

    #[test]
    fn test_property_type_from() {
        assert_eq!(
            PropertyType::from("background"),
            PropertyType::Single("background".to_string())
        );
        assert_eq!(
            PropertyType::from("background-color"),
            PropertyType::Single("background-color".to_string())
        );
        assert_eq!(
            PropertyType::from("color"),
            PropertyType::Single("color".to_string())
        );
        assert_eq!(
            PropertyType::from("margin"),
            PropertyType::Single("margin".to_string())
        );
        assert_eq!(
            PropertyType::from("margin-top"),
            PropertyType::Single("margin-top".to_string())
        );
        assert_eq!(
            PropertyType::from("margin-right"),
            PropertyType::Single("margin-right".to_string())
        );
        assert_eq!(
            PropertyType::from("margin-bottom"),
            PropertyType::Single("margin-bottom".to_string())
        );
        assert_eq!(
            PropertyType::from("margin-left"),
            PropertyType::Single("margin-left".to_string())
        );
        assert_eq!(
            PropertyType::from("padding"),
            PropertyType::Single("padding".to_string())
        );
        assert_eq!(
            PropertyType::from("padding-top"),
            PropertyType::Single("padding-top".to_string())
        );
        assert_eq!(
            PropertyType::from("padding-right"),
            PropertyType::Single("padding-right".to_string())
        );
        assert_eq!(
            PropertyType::from("padding-bottom"),
            PropertyType::Single("padding-bottom".to_string())
        );
        assert_eq!(
            PropertyType::from("padding-left"),
            PropertyType::Single("padding-left".to_string())
        );
        assert_eq!(
            PropertyType::from("width"),
            PropertyType::Single("width".to_string())
        );
        assert_eq!(
            PropertyType::from("height"),
            PropertyType::Single("height".to_string())
        );
        assert_eq!(
            PropertyType::from("min-width"),
            PropertyType::Single("min-width".to_string())
        );
        assert_eq!(
            PropertyType::from("min-height"),
            PropertyType::Single("min-height".to_string())
        );
        assert_eq!(
            PropertyType::from("max-width"),
            PropertyType::Single("max-width".to_string())
        );
        assert_eq!(
            PropertyType::from("max-height"),
            PropertyType::Single("max-height".to_string())
        );
        assert_eq!(
            PropertyType::from(["margin-left", "margin-right"]),
            PropertyType::Multi(vec!["margin-left".to_string(), "margin-right".to_string()])
        );
        assert_eq!(
            PropertyType::from(["margin-top", "margin-bottom"]),
            PropertyType::Multi(vec!["margin-top".to_string(), "margin-bottom".to_string()])
        );
    }

    #[test]
    fn test_kebab_case() {
        assert_eq!(to_kebab_case("backgroundColor"), "background-color");
        assert_eq!(to_kebab_case("color"), "color");
        assert_eq!(to_kebab_case("margin"), "margin");
        assert_eq!(to_kebab_case("marginTop"), "margin-top");
        assert_eq!(to_kebab_case("marginRight"), "margin-right");
        assert_eq!(to_kebab_case("marginBottom"), "margin-bottom");
        assert_eq!(to_kebab_case("marginLeft"), "margin-left");
        assert_eq!(to_kebab_case("padding"), "padding");
        assert_eq!(to_kebab_case("paddingTop"), "padding-top");
        assert_eq!(to_kebab_case("paddingRight"), "padding-right");
        assert_eq!(to_kebab_case("paddingBottom"), "padding-bottom");
        assert_eq!(to_kebab_case("paddingLeft"), "padding-left");
        assert_eq!(to_kebab_case("width"), "width");
        assert_eq!(to_kebab_case("height"), "height");
        assert_eq!(to_kebab_case("minWidth"), "min-width");
        assert_eq!(to_kebab_case("minHeight"), "min-height");
        assert_eq!(to_kebab_case("maxWidth"), "max-width");
        assert_eq!(to_kebab_case("maxHeight"), "max-height");
        assert_eq!(to_kebab_case("MaxHeight"), "max-height");
        assert_eq!(to_kebab_case("Hover"), "hover");
    }

    #[test]
    fn test_style_selector() {
        assert_eq!(StyleSelector::from("hover"), Postfix("hover".to_string()));
        assert_eq!(
            StyleSelector::from("focusVisible"),
            Postfix("focus-visible".to_string())
        );
        assert_eq!(
            StyleSelector::from("groupHover"),
            Dual("*[role=group]".to_string(), "hover".to_string())
        );
        assert_eq!(
            StyleSelector::from("groupFocusVisible"),
            Dual("*[role=group]".to_string(), "focus-visible".to_string())
        );
        assert_eq!(
            StyleSelector::from("group1"),
            Dual("*[role=group]".to_string(), "1".to_string())
        );

        assert_eq!(Prefix(".cls".to_string()).to_string(), "-.cls-");
        assert_eq!(Postfix(".cls".to_string()).to_string(), "-.cls");

        assert_eq!(
            StyleSelector::from("themeLight"),
            Prefix(":root[data-theme=light]".to_string())
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
            merge_selector("cls", Some(&"themeDark".into())),
            ":root[data-theme=dark] .cls"
        );
        assert_eq!(
            merge_selector(
                "cls",
                Some(&Dual(
                    ":root[data-theme=dark]".to_string(),
                    "hover".to_string()
                )),
            ),
            ":root[data-theme=dark]:hover .cls"
        );
        assert_eq!(
            merge_selector(
                "cls",
                Some(&Dual(
                    ":root[data-theme=dark]".to_string(),
                    "placeholder".to_string()
                )),
            ),
            ":root[data-theme=dark]::placeholder .cls"
        );
    }
}
