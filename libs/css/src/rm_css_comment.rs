use crate::constant::CSS_COMMENT_RE;

pub fn rm_css_comment(value: &str) -> String {
    CSS_COMMENT_RE.replace_all(value, "").trim().split_ascii_whitespace().collect::<Vec<&str>>().join(" ").replace(", ", ",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("/* comment */", "")]
    #[case("/* comment */ div", "div")]
    #[case("div /* comment */", "div")]
    #[case("div /* comment */ div", "div div")]
    #[case("div /* comment */ div /* comment */", "div div")]
    #[case("div /* comment */ div /* comment */ div", "div div div")]
    #[case("div /* comment */ div /* comment */ div /* comment */", "div div div")]
    #[case(
        "div /* comment
         */ div /* comment */ div /* comment */",
        "div div div"
    )]
    #[case("div /* comment */ div /* comment */ div /* comment */ div", "div div div div")]
    fn test_rm_css_comment(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(rm_css_comment(input), expected);
    }
}
