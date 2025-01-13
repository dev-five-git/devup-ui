use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};
use std::sync::Mutex;

#[derive(Clone)]
pub enum PropertyType {
    Single(String),
    Multi(Vec<String>),
}

impl From<&str> for PropertyType {
    fn from(value: &str) -> Self {
        PropertyType::Single(value.to_string())
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

static GLOBAL_STYLE_PROPERTY: Lazy<Mutex<HashMap<&str, PropertyType>>> = Lazy::new(|| {
    Mutex::new({
        let mut map = HashMap::new();

        for (key, value) in [
            ("bg", "background"),
            ("bgColor", "background-color"),
            ("color", "color"),
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
        ] {
            map.insert(key, value.into());
        }
        map
    })
});
fn to_kebab_case(value: &str) -> String {
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

pub fn convert_property(property: &str) -> PropertyType {
    GLOBAL_STYLE_PROPERTY
        .lock()
        .unwrap()
        .get(property)
        .cloned()
        .unwrap_or_else(|| to_kebab_case(property).into())
}

pub fn sheet_to_classname(property: &str, level: u8, value: Option<&str>) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write(format!("{}-{}-{}", property, level, value.unwrap_or("")).as_bytes());
    format!("d{}", hasher.finish())
}

pub fn sheet_to_variable_name(property: &str, level: u8) -> String {
    let mut hasher = DefaultHasher::new();
    hasher.write(format!("{}-{}", property, level).as_bytes());
    format!("--d{}", hasher.finish())
}
