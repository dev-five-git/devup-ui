/// Stands for one quoted string in masked text. No optimization pass reads it:
/// they look for ASCII digits, white space, parentheses, `,`, `#`, `;` and `.`
pub(crate) const PROTECTED: char = '\u{E000}';

/// End of a string span, including its closing quote or raw bad-string newline.
/// Escaped newlines stay inside the span; EOF string payload is never discarded.
#[must_use]
pub fn string_end(bytes: &[u8], start: usize, quote: u8) -> usize {
    let mut state = QuoteState {
        quote: Some(quote),
        ..QuoteState::default()
    };
    let mut index = start + 1;
    while let Some(&byte) = bytes.get(index) {
        state.is_syntax(byte);
        index += 1;
        if state.quote.is_none() {
            if byte == b'\r' && bytes.get(index) == Some(&b'\n') {
                index += 1;
            }
            return index;
        }
    }
    bytes.len()
}

/// Quote and escape state retained across adjacent CSS text fragments.
#[derive(Default)]
pub struct QuoteState {
    quote: Option<u8>,
    escaped: bool,
    skip_lf: bool,
    hex_digits: u8,
}

impl QuoteState {
    /// Consume a byte, returning whether it is unquoted, unescaped syntax.
    #[inline]
    pub fn is_syntax(&mut self, byte: u8) -> bool {
        if self.skip_lf {
            self.skip_lf = false;
            if byte == b'\n' {
                return false;
            }
        }
        if self.hex_digits > 0 {
            if byte.is_ascii_hexdigit() && self.hex_digits < 6 {
                self.hex_digits += 1;
                return false;
            }
            self.hex_digits = 0;
            if matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c') {
                self.skip_lf = byte == b'\r';
                return false;
            }
        }
        if self.escaped {
            self.escaped = false;
            self.skip_lf = byte == b'\r' && self.quote.is_some();
            self.hex_digits = u8::from(byte.is_ascii_hexdigit());
            return false;
        }
        if byte == b'\\' {
            self.escaped = true;
            return false;
        }
        match self.quote {
            Some(quote) => {
                if byte == quote || matches!(byte, b'\n' | b'\r' | b'\x0c') {
                    self.quote = None;
                }
                self.skip_lf = byte == b'\r';
                false
            }
            None => {
                if matches!(byte, b'\'' | b'"') {
                    self.quote = Some(byte);
                    false
                } else {
                    true
                }
            }
        }
    }
}

/// Trim outside whitespace without discarding EOF payload or bad-string newlines.
#[must_use]
pub fn trim_css(value: &str) -> &str {
    let value = value.trim_start();
    let trimmed = value.trim_end();
    if value.len() == trimmed.len()
        || !value
            .bytes()
            .any(|byte| matches!(byte, b'\'' | b'"' | b'\\'))
    {
        return trimmed;
    }
    let mut state = QuoteState::default();
    let mut end = trimmed.len();
    for (index, c) in value.char_indices() {
        if !state.is_syntax(value.as_bytes()[index]) {
            end = end.max(index + c.len_utf8());
        }
    }
    &value[..end]
}

/// The bytes outside quoted strings and escapes, with their positions.
struct Unquoted<'a> {
    bytes: &'a [u8],
    index: usize,
    state: QuoteState,
}

impl Iterator for Unquoted<'_> {
    type Item = (usize, u8);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let &byte = self.bytes.get(self.index)?;
            self.index += 1;
            if self.state.is_syntax(byte) {
                return Some((self.index - 1, byte));
            }
        }
    }
}

/// The bytes of `text` outside quoted strings and escapes, with their positions.
pub fn unquoted_bytes(text: &str) -> impl Iterator<Item = (usize, u8)> + '_ {
    Unquoted {
        bytes: text.as_bytes(),
        index: 0,
        state: QuoteState::default(),
    }
}

/// Replace an ASCII delimiter outside strings and escapes, borrowing if absent.
#[must_use]
pub fn replace_unquoted<'a>(
    text: &'a str,
    delimiter: u8,
    replacement: &str,
) -> std::borrow::Cow<'a, str> {
    let mut positions = unquoted_bytes(text).filter(|&(_, byte)| byte == delimiter);
    let Some((first, _)) = positions.next() else {
        return std::borrow::Cow::Borrowed(text);
    };
    let mut out = String::with_capacity(text.len() + replacement.len());
    out.push_str(&text[..first]);
    out.push_str(replacement);
    let mut start = first + 1;
    for (index, _) in positions {
        out.push_str(&text[start..index]);
        out.push_str(replacement);
        start = index + 1;
    }
    out.push_str(&text[start..]);
    std::borrow::Cow::Owned(out)
}

/// Where `needle` first stands in `text` outside quoted strings
#[must_use]
pub fn find_unquoted(text: &str, needle: u8) -> Option<usize> {
    unquoted_bytes(text)
        .find(|&(_, byte)| byte == needle)
        .map(|(index, _)| index)
}

