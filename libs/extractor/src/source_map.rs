//! Source maps of code extraction parsed after editing it, pointed back at
//! the code as written

use std::borrow::Cow;

use oxc_sourcemap::{SourceMap, Token};

use crate::import_alias_visit::{Edit, source_offset};

/// Where the lines of a text start, split as the codegen splits them
struct Lines<'t> {
    text: &'t str,
    starts: Vec<usize>,
}

impl<'t> Lines<'t> {
    fn new(text: &'t str) -> Self {
        let mut starts = vec![0];
        let mut characters = text.char_indices().peekable();
        while let Some((index, character)) = characters.next() {
            let end = match character {
                '\r' if characters.next_if(|(_, next)| *next == '\n').is_some() => index + 2,
                '\r' | '\n' | '\u{2028}' | '\u{2029}' => index + character.len_utf8(),
                _ => continue,
            };
            starts.push(end);
        }
        Self { text, starts }
    }

    /// The byte offset of `column`, in UTF-16 units, on `line`
    fn offset(&self, line: u32, column: u32) -> usize {
        let start = self
            .starts
            .get(line as usize)
            .copied()
            .unwrap_or(self.text.len());
        let mut units = 0;
        for (index, character) in self.text[start..].char_indices() {
            if units >= column as usize {
                return start + index;
            }
            units += character.len_utf16();
        }
        self.text.len()
    }

    /// The line and UTF-16 column of the byte `offset`
    fn position(&self, offset: usize) -> (u32, u32) {
        let line = self.starts.partition_point(|start| *start <= offset) - 1;
        let column: usize = self.text[self.starts[line]..offset]
            .chars()
            .map(char::len_utf16)
            .sum();
        (
            u32::try_from(line).unwrap_or(u32::MAX),
            u32::try_from(column).unwrap_or(u32::MAX),
        )
    }
}

/// `map`, made for the code `parsed`, pointed at `source` instead: `edits`,
/// last made first, map offsets in `parsed` back to `source`
pub(crate) fn remap<'a>(
    map: SourceMap<'a>,
    parsed: &str,
    source: &'a str,
    edits: &[&[Edit]],
) -> SourceMap<'a> {
    let (parsed_lines, source_lines) = (Lines::new(parsed), Lines::new(source));
    let mut parts = map.into_parts();
    parts.tokens = parts
        .tokens
        .iter()
        .map(|token| {
            let offset = edits.iter().fold(
                parsed_lines.offset(token.get_src_line(), token.get_src_col()),
                |offset, edits| source_offset(edits, offset),
            );
            let (line, column) = source_lines.position(offset);
            Token::new(
                token.get_dst_line(),
                token.get_dst_col(),
                line,
                column,
                token.get_source_id(),
                token.get_name_id(),
            )
        })
        .collect();
    parts.source_contents = vec![Some(Cow::Borrowed(source))];
    SourceMap::from_parts(parts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lines_split_as_the_codegen_does() {
        let text = "a\r\nb\rc\u{2028}d\u{2029}e\nf😀g한h";
        let lines = Lines::new(text);
        assert_eq!(lines.starts, [0, 3, 5, 9, 13, 15]);
        assert_eq!(lines.offset(5, 0), 15);
        assert_eq!(lines.offset(5, 3), 20);
        assert_eq!(lines.offset(5, 4), 21);
        assert_eq!(lines.offset(5, 99), text.len());
        assert_eq!(lines.offset(9, 0), text.len());
        assert_eq!(lines.position(20), (5, 3));
        assert_eq!(lines.position(21), (5, 4));
        assert_eq!(lines.position(9), (3, 0));
        assert_eq!(lines.position(text.len()), (5, 6));
    }
}
