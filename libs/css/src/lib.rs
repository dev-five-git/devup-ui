use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;

static SELECTOR_ORDER_MAP: Lazy<HashMap<String, u8>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for (idx, selector) in [
        "hover",
        "focus-visible",
        "focus",
        "active",
        "selected",
        "disabled",
    ]
    .into_iter()
    .enumerate()
    {
        map.insert(format!("&:{}", selector), idx as u8);
    }
    map
});

static DEBUG: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub fn set_debug(value: bool) {
    let mut debug = DEBUG.lock().unwrap();
    *debug = value;
}

pub fn is_debug() -> bool {
    *DEBUG.lock().unwrap()
}

#[derive(Debug, PartialEq, Clone, Hash, Eq, Serialize, Deserialize)]
pub enum StyleSelector {
    Media(String),
    Selector(String),
}

impl PartialOrd for StyleSelector {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn get_selector_order(selector: &str) -> u8 {
    // & count
    let t = if selector.chars().filter(|c| c == &'&').count() == 1 {
        selector
            .split('&')
            .last()
            .map(|a| format!("&{}", a))
            .unwrap_or(selector.to_string())
    } else {
        selector.to_string()
    };

    *SELECTOR_ORDER_MAP
        .get(&t)
        .unwrap_or(if t.starts_with("&") { &0 } else { &99 })
}

impl Ord for StyleSelector {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (StyleSelector::Media(a), StyleSelector::Media(b)) => a.cmp(b),
            (StyleSelector::Selector(a), StyleSelector::Selector(b)) => {
                get_selector_order(a).cmp(&get_selector_order(b))
            }
            (StyleSelector::Media(_), StyleSelector::Selector(_)) => Ordering::Greater,
            (StyleSelector::Selector(_), StyleSelector::Media(_)) => Ordering::Less,
        }
    }
}

impl From<&str> for StyleSelector {
    fn from(value: &str) -> Self {
        if value.contains("&") {
            StyleSelector::Selector(value.to_string())
        } else if let Some(s) = value.strip_prefix("group") {
            let post = to_kebab_case(s);
            StyleSelector::Selector(format!(
                "{}{}{} &",
                "*[role=group]",
                get_selector_separator(&post),
                post
            ))
        } else if let Some(s) = value.strip_prefix("theme") {
            // first character should lower case
            StyleSelector::Selector(format!(
                ":root[data-theme={}{}] &",
                s.chars().next().unwrap().to_ascii_lowercase(),
                &s[1..]
            ))
        } else if value == "print" {
            StyleSelector::Media("print".to_string())
        } else if value.ends_with(" ") {
            StyleSelector::Selector(format!("{} &", value.trim()))
        } else {
            let post = to_kebab_case(value);

            StyleSelector::Selector(format!("&{}{}", get_selector_separator(&post), post))
        }
    }
}

impl From<[&str; 2]> for StyleSelector {
    fn from(value: [&str; 2]) -> Self {
        let post = to_kebab_case(value[1]);
        StyleSelector::Selector(format!(
            "{}{}{}",
            StyleSelector::from(value[0]),
            get_selector_separator(&post),
            post
        ))
    }
}

impl Display for StyleSelector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                StyleSelector::Selector(value) => value.to_string(),
                StyleSelector::Media(value) => format!("@{}", value),
            }
        )
    }
}

pub fn merge_selector(class_name: &str, selector: Option<&StyleSelector>) -> String {
    if let Some(selector) = selector {
        match selector {
            StyleSelector::Selector(value) => value.replace("&", &format!(".{}", class_name)),
            StyleSelector::Media(_) => format!(".{}", class_name),
        }
    } else {
        format!(".{}", class_name)
    }
}