/// Where `needle` last stands in `text` outside quoted strings
#[must_use]
pub fn rfind_unquoted(text: &str, needle: u8) -> Option<usize> {
    unquoted_bytes(text)
        .filter(|&(_, byte)| byte == needle)
        .last()
        .map(|(index, _)| index)
}

/// The parts of `text` between the `delimiter`s outside quoted strings
pub fn split_unquoted(text: &str, delimiter: u8) -> impl Iterator<Item = &str> {
    let mut positions = unquoted_bytes(text)
        .filter(move |&(_, byte)| byte == delimiter)
        .map(|(index, _)| index);
    let mut start = Some(0);
    std::iter::from_fn(move || {
        let from = start?;
        if let Some(index) = positions.next() {
            start = Some(index + 1);
            Some(&text[from..index])
        } else {
            start = None;
            Some(&text[from..])
        }
    })
}

/// Mask strings, ASCII escapes and literal markers, retaining their original spans.
pub(crate) fn mask(value: &str) -> (String, Vec<&str>) {
    let bytes = value.as_bytes();
    let mut masked = String::with_capacity(value.len());
    let mut spans = Vec::new();
    let mut copied = 0;
    let mut index = 0;
    while index < bytes.len() {
        let end = match bytes[index] {
            b'\\' if bytes.get(index + 1).is_some_and(u8::is_ascii) => index + 2,
            quote @ (b'\'' | b'"') => string_end(bytes, index, quote),
            0xEE if value[index..].starts_with(PROTECTED) => index + PROTECTED.len_utf8(),
            _ => {
                index += 1;
                continue;
            }
        };
        masked.push_str(&value[copied..index]);
        masked.push(PROTECTED);
        spans.push(&value[index..end]);
        copied = end;
        index = end;
    }
    masked.push_str(&value[copied..]);
    (masked, spans)
}

/// Put `spans` back in place of the [`PROTECTED`] characters of `masked`
pub(crate) fn unmask(masked: &str, spans: &[&str]) -> String {
    let mut parts = masked.split(PROTECTED);
    let mut value =
        String::with_capacity(masked.len() + spans.iter().map(|span| span.len()).sum::<usize>());
    value.push_str(parts.next().unwrap_or_default());
    for (part, span) in parts.zip(spans) {
        value.push_str(span);
        value.push_str(part);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("red", "red", &[])]
    #[case("'a'", "\u{E000}", &["'a'"])]
    #[case("1 'a' 2", "1 \u{E000} 2", &["'a'"])]
    #[case(r#""a" 'b'"#, "\u{E000} \u{E000}", &[r#""a""#, "'b'"])]
    #[case(r#"'a"b' "c'd""#, "\u{E000} \u{E000}", &[r#"'a"b'"#, r#""c'd""#])]
    #[case(r"'a\'b' c", "\u{E000} c", &[r"'a\'b'"])]
    #[case(r"'a\\' b", "\u{E000} b", &[r"'a\\'"])]
    #[case("'a\\한' b", "\u{E000} b", &["'a\\한'"])]
    #[case("x 'abc", "x \u{E000}", &["'abc"])]
    #[case("x '", "x \u{E000}", &["'"])]
    #[case(r"x 'a\", "x \u{E000}", &[r"'a\"])]
    #[case("a\u{E000}b 'c'", "a\u{E000}b \u{E000}", &["\u{E000}", "'c'"])]
    #[case("한글 '한 글' 0.5", "한글 \u{E000} 0.5", &["'한 글'"])]
    fn masks_quoted_strings_and_restores_them(
        #[case] value: &str,
        #[case] masked: &str,
        #[case] spans: &[&str],
    ) {
        let (actual, actual_spans) = mask(value);
        assert_eq!(actual, masked);
        assert_eq!(actual_spans, spans);
        assert_eq!(unmask(&actual, &actual_spans), value);
    }

    #[rstest]
    #[case("a;b", b';', Some(1), Some(1))]
    #[case("a'x;y';b", b';', Some(6), Some(6))]
    #[case("'x;y'", b';', None, None)]
    #[case("a;b;c", b';', Some(1), Some(3))]
    #[case("\"{\" {", b'{', Some(4), Some(4))]
    #[case("'open ; string", b';', None, None)]
    #[case("한;'글;'", b';', Some(3), Some(3))]
    fn finds_bytes_outside_quoted_strings(
        #[case] text: &str,
        #[case] needle: u8,
        #[case] first: Option<usize>,
        #[case] last: Option<usize>,
    ) {
        assert_eq!(find_unquoted(text, needle), first);
        assert_eq!(rfind_unquoted(text, needle), last);
    }

    #[rstest]
    #[case("a;b", vec!["a", "b"])]
    #[case("a;'b;c';d", vec!["a", "'b;c'", "d"])]
    #[case("", vec![""])]
    #[case(";", vec!["", ""])]
    #[case("'x;", vec!["'x;"])]
    #[case("한;'글;'", vec!["한", "'글;'"])]
    fn splits_outside_quoted_strings(#[case] text: &str, #[case] parts: Vec<&str>) {
        assert_eq!(split_unquoted(text, b';').collect::<Vec<_>>(), parts);
    }
}
