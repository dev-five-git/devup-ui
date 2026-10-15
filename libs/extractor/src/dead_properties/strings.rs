use oxc_ast::ast::StringLiteral;

pub(super) fn origins(literal: &StringLiteral<'_>) -> Vec<u32> {
    let Some(raw) = literal
        .raw
        .as_ref()
        .and_then(|raw| raw.as_str().get(1..raw.len().saturating_sub(1)))
    else {
        return std::iter::repeat_n(literal.span.start, literal.value.len()).collect();
    };
    let mut origins = Vec::new();
    let mut characters = raw.char_indices().peekable();
    while let Some((start, character)) = characters.next() {
        let mut index = start + character.len_utf8();
        let length = if character == '\\' {
            characters.next().map_or(0, |(_, escape)| {
                index += escape.len_utf8();
                match escape {
                    '\n' => 0,
                    '\r' => {
                        if raw.as_bytes().get(index) == Some(&b'\n') {
                            index += 1;
                        }
                        0
                    }
                    'x' => {
                        index = (index + 2).min(raw.len());
                        1
                    }
                    'u' => {
                        let (end, digits) = if raw.as_bytes().get(index) == Some(&b'{') {
                            let end = raw[index..].find('}').map_or(raw.len(), |end| index + end);
                            (end.saturating_add(1).min(raw.len()), &raw[index + 1..end])
                        } else {
                            let end = (index + 4).min(raw.len());
                            (end, &raw[index..end])
                        };
                        index = end;
                        let codepoint = u32::from_str_radix(digits, 16).unwrap_or(0xfffd);
                        if (0xd800..=0xdbff).contains(&codepoint) && raw[index..].starts_with("\\u")
                        {
                            index = (index + 6).min(raw.len());
                            4
                        } else {
                            char::from_u32(codepoint).unwrap_or('\u{fffd}').len_utf8()
                        }
                    }
                    _ => escape.len_utf8(),
                }
            })
        } else {
            character.len_utf8()
        };
        while characters.next_if(|(offset, _)| *offset < index).is_some() {}
        if let Ok(offset) = u32::try_from(start) {
            origins.extend(std::iter::repeat_n(literal.span.start + 1 + offset, length));
        }
    }
    origins
}
