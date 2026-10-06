use super::cursor;

pub(super) fn clean_value(text: &str, order: bool) -> String {
    if !order {
        return cursor::clean(text);
    }
    let mut value = String::with_capacity(text.len());
    let mut start = 0;
    for comment in cursor::comments(text) {
        value.push_str(&text[start..comment.start]);
        value.push(' ');
        start = comment.end;
    }
    value.push_str(&text[start..]);
    value
}
