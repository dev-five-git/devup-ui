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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("background-color", "backgroundColor")]
    #[case("min-width", "minWidth")]
    #[case("max-height", "maxHeight")]
    #[case("border-radius", "borderRadius")]
    #[case("color", "color")]
    fn test_to_camel_case(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(to_camel_case(input), expected);
    }

    #[rstest]
    #[case("backgroundColor", "background-color")]
    #[case("minWidth", "min-width")]
    #[case("maxHeight", "max-height")]
    #[case("borderRadius", "border-radius")]
    #[case("color", "color")]
    fn test_to_kebab_case(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(to_kebab_case(input), expected);
    }
}
