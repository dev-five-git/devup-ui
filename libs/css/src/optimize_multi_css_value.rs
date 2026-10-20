use std::borrow::Cow;

use crate::constant::{CSS_FUNCTION_RE, OPTIMIZE_MULTI_CSS_VALUE_PROPERTY};
use crate::quoted::{self, PROTECTED};

/// Whether a byte makes a bare value need quotes: a parenthesis or white space
/// (`\s` in `regex_lite` is `[ \t\n\r\x0b\x0c]`, so vertical tab and form feed
/// MUST be included)
const fn needs_quote_byte(b: u8) -> bool {
    matches!(b, b'(' | b')' | b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

#[must_use]
pub fn optimize_multi_css_value(value: &str) -> Cow<'_, str> {
    // One scan for both bytes that steer the work below.
    let (has_comma, has_quote) = value.bytes().fold((false, false), |(comma, quote), b| {
        (comma || b == b',', quote || matches!(b, b'\'' | b'"'))
    });
    if has_quote {
        return Cow::Owned(optimize_quoted_list(value));
    }
    // Borrow fast path: a single bare segment (no comma) whose trimmed form is
    // already the exact output. This is the common `Roboto` / `sans-serif` case.
    // It qualifies only when trimming does not slice (the trimmed slice IS a
    // suffix-and-prefix of `value` — we return that borrowed slice) and it needs no
    // re-quoting. Every other shape (comma list, requote-needed, <=1 byte) falls
    // through to the owning builder below and stays byte-identical.
    if !has_comma {
        let s = value.trim();
        if s.len() >= 2 {
            let needs_quotes = s.bytes().any(needs_quote_byte);
            if !needs_quotes || CSS_FUNCTION_RE.is_match(s) {
                // Output would be exactly `s` pushed bare -> borrow it.
                return Cow::Borrowed(s);
            }
        } else if s.len() == 1 && !needs_quote_byte(s.as_bytes()[0]) {
            // A single-byte bare segment can never be a quote *pair*, and
            // `CSS_FUNCTION_RE = ^[a-zA-Z-]+(\(.*\))` can never match `<2` bytes, so it
            // is never re-quoted. When the byte is not quote-worthy the owned
            // builder pushes it bare too — the output is exactly `s`, so borrow it and
            // skip the `String` allocation.
            return Cow::Borrowed(s);
        }
    }
    Cow::Owned(optimize_multi_css_value_owned(value, has_comma))
}

/// Push one comma-separated segment that holds no quoted string: wrapped in
/// quotes when it holds a parenthesis or white space and is not a function
fn push_segment(result: &mut String, s: &str) {
    // Fast path for empty/single-char segments: `CSS_FUNCTION_RE` can NEVER match a
    // string shorter than 2 bytes (it needs a name byte plus a `(`), so a single
    // quote-worthy byte is always wrapped and everything else is pushed bare.
    // Deciding this from the single byte skips both the `.any()` scan and the
    // regex-engine setup while staying byte-identical.
    if s.len() <= 1 {
        if s.len() == 1 && needs_quote_byte(s.as_bytes()[0]) {
            result.push('"');
            result.push_str(s);
            result.push('"');
        } else {
            result.push_str(s);
        }
        return;
    }

    let needs_quotes = s.bytes().any(needs_quote_byte);
    if needs_quotes && !CSS_FUNCTION_RE.is_match(s) {
        result.push('"');
        result.push_str(s);
        result.push('"');
    } else {
        result.push_str(s);
    }
}

fn optimize_multi_css_value_owned(value: &str, has_comma: bool) -> String {
    // Headroom for re-wrapped segments: EACH comma-separated segment may gain a
    // `"…"` quote pair (+2 bytes), so a multi-segment list like `A B, C D, E`
    // → `"A B","C D",E` can add up to one pair per segment. Budget `2` per comma
    // plus a trailing `2` so even the worst case (every segment re-wrapped) never
    // triggers a grow-realloc. Output stays byte-identical.
    // Headroom is `2` per comma, `0` when none. The caller already probed comma
    // presence to pick its borrow fast path and hands the result in via
    // `has_comma`. On the no-comma owned path the count is provably `0`, so we skip
    // a second full O(n) `value.bytes()` scan entirely. Only the comma-present path
    // pays the count.
    let comma_headroom = if has_comma {
        value.bytes().filter(|&b| b == b',').count() * 2
    } else {
        0
    };
    let mut result = String::with_capacity(value.len() + comma_headroom + 2);
    for (idx, s) in value.split(',').enumerate() {
        if idx > 0 {
            result.push(',');
        }
        push_segment(&mut result, s.trim());
    }
    result
}

/// The content of a quoted string that can stand bare or in double quotes without
/// changing what it says: no escape, and no `,` or `;` to end the item. Another
/// kind of quote is only fine inside a function such as `url('a.woff2')`.
fn plain_content(string: &str) -> Option<&str> {
    let quote = string.chars().next()?;
    let content = string.strip_prefix(quote)?.strip_suffix(quote)?;
    if content.is_empty() || content.contains(['\\', ',', ';']) {
        return None;
    }
    (CSS_FUNCTION_RE.is_match(content) || !content.contains(['\'', '"'])).then_some(content)
}

/// `optimize_multi_css_value` for a value with quoted strings. A comma inside a
/// string does not separate items. An item that is one string loses its quotes, or
/// gets double ones, only when its content says the same either way; any other
/// item holding a string stays as written. Items without strings are optimized as
/// they are in a value without any.
fn optimize_quoted_list(value: &str) -> String {
    let (masked, spans) = quoted::mask(value);
    let mut spans = spans.into_iter();
    let mut result = String::with_capacity(value.len() + 2);
    for (idx, segment) in masked.split(',').enumerate() {
        if idx > 0 {
            result.push(',');
        }
        let segment = segment.trim();
        if !segment.contains(PROTECTED) {
            push_segment(&mut result, segment);
        } else if segment.len() == PROTECTED.len_utf8()
            && let Some(string) = spans.next()
        {
            match plain_content(string) {
                Some(content) => push_segment(&mut result, content),
                None => result.push_str(string),
            }
        } else {
            for (idx, part) in segment.split(PROTECTED).enumerate() {
                if idx > 0 {
                    result.push_str(spans.next().unwrap_or_default());
                }
                result.push_str(part);
            }
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
        // Build `url(<s>)` into a presized buffer instead of routing through
        // `format!`'s `Arguments` machinery; the single owned allocation is
        // unavoidable but the formatting overhead is not. Byte-identical output.
        let mut out = String::with_capacity(s.len() + 5);
        out.push_str("url(");
        out.push_str(s);
        out.push(')');
        Cow::Owned(out)
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
    #[case("A", "A")]
    #[case("x", "x")]
    #[case("' '", "\" \"")]
    #[case("url(abc)", "url(abc)")]
    #[case("url(\"a bc\")", "url(\"a bc\")")]
    #[case("'A', 'B', 'C'", "A,B,C")]
    #[case("\"A\", \"B\", \"C\"", "A,B,C")]
    fn test_optimize_multi_css_value(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_multi_css_value(input), expected);
    }

    #[rstest]
    #[case(r"'A, B', sans-serif", r"'A, B',sans-serif")]
    #[case(r#""A,B", C"#, r#""A,B",C"#)]
    #[case(r"'A,B'", r"'A,B'")]
    #[case(r"',', a", r"',',a")]
    #[case(r"'a', 'b,c', d", r"a,'b,c',d")]
    #[case(r"'A B' , C", r#""A B",C"#)]
    #[case(
        r#"url("a,b.woff2") format("woff2"), local(x)"#,
        r#"url("a,b.woff2") format("woff2"),local(x)"#
    )]
    #[case(r#"'say "hi"'"#, r#"'say "hi"'"#)]
    #[case(r#""url('/fonts/a.ttf')""#, "url('/fonts/a.ttf')")]
    #[case(r#"'local("Open Sans")'"#, r#"local("Open Sans")"#)]
    #[case(r#""url(a), url(b)""#, r#""url(a), url(b)""#)]
    #[case(r#"'url(\"a\")'"#, r#"'url(\"a\")'"#)]
    #[case(r#""it's""#, r#""it's""#)]
    #[case(r"'It\'s two', b", r"'It\'s two',b")]
    #[case(r"'\\' , a", r"'\\',a")]
    #[case(r#"Open "X" Sans, b"#, r#"Open "X" Sans,b"#)]
    #[case("''", "''")]
    #[case(r"'', a", r"'',a")]
    #[case(r#""", a"#, r#""",a"#)]
    #[case(r"'a, b", r"'a, b")]
    fn test_optimize_multi_css_value_keeps_quoted_strings(
        #[case] input: &str,
        #[case] expected: &str,
    ) {
        assert_eq!(optimize_multi_css_value(input), expected);
        assert_eq!(optimize_multi_css_value(expected), expected);
    }

    #[rstest]
    #[case("font-family", true)]
    #[case("src", true)]
    #[case("content", false)]
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
