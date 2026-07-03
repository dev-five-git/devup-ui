use std::borrow::Cow;

use crate::{constant::CSS_COMMENT_RE, utils::collapse_whitespace};

pub fn rm_css_comment(value: &str) -> String {
    // On the common no-comment path `replace_all` returns `Cow::Borrowed`
    // (an alias of `value`), so collapse whitespace on the original slice and
    // skip materializing the borrowed-equal temporary.
    match CSS_COMMENT_RE.replace_all(value, "") {
        Cow::Borrowed(_) => collapse_whitespace(value),
        Cow::Owned(s) => collapse_whitespace(&s),
    }
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
    #[case(
        "div /* comment */ div /* comment */ div /* comment */ div",
        "div div div div"
    )]
    fn test_rm_css_comment(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(rm_css_comment(input), expected);
    }
}
