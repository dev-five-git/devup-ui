use std::iter::Peekable;
use std::str::Chars;

const fn name_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_' || character >= '\u{80}'
}

fn escape(characters: &mut Peekable<Chars<'_>>) -> Option<char> {
    let first = characters.next()?;
    if matches!(first, '\n' | '\r' | '\u{c}') {
        return None;
    }
    if let Some(mut value) = first.to_digit(16) {
        for _ in 1..6 {
            let Some(digit) = characters
                .peek()
                .and_then(|character| character.to_digit(16))
            else {
                break;
            };
            characters.next();
            value = value * 16 + digit;
        }
        if characters
            .peek()
            .is_some_and(|character| matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{c}'))
        {
            let whitespace = characters.next();
            if whitespace == Some('\r') && characters.peek() == Some(&'\n') {
                characters.next();
            }
        }
        Some(
            char::from_u32(value)
                .filter(|character| *character != '\0')
                .unwrap_or('\u{fffd}'),
        )
    } else {
        Some(first)
    }
}

fn identifier(characters: &mut Peekable<Chars<'_>>) -> Option<String> {
    let first = characters.next()?;
    let mut decoded = String::new();
    match first {
        '\\' => decoded.push(escape(characters)?),
        '-' => {
            if !characters
                .peek()
                .is_some_and(|next| name_start(*next) || matches!(next, '-' | '\\'))
            {
                return None;
            }
            decoded.push(first);
        }
        character if name_start(character) => decoded.push(character),
        _ => return None,
    }
    while let Some(character) = characters.peek().copied() {
        if character == '\\' {
            characters.next();
            decoded.push(escape(characters)?);
        } else if name_start(character) || character.is_ascii_digit() || character == '-' {
            characters.next();
            decoded.push(character);
        } else {
            break;
        }
    }
    if ["initial", "inherit", "unset", "revert", "revert-layer"]
        .iter()
        .any(|reserved| decoded.eq_ignore_ascii_case(reserved))
    {
        return None;
    }
    Some(serialize(&decoded))
}

fn serialize(decoded: &str) -> String {
    let mut result = String::new();
    let lone_hyphen = decoded == "-";
    let leading_hyphen = decoded.starts_with('-');
    for (index, character) in decoded.chars().enumerate() {
        if character.is_ascii_digit() && (index == 0 || index == 1 && leading_hyphen)
            || character <= '\u{1f}'
            || character == '\u{7f}'
        {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            let mut bytes = [0; 4];
            character.encode_utf8(&mut bytes);
            let value = bytes[0];
            result.push('\\');
            if value >= 16 {
                result.push(char::from(HEX[usize::from(value >> 4)]));
            }
            result.push(char::from(HEX[usize::from(value & 15)]));
            result.push(' ');
        } else if name_start(character)
            || character.is_ascii_digit()
            || character == '-' && !lone_hyphen
        {
            result.push(character);
        } else {
            result.push('\\');
            result.push(character);
        }
    }
    result
}

fn skip_trivia(characters: &mut Peekable<Chars<'_>>, whitespace: bool) -> Option<()> {
    loop {
        while whitespace
            && characters
                .peek()
                .is_some_and(|character| matches!(character, ' ' | '\t' | '\n' | '\r' | '\u{c}'))
        {
            characters.next();
        }
        if characters.peek() != Some(&'/') {
            return Some(());
        }
        let mut comment = characters.clone();
        comment.next();
        if comment.next() != Some('*') {
            return Some(());
        }
        loop {
            if comment.next()? == '*' && comment.peek() == Some(&'/') {
                comment.next();
                *characters = comment;
                break;
            }
        }
    }
}

/// Canonical CSS serialization keeps escaped dots within an identifier distinct
/// from dots separating nested layers, while equivalent escapes share metadata.
pub(crate) fn parse(name: &str) -> Option<String> {
    let mut characters = name.chars().peekable();
    let mut parts = Vec::new();
    skip_trivia(&mut characters, true)?;
    loop {
        skip_trivia(&mut characters, false)?;
        parts.push(identifier(&mut characters)?);
        skip_trivia(&mut characters, false)?;
        match characters.next() {
            Some('.') => {}
            None => return Some(parts.join(".")),
            Some(' ' | '\t' | '\n' | '\r' | '\u{c}') => {
                skip_trivia(&mut characters, true)?;
                return characters.next().is_none().then(|| parts.join("."));
            }
            Some(_) => return None,
        }
    }
}
