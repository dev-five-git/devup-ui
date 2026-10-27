pub(super) fn location(code: &str, offset: u32) -> (usize, usize) {
    let offset = usize::try_from(offset).unwrap_or(code.len());
    let mut line = 1;
    let mut column = 1;
    let mut carriage_return = false;
    for (byte, ch) in code.char_indices() {
        if byte >= offset {
            break;
        }
        match ch {
            '\n' if carriage_return => {}
            '\r' | '\n' | '\u{2028}' | '\u{2029}' => {
                line += 1;
                column = 1;
            }
            _ => column += ch.len_utf16(),
        }
        carriage_return = ch == '\r';
    }
    (line, column)
}
