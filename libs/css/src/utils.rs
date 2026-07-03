#[inline]
#[must_use]
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
#[must_use]
pub fn to_camel_case(value: &str) -> String {
    // The split-based body below already yields the input verbatim for a
    // dash-free string (a single segment pushed as-is), so we skip the extra
    // `contains('-')` pre-scan and rebuild in one pass.
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

#[inline]
#[must_use]
pub(crate) fn collapse_whitespace(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for part in value.split_whitespace() {
        // Suppress the separating space immediately after a comma so that
        // `"a, b"` collapses to `"a,b"` (comma-delimited value lists stay tight);
        // do NOT "simplify" this `ends_with(',')` guard away.
        if !result.is_empty() && !result.ends_with(',') {
            result.push(' ');
        }
        result.push_str(part);
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

    #[rstest]
    #[case(" a   b ", "a b")]
    #[case("a, b", "a,b")]
    #[case("a , b", "a ,b")]
    #[case("a,  b   c", "a,b c")]
    fn test_collapse_whitespace(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(collapse_whitespace(input), expected);
    }
}
