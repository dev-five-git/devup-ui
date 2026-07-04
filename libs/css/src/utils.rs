use std::borrow::Cow;

use regex_lite::Regex;

/// Compile a built-in regex pattern, panicking with a descriptive message.
///
/// The pattern strings are compile-time constants, so a failure here is a
/// programming error, not a runtime condition. Shared canonical helper so `css`
/// and its dependent crates (e.g. `sheet`) use one definition and one
/// panic-message wording instead of copy-pasted duplicates.
#[must_use]
pub fn compile_regex(pattern: &str) -> Regex {
    Regex::new(pattern)
        .unwrap_or_else(|err| panic!("invalid built-in regex pattern `{pattern}`: {err}"))
}

#[inline]
#[must_use]
pub fn to_kebab_case(value: &str) -> String {
    let mut result = String::with_capacity(value.len() + 4);
    // Inputs here are always ASCII CSS property / selector identifiers. Use the
    // ASCII-only uppercase check (a single byte compare) instead of
    // `char::is_uppercase()`, which consults the Unicode uppercase tables. This
    // matches the sibling `to_camel_case` (which already uses `to_ascii_uppercase`)
    // and keeps output byte-identical: any non-ASCII char (never ASCII-uppercase)
    // is copied through verbatim, exactly as before.
    for (i, c) in value.chars().enumerate() {
        if c.is_ascii_uppercase() {
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
            // Split "head char + rest" once so the first char's byte width is
            // derived a single time (via `split_at`) instead of recomputing
            // `first.len_utf8()` to re-slice `rest`. Byte-identical output.
            let (_, rest) = s.split_at(first.len_utf8());
            result.push(first.to_ascii_uppercase());
            result.push_str(rest);
        }
    }
    result
}

#[inline]
#[must_use]
pub(crate) fn collapse_whitespace(value: &str) -> Cow<'_, str> {
    // Fast path: the dominant selector inputs (`"hover"`, `"focusVisible"`,
    // `"active"`, already-tight `"a,b"`) need NO collapsing, so their output
    // equals the input verbatim. Detect that and BORROW instead of allocating a
    // fresh `String`. Collapsing is required only when the value has a leading /
    // trailing / doubled ASCII whitespace run, OR a `", "` (space right after a
    // comma) that the comma-tightening rule below would strip. This scan is
    // byte-wise (all whitespace here is ASCII) and stays byte-identical with the
    // allocating slow path.
    let bytes = value.as_bytes();
    let mut needs_collapse = false;
    let mut prev_ws = true; // treat start-of-string as preceding whitespace so a leading run is caught
    let mut prev_comma = false;
    for &b in bytes {
        let is_ws = b.is_ascii_whitespace();
        if is_ws && (prev_ws || prev_comma) {
            // leading run, doubled whitespace, or a space right after a comma
            needs_collapse = true;
            break;
        }
        prev_ws = is_ws;
        prev_comma = b == b',';
    }
    // a trailing whitespace run also needs collapsing
    if !needs_collapse && prev_ws && !value.is_empty() {
        needs_collapse = true;
    }
    if !needs_collapse {
        return Cow::Borrowed(value);
    }

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
    Cow::Owned(result)
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
