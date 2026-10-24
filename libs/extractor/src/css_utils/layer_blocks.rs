pub(super) fn boundaries(text: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut characters = text.bytes().enumerate().peekable();
    let mut quote = css::quoted::QuoteState::default();
    let mut comment = false;
    let mut parentheses = 0usize;
    std::iter::from_fn(move || {
        while let Some((index, character)) = characters.next() {
            if comment {
                if character == b'*' && characters.peek().is_some_and(|(_, next)| *next == b'/') {
                    characters.next();
                    comment = false;
                }
                continue;
            }
            if !quote.is_syntax(character) {
                continue;
            }
            match character {
                b'/' if characters.peek().is_some_and(|(_, next)| *next == b'*') => {
                    characters.next();
                    comment = true;
                }
                b'(' => parentheses += 1,
                b')' => parentheses = parentheses.saturating_sub(1),
                b';' | b'{' | b'}' if parentheses == 0 => {
                    return Some((index, char::from(character)));
                }
                _ => {}
            }
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::boundaries;

    #[test]
    fn layer_boundaries_resume_when_bad_strings_end_at_raw_newlines() {
        // Given
        for newline in ["\n", "\r", "\r\n", "\u{c}"] {
            let text = format!("content: 'broken{newline}; @layer base {{ color: red; }}");
            // When
            let actual: Vec<_> = boundaries(&text).map(|(index, _)| &text[index..]).collect();
            // Then
            assert_eq!(
                actual,
                vec![
                    "; @layer base { color: red; }",
                    "{ color: red; }",
                    "; }",
                    "}"
                ]
            );
        }
    }

    #[test]
    fn layer_boundaries_resume_when_a_function_contains_a_bad_string() {
        // Given
        let text = "background: fn('broken\n); @layer base { color: red; }";
        // When
        let actual: Vec<_> = boundaries(text).map(|(_, character)| character).collect();
        // Then
        assert_eq!(actual, vec![';', '{', ';', '}']);
    }

    #[test]
    fn layer_boundaries_stay_quoted_when_hex_escapes_consume_newlines() {
        // Given
        for escape in ["\\62\n", "\\000062\r\n", "\\62\u{c}", "\\\r\n"] {
            let text =
                format!("@layer base {{ content: '{escape}}}; @layer fake {{'; color: red; }}");
            // When
            let actual: Vec<_> = boundaries(&text).map(|(_, character)| character).collect();
            // Then
            assert_eq!(actual, vec!['{', ';', ';', '}']);
        }
    }

    #[test]
    fn layer_boundaries_ignore_comments_and_nested_function_delimiters() {
        // Given
        let text =
            "/* ' \\ { ; ( */ @layer base { value: fn((a;{b}), '};'); /* ) } */ color: red; }";
        // When
        let actual: Vec<_> = boundaries(text).map(|(_, character)| character).collect();
        // Then
        assert_eq!(actual, vec!['{', ';', ';', '}']);
    }

    #[test]
    fn layer_boundaries_keep_escaped_delimiters_and_unicode_byte_offsets() {
        // Given
        let text = "@layer 한글\\{base { content: '\\' }'; color: red; }";
        // When
        let actual: Vec<_> = boundaries(text).map(|(index, _)| &text[index..]).collect();
        // Then
        assert_eq!(
            actual,
            vec![
                "{ content: '\\' }'; color: red; }",
                "; color: red; }",
                "; }",
                "}"
            ]
        );
    }

    #[test]
    fn layer_boundaries_ignore_eof_string_payload_whitespace() {
        // Given
        let text = "@layer base { content: 'open };   ";
        // When
        let actual: Vec<_> = boundaries(text).map(|(_, character)| character).collect();
        // Then
        assert_eq!(actual, vec!['{']);
    }
}
