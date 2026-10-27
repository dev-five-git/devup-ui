use oxc_ast::ast::StringLiteral;

/// Canonical JSON string text, including JavaScript's unpaired surrogates.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Text(String);

impl Text {
    pub fn json(&self) -> &str {
        &self.0
    }

    pub fn literal(value: &StringLiteral<'_>) -> Self {
        Self::encode(value.value.encode_utf16())
    }

    pub fn is_empty(&self) -> bool {
        self.0 == "\"\""
    }

    pub fn is_default(&self) -> bool {
        self.0 == "\"default\""
    }

    fn encode(units: impl Iterator<Item = u16>) -> Self {
        let mut text = String::from("\"");
        for ch in char::decode_utf16(units) {
            match ch {
                Ok('"') => text.push_str("\\\""),
                Ok('\\') => text.push_str("\\\\"),
                Ok(ch) if ch <= '\u{1f}' => {
                    let mut scratch = [0; 2];
                    Self::escape(&mut text, ch.encode_utf16(&mut scratch)[0]);
                }
                Ok(ch) => text.push(ch),
                Err(error) => Self::escape(&mut text, error.unpaired_surrogate()),
            }
        }
        text.push('"');
        Self(text)
    }

    fn escape(text: &mut String, unit: u16) {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        text.push_str("\\u");
        for shift in [12, 8, 4, 0] {
            text.push(char::from(HEX[usize::from((unit >> shift) & 15)]));
        }
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Self::encode(value.encode_utf16())
    }
}
