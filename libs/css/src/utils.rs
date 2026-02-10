#[inline]
pub fn to_kebab_case(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 4);
    for (i, c) in value.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                result.push('-');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

#[inline]
pub fn to_camel_case(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for (i, s) in value.split('-').enumerate() {
        if i == 0 {
            result.push_str(s);
        } else if let Some(first) = s.chars().next() {
            result.push(first.to_ascii_uppercase());
            result.push_str(&s[first.len_utf8()..]);
        }
    }
    result
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
