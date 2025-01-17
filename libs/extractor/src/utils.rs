use once_cell::sync::Lazy;
use std::collections::HashSet;

/// Convert a value to a pixel value
pub fn convert_value(value: &str) -> String {
    let value = value.to_string();
    if let Ok(num) = value.parse::<f64>() {
        let num = num * 4.0;
        return format!("{}px", num);
    }
    value
}

static SPECIAL_PROPERTIES: Lazy<HashSet<&str>> = Lazy::new(|| {
    let mut set = HashSet::<&str>::new();
    for prop in [
        "style",
        "className",
        "role",
        "ref",
        "key",
        "alt",
        "src",
        "children",
        "placeholder",
        "maxLength",
        "minLength",
    ] {
        set.insert(prop);
    }
    set
});

pub fn is_special_property(name: &str) -> bool {
    name.starts_with("on")
        || name.starts_with("data-")
        || name.starts_with("aria-")
        || SPECIAL_PROPERTIES.contains(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_value() {
        assert_eq!(convert_value("1px"), "1px");
        assert_eq!(convert_value("1%"), "1%");
        assert_eq!(convert_value("foo"), "foo");
        assert_eq!(convert_value("4"), "16px");
    }
}
