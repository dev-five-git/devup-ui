use std::borrow::Cow;

use crate::constant::{CSS_FUNCTION_RE, OPTIMIZE_MULTI_CSS_VALUE_PROPERTY};

#[must_use]
pub fn optimize_multi_css_value(value: &str) -> String {
    // `+ 2` headroom: a bare (unquoted-in-source) family name that contains a
    // space gets re-wrapped in `"…"`, adding a quote pair beyond the input
    // length. Presizing for the common single-wrap case avoids a grow-realloc.
    let mut result = String::with_capacity(value.len() + 2);
    // Loop-invariant predicate: captures nothing, so construct it once per call
    // instead of once per comma-separated segment. Byte-scan equivalent of the
    // `[()\s]` regex — `\s` in `regex_lite` matches `[ \t\n\r\x0b\x0c]`, so the
    // vertical-tab (0x0b) and form-feed (0x0c) bytes MUST be included to stay
    // byte-identical.
    let quote_byte = |b: u8| matches!(b, b'(' | b')' | b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c);
    for (idx, s) in value.split(',').enumerate() {
        if idx > 0 {
            result.push(',');
        }

        let s = s.trim();
        let unquoted = if s.len() >= 2
            && ((s.starts_with('\'') && s.ends_with('\''))
                || (s.starts_with('"') && s.ends_with('"')))
        {
            &s[1..s.len() - 1]
        } else {
            s
        };

        // Fast path for empty/single-char segments (`''`/`""` strip to empty, the
        // lone `'`/`"` cases stay one char): `CSS_FUNCTION_RE = ^[a-zA-Z-]+(\(.*\))`
        // can NEVER match a string shorter than 2 bytes (it needs a name byte plus
        // a `(`), so a single quote-worthy byte is always wrapped and everything
        // else is pushed bare. Deciding this from the single byte skips both the
        // `.any()` scan and the regex-engine setup while staying byte-identical.
        if unquoted.len() <= 1 {
            if unquoted.len() == 1 && quote_byte(unquoted.as_bytes()[0]) {
                result.push('"');
                result.push_str(unquoted);
                result.push('"');
            } else {
                result.push_str(unquoted);
            }
            continue;
        }

        // Byte-scan equivalent of the `[()\s]` regex: `\s` in `regex_lite`
        // matches `[ \t\n\r\x0b\x0c]`, so the vertical-tab (0x0b) and form-feed
        // (0x0c) bytes MUST be included to stay byte-identical.
        let needs_quotes = unquoted.bytes().any(quote_byte);
        if needs_quotes && !CSS_FUNCTION_RE.is_match(unquoted) {
            result.push('"');
            result.push_str(unquoted);
            result.push('"');
        } else {
            result.push_str(unquoted);
        }
    }
    result
}

#[must_use]
pub fn wrap_url(s: &str) -> Cow<'_, str> {
    if CSS_FUNCTION_RE.is_match(s) {
        // Already a CSS function (e.g. `url(...)`/`local(...)`): borrow the input
        // instead of cloning the whole value onto the heap.
        Cow::Borrowed(s)
    } else {
        Cow::Owned(format!("url({s})"))
    }
}

#[must_use]
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
    #[case("'", "'")]
    #[case("\"", "\"")]
    #[case("url(abc)", "url(abc)")]
    #[case("url(\"a bc\")", "url(\"a bc\")")]
    #[case("'A', 'B', 'C'", "A,B,C")]
    #[case("\"A\", \"B\", \"C\"", "A,B,C")]
    fn test_optimize_multi_css_value(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_multi_css_value(input), expected);
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
            super::wrap_url(&super::optimize_multi_css_value(input)),
            expected
        );
    }
}
