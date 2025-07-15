use crate::{constant::GLOBAL_STYLE_PROPERTY, utils::to_kebab_case};

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyType {
    Single(String),
    Multi(Vec<String>),
}

impl From<&str> for PropertyType {
    fn from(value: &str) -> Self {
        GLOBAL_STYLE_PROPERTY
            .get(value)
            .cloned()
            .map(|v| match v.len() {
                1 => PropertyType::Single(v[0].to_string()),
                _ => PropertyType::Multi(v.iter().map(|v| v.to_string()).collect()),
            })
            .unwrap_or_else(|| {
                if (value.starts_with("Webkit")
                    && value.len() > 6
                    && value.chars().nth(6).unwrap().is_uppercase())
                    || (value.starts_with("Moz")
                        && value.len() > 3
                        && value.chars().nth(3).unwrap().is_uppercase())
                    || (value.starts_with("ms")
                        && value.len() > 2
                        && value.chars().nth(2).unwrap().is_uppercase())
                {
                    PropertyType::Single(format!("-{}", to_kebab_case(value)))
                } else {
                    PropertyType::Single(to_kebab_case(value))
                }
            })
    }
}

impl From<[&str; 2]> for PropertyType {
    fn from(value: [&str; 2]) -> Self {
        PropertyType::Multi(value.iter().map(|v| v.to_string()).collect())
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn test_convert_property() {
        assert_eq!(
            PropertyType::from("bg"),
            PropertyType::Single("background".to_string())
        );
        assert_eq!(
            PropertyType::from("bgColor"),
            PropertyType::Single("background-color".to_string())
        );
        assert_eq!(
            PropertyType::from("color"),
            PropertyType::Single("color".to_string())
        );
        assert_eq!(
            PropertyType::from("m"),
            PropertyType::Single("margin".to_string())
        );
        assert_eq!(
            PropertyType::from("mt"),
            PropertyType::Single("margin-top".to_string())
        );
        assert_eq!(
            PropertyType::from("mr"),
            PropertyType::Single("margin-right".to_string())
        );
        assert_eq!(
            PropertyType::from("mb"),
            PropertyType::Single("margin-bottom".to_string())
        );
        assert_eq!(
            PropertyType::from("ml"),
            PropertyType::Single("margin-left".to_string())
        );
        assert_eq!(
            PropertyType::from("p"),
            PropertyType::Single("padding".to_string())
        );
        assert_eq!(
            PropertyType::from("pt"),
            PropertyType::Single("padding-top".to_string())
        );
        assert_eq!(
            PropertyType::from("pr"),
            PropertyType::Single("padding-right".to_string())
        );
        assert_eq!(
            PropertyType::from("pb"),
            PropertyType::Single("padding-bottom".to_string())
        );
        assert_eq!(
            PropertyType::from("pl"),
            PropertyType::Single("padding-left".to_string())
        );
        assert_eq!(
            PropertyType::from("w"),
            PropertyType::Single("width".to_string())
        );
        assert_eq!(
            PropertyType::from("h"),
            PropertyType::Single("height".to_string())
        );
        assert_eq!(
            PropertyType::from("minW"),
            PropertyType::Single("min-width".to_string())
        );
        assert_eq!(
            PropertyType::from("minH"),
            PropertyType::Single("min-height".to_string())
        );
        assert_eq!(
            PropertyType::from("maxW"),
            PropertyType::Single("max-width".to_string())
        );
        assert_eq!(
            PropertyType::from("maxH"),
            PropertyType::Single("max-height".to_string())
        );
        assert_eq!(
            PropertyType::from("mx"),
            PropertyType::Multi(vec!["margin-left".to_string(), "margin-right".to_string()])
        );
        assert_eq!(
            PropertyType::from("my"),
            PropertyType::Multi(vec!["margin-top".to_string(), "margin-bottom".to_string()])
        );
        assert_eq!(
            PropertyType::from("px"),
            PropertyType::Multi(vec![
                "padding-left".to_string(),
                "padding-right".to_string()
            ])
        );
        assert_eq!(
            PropertyType::from("py"),
            PropertyType::Multi(vec![
                "padding-top".to_string(),
                "padding-bottom".to_string()
            ])
        );
    }

    #[test]
    fn test_convert_vendor_property() {
        assert_eq!(
            PropertyType::from("MozUserSelect"),
            PropertyType::Single("-moz-user-select".to_string())
        );
        assert_eq!(
            PropertyType::from("msAccelerator"),
            PropertyType::Single("-ms-accelerator".to_string())
        );
        assert_eq!(
            PropertyType::from("WebkitAlignContent"),
            PropertyType::Single("-webkit-align-content".to_string())
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
}
