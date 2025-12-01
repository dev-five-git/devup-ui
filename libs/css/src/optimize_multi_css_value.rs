use crate::constant::{CHECK_QUOTES_RE, CSS_FUNCTION_RE, OPTIMIZE_MULTI_CSS_VALUE_PROPERTY};

pub fn optimize_mutli_css_value(value: &str) -> String {
    value
        .split(",")
        .map(|s| {
            let s = s.trim();
            let s = if s.starts_with("'") && s.ends_with("'")
                || s.starts_with('"') && s.ends_with('"')
            {
                s[1..s.len() - 1].to_string()
            } else {
                s.to_string()
            };
            if CHECK_QUOTES_RE.is_match(&s) && !CSS_FUNCTION_RE.is_match(&s) {
                format!("\"{s}\"")
            } else {
                s
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

pub fn wrap_url(s: &str) -> String {
    if CSS_FUNCTION_RE.is_match(s) {
        s.to_string()
    } else {
        format!("url({s})")
    }
}

pub fn check_multi_css_optimize(property: &str) -> bool {
    OPTIMIZE_MULTI_CSS_VALUE_PROPERTY.contains(property)
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case("Roboto, sans-serif", "Roboto,sans-serif")]
    #[case("'Roboto', sans-serif", "Roboto,sans-serif")]
    #[case("\"Roboto\", sans-serif", "Roboto,sans-serif")]
    #[case("'Roboto Hello', sans-serif", "\"Roboto Hello\",sans-serif")]
    #[case("\"Roboto Hello\", sans-serif", "\"Roboto Hello\",sans-serif")]
    #[case("Roboto", "Roboto")]
    #[case("'Roboto'", "Roboto")]
    #[case("\"Roboto\"", "Roboto")]
    #[case("url('/fonts/Roboto-Regular.ttf')", "url('/fonts/Roboto-Regular.ttf')")]
    #[case(
        "url(\"/fonts/Roboto-Regular.ttf\")",
        "url(\"/fonts/Roboto-Regular.ttf\")"
    )]
    #[case("'A B', 'C D', E", "\"A B\",\"C D\",E")]
    #[case("A,B,C", "A,B,C")]
    #[case("A, B, C", "A,B,C")]
    #[case("url(abc)", "url(abc)")]
    #[case("url(\"a bc\")", "url(\"a bc\")")]
    #[case("'A', 'B', 'C'", "A,B,C")]
    #[case("\"A\", \"B\", \"C\"", "A,B,C")]
    fn test_optimize_mutli_css_value(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_mutli_css_value(input), expected);
    }

    #[rstest]
    #[case("font-family", true)]
    #[case("src", true)]
    #[case("content", true)]
    #[case("animation-name", true)]
    #[case("background", false)]
    #[case("color", false)]
    #[case("margin", false)]
    fn test_check_multi_css_optimize(#[case] property: &str, #[case] expected: bool) {
        assert_eq!(check_multi_css_optimize(property), expected);
    }

    #[rstest]
    #[case("url('/fonts/Roboto-Regular.ttf')", "url('/fonts/Roboto-Regular.ttf')")]
    #[case(
        "url(\"/fonts/Roboto-Regular.ttf\")",
        "url(\"/fonts/Roboto-Regular.ttf\")"
    )]
    #[case("//fonts/Roboto-Regular.ttf", "url(//fonts/Roboto-Regular.ttf)")]
    #[case("fonts/Roboto-Regular.ttf", "url(fonts/Roboto-Regular.ttf)")]
    #[case(
        "local('fonts/Roboto Regular.ttf')",
        "local('fonts/Roboto Regular.ttf')"
    )]
    #[case("(hello)", "url(\"(hello)\")")]
    #[case("(hello world)", "url(\"(hello world)\")")]
    fn test_wrap_url(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(
            super::wrap_url(&super::optimize_mutli_css_value(input)),
            expected
        );
    }
}