pub enum SelectorSeparator {
    Single,
    Double,
    None,
}
impl Display for SelectorSeparator {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                SelectorSeparator::Single => ":",
                SelectorSeparator::Double => "::",
                SelectorSeparator::None => "",
            }
        )
    }
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
    if key.starts_with(":") || key.is_empty() || key.starts_with("[") {
        SelectorSeparator::None
    } else if DOUBLE_SEPARATOR.contains(key) {
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
        (
            "borderBottomRadius",
            ["border-bottom-left-radius", "border-bottom-right-radius"],
        ),
        (
            "borderTopRadius",
            ["border-top-left-radius", "border-top-right-radius"],
        ),
        (
            "borderLeftRadius",
            ["border-top-left-radius", "border-bottom-left-radius"],
        ),
        (
            "borderRightRadius",
            ["border-top-right-radius", "border-bottom-right-radius"],
        ),
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

pub fn set_class_map(map: HashMap<String, i32>) {
    let mut global_map = GLOBAL_CLASS_MAP.lock().unwrap();
    *global_map = map;
}

pub fn get_class_map() -> HashMap<String, i32> {
    GLOBAL_CLASS_MAP.lock().unwrap().clone()
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

pub fn short_to_long(property: &str) -> String {
    GLOBAL_STYLE_PROPERTY
        .get(property)
        .map(|v| match v {
            PropertyType::Single(value) => to_camel_case(value),
            PropertyType::Multi(_) => property.to_string(),
        })
        .unwrap_or_else(|| property.to_string())
}

static F_SPACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s*,\s*").unwrap());
static COLOR_HASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"#([0-9a-zA-Z]+)").unwrap());

fn optimize_color(value: &str) -> String {
    let mut ret = value.to_string().to_uppercase();

    if ret.len() == 6 {
        let ch = ret.chars().collect::<Vec<char>>();
        if ch[0] == ch[1] && ch[2] == ch[3] && ch[4] == ch[5] {
            ret = format!("{}{}{}", ch[0], ch[2], ch[4]);
        }
    } else if ret.len() == 8 {
        let ch = ret.chars().collect::<Vec<char>>();
        if ch[0] == ch[1] && ch[2] == ch[3] && ch[4] == ch[5] && ch[6] == ch[7] {
            ret = format!("{}{}{}{}", ch[0], ch[2], ch[4], ch[6]);
        }
    }

    format!("#{}", ret)
}

fn optimize_value(value: &str) -> String {
    let mut ret = value.trim().to_string();
    if ret.contains(",") {
        ret = F_SPACE_RE.replace_all(&ret, ",").trim().to_string();
    }
    if ret.contains("#") {
        ret = COLOR_HASH
            .replace_all(&ret, |c: &regex::Captures| optimize_color(&c[1]))
            .to_string();
    }
    ret
}

pub fn sheet_to_classname(
    property: &str,
    level: u8,
    value: Option<&str>,
    selector: Option<&str>,
    style_order: Option<u8>,
) -> String {
    if *DEBUG.lock().unwrap() {
        let selector = selector.unwrap_or("").trim();
        format!(
            "{}-{}-{}-{}-{}",
            property.trim(),
            level,
            optimize_value(value.unwrap_or("")),
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
            optimize_value(value.unwrap_or("")),
            selector.unwrap_or("").trim(),
            style_order.unwrap_or(255)
        );
        let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
        map.get(&key).map(|v| format!("d{}", v)).unwrap_or_else(|| {
            let len = map.len();
            map.insert(key, len as i32);
            format!("d{}", map.len() - 1)
        })
    }
}

pub fn css_to_classname(css: &str) -> String {
    if *DEBUG.lock().unwrap() {
        let mut hasher = DefaultHasher::new();
        css.hash(&mut hasher);
        format!("css-{}", hasher.finish())
    } else {
        let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
        map.get(css).map(|v| format!("d{}", v)).unwrap_or_else(|| {
            let len = map.len();
            map.insert(css.to_string(), len as i32);
            format!("d{}", map.len() - 1)
        })
    }
}

