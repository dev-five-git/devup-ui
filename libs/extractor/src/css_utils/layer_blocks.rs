pub(super) fn boundaries(text: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut characters = text.char_indices().peekable();
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;
    let mut parentheses = 0usize;
    std::iter::from_fn(move || {
        while let Some((index, character)) = characters.next() {
            if comment {
                if character == '*' && characters.peek().is_some_and(|(_, next)| *next == '/') {
                    characters.next();
                    comment = false;
                }
                continue;
            }
            if escaped {
                escaped = false;
                continue;
            }
            if character == '\\' {
                escaped = true;
                continue;
            }
            if let Some(end) = quote {
                if character == end {
                    quote = None;
                }
                continue;
            }
            match character {
                '/' if characters.peek().is_some_and(|(_, next)| *next == '*') => {
                    characters.next();
                    comment = true;
                }
                '\'' | '"' => quote = Some(character),
                '(' => parentheses += 1,
                ')' => parentheses = parentheses.saturating_sub(1),
                ';' | '{' | '}' if parentheses == 0 => return Some((index, character)),
                _ => {}
            }
        }
        None
    })
}
