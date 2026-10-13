//! Structural CSS boundaries, before comment removal or value normalization.

use std::ops::Range;

#[derive(Debug)]
pub(crate) enum Item {
    Declaration {
        key: Range<usize>,
        value: Range<usize>,
    },
    Block {
        prelude: Range<usize>,
        body: Range<usize>,
    },
    Statement(Range<usize>),
}

/// Structural punctuation outside CSS strings, comments and functional values.
pub(crate) fn boundaries(text: &str) -> Vec<(usize, u8)> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut index = 0;
    let mut quote = None;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' {
            index = (index + 2).min(bytes.len());
            continue;
        }
        if let Some(delimiter) = quote {
            if byte == delimiter {
                quote = None;
            }
            index += 1;
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index = text[index + 2..]
                .find("*/")
                .map_or(bytes.len(), |end| index + end + 4);
            continue;
        }
        match byte {
            b'\'' | b'"' => quote = Some(byte),
            b'(' => parens += 1,
            b')' => parens = parens.saturating_sub(1),
            b'[' => brackets += 1,
            b']' => brackets = brackets.saturating_sub(1),
            b'{' | b'}' | b';' | b':' if parens == 0 && brackets == 0 => result.push((index, byte)),
            _ => {}
        }
        index += 1;
    }
    result
}

pub(crate) fn trimmed(text: &str, range: Range<usize>) -> Range<usize> {
    let slice = &text[range.clone()];
    let mut start = range.start + slice.len() - slice.trim_start().len();
    while text[start..range.end].starts_with("/*") {
        let Some(end) = text[start + 2..range.end].find("*/") else {
            return range.end..range.end;
        };
        start += end + 4;
        start += text[start..range.end].len() - text[start..range.end].trim_start().len();
    }
    let end = start + text[start..range.end].trim_end().len();
    start..end
}

/// One declaration scope, retaining source byte ranges for its direct children.
pub(crate) fn items(text: &str) -> Vec<Item> {
    let marks = boundaries(text);
    let mut result = Vec::new();
    let mut start = 0;
    let mut colon = None;
    let mut index = 0;
    while index < marks.len() {
        let (at, byte) = marks[index];
        match byte {
            b':' if colon.is_none() => colon = Some(at),
            b'{' => {
                let custom_value =
                    colon.is_some_and(|at| clean(&text[start..at]).starts_with("--"));
                let mut depth = 1usize;
                let mut end = text.len();
                index += 1;
                while index < marks.len() {
                    match marks[index].1 {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    if depth == 0 {
                        end = marks[index].0;
                        break;
                    }
                    index += 1;
                }
                if !custom_value {
                    result.push(Item::Block {
                        prelude: trimmed(text, start..at),
                        body: at + 1..end,
                    });
                    start = (end + 1).min(text.len());
                    colon = None;
                }
            }
            b';' => {
                append(text, start..at, colon, &mut result);
                start = at + 1;
                colon = None;
            }
            _ => {}
        }
        index += 1;
    }
    append(text, start..text.len(), colon, &mut result);
    result
}

fn append(text: &str, range: Range<usize>, colon: Option<usize>, result: &mut Vec<Item>) {
    let range = trimmed(text, range);
    if clean(&text[range.clone()]).trim().is_empty() {
        return;
    }
    match colon {
        Some(at) => result.push(Item::Declaration {
            key: trimmed(text, range.start..at),
            value: trimmed(text, at + 1..range.end),
        }),
        None => result.push(Item::Statement(range)),
    }
}

/// Remove actual comments, preserving strings and their whitespace verbatim.
pub(crate) fn clean(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut start = 0;
    for range in comments(text) {
        result.push_str(&text[start..range.start]);
        start = range.end;
    }
    result.push_str(&text[start..]);
    result
}

pub(crate) fn comments(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut result = Vec::new();
    let mut index = 0;
    let mut quote = None;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => {
                index = (index + 2).min(bytes.len());
                continue;
            }
            byte if quote == Some(byte) => quote = None,
            b'\'' | b'"' if quote.is_none() => quote = Some(bytes[index]),
            b'/' if quote.is_none() && bytes.get(index + 1) == Some(&b'*') => {
                let start = index;
                index = text[index + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |end| index + end + 4);
                result.push(start..index);
                continue;
            }
            _ => {}
        }
        index += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_cursor_when_punctuation_is_quoted_only_delimits_real_scopes() {
        let text = concat!(
            "/*style-order:9*/content:';{",
            "}:style-order:8';background:url('data:a;{",
            "}');[style-order='2']{color:red}style-order:2"
        );
        let declarations: Vec<_> = items(text)
            .into_iter()
            .filter_map(|item| match item {
                Item::Declaration { key, .. } => Some(clean(&text[key])),
                Item::Block { .. } | Item::Statement(_) => None,
            })
            .collect();
        assert_eq!(declarations, vec!["content", "background", "style-order"]);
    }

    #[test]
    fn literal_cursor_when_custom_property_contains_braces_retains_token_stream() {
        let text = "--data:{style-order:9;content:'}';};style-order:2";
        let declarations: Vec<_> = items(text)
            .into_iter()
            .filter_map(|item| match item {
                Item::Declaration { key, value } => Some((&text[key], &text[value])),
                Item::Block { .. } | Item::Statement(_) => None,
            })
            .collect();
        assert_eq!(
            declarations,
            vec![
                ("--data", "{style-order:9;content:'}';}"),
                ("style-order", "2")
            ]
        );
    }

    #[test]
    fn literal_cursor_when_comments_are_cleaned_preserves_quoted_text_and_escapes() {
        let text = concat!(r"content:'/*data*/;\'{", r"}';/*discard*/--x:a\;b;");
        assert_eq!(
            clean(text),
            concat!(r"content:'/*data*/;\'{", r"}';--x:a\;b;")
        );
    }
}