pub fn sheet_to_variable_name(property: &str, level: u8, selector: Option<&str>) -> String {
    if *DEBUG.lock().unwrap() {
        let selector = selector.unwrap_or("").trim();
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
        let key = format!("{}-{}-{}", property, level, selector.unwrap_or("").trim());
        let mut map = GLOBAL_CLASS_MAP.lock().unwrap();
        map.get(&key)
            .map(|v| format!("--d{}", v))
            .unwrap_or_else(|| {
                let len = map.len();
                map.insert(key, len as i32);
                format!("--d{}", map.len() - 1)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_set_debug() {
        set_debug(true);
        assert!(is_debug());
        set_debug(false);
        assert!(!is_debug());
    }

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
            sheet_to_classname("background", 0, Some("rgba(255, 0, 0,    0.5)"), None, None),
            sheet_to_classname("background", 0, Some("rgba(255,0,0,0.5)"), None, None),
        );

        {
            let map = GLOBAL_CLASS_MAP.lock().unwrap();
            assert_eq!(map.get("background-0-rgba(255,0,0,0.5)--255"), Some(&2));
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
    #[serial]
    fn test_css_to_classname() {
        set_debug(false);
        reset_class_map();
        assert_eq!(css_to_classname("background: red"), "d0");
        assert_eq!(css_to_classname("background: blue"), "d1");
    }
    #[test]
    #[serial]
    fn test_debug_css_to_classname() {
        set_debug(true);
        assert_eq!(
            css_to_classname("background: red"),
            "css-10773204219957113694"
        );
        assert_eq!(
            css_to_classname("background: blue"),
            "css-1226995032436176700"
        );
        set_debug(true);
        reset_class_map();
        assert_eq!(
            css_to_classname("background: red"),
            "css-10773204219957113694"
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
        assert_eq!(
            StyleSelector::from("hover"),
            StyleSelector::Selector("&:hover".to_string())
        );
        assert_eq!(
            StyleSelector::from("focusVisible"),
            StyleSelector::Selector("&:focus-visible".to_string())
        );
        assert_eq!(
            StyleSelector::from("groupHover"),
            StyleSelector::Selector("*[role=group]:hover &".to_string())
        );
        assert_eq!(
            StyleSelector::from("groupFocusVisible"),
            StyleSelector::Selector("*[role=group]:focus-visible &".to_string())
        );
        assert_eq!(
            StyleSelector::from("group1"),
            StyleSelector::Selector("*[role=group]:1 &".to_string())
        );

        assert_eq!(
            StyleSelector::from(["themeDark", "placeholder"]),
            StyleSelector::Selector(":root[data-theme=dark] &::placeholder".to_string())
        );

        assert_eq!(
            StyleSelector::from("themeLight"),
            StyleSelector::Selector(":root[data-theme=light] &".to_string())
        );

        assert_eq!(
            StyleSelector::from("*[aria=disabled='true'] &:hover"),
            StyleSelector::Selector("*[aria=disabled='true'] &:hover".to_string())
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
            merge_selector("cls", Some(&["themeDark", "hover"].into()),),
            ":root[data-theme=dark] .cls:hover"
        );
    }
    #[test]
    fn test_get_selector_separator() {
        assert!(matches!(
            get_selector_separator("placeholder"),
            SelectorSeparator::Double
        ));
        assert!(matches!(
            get_selector_separator("before"),
            SelectorSeparator::Double
        ));
        assert!(matches!(
            get_selector_separator("after"),
            SelectorSeparator::Double
        ));

        assert!(matches!(
            get_selector_separator("hover"),
            SelectorSeparator::Single
        ));

        assert!(matches!(
            get_selector_separator(":hover"),
            SelectorSeparator::None
        ));

        assert!(matches!(
            get_selector_separator("::placeholder"),
            SelectorSeparator::None
        ));

        assert!(matches!(
            get_selector_separator("[aria-disabled='true']"),
            SelectorSeparator::None
        ));
    }

    #[test]
    #[serial]
    fn test_set_class_map() {
        let mut map = HashMap::new();
        map.insert("background-0-rgba(255,0,0,0.5)-".to_string(), 1);
        set_class_map(map);
        assert_eq!(get_class_map().len(), 1);
    }
}
