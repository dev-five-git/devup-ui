use std::ops::Range;

pub(super) fn quasi_ranges(
    raw: &str,
    source: &str,
    start: u32,
) -> Option<Vec<(Range<usize>, u32)>> {
    let start_index = usize::try_from(start).ok()?;
    let source = source.get(start_index..)?;
    let mut cursor = 0;
    let mut ranges = Vec::new();
    for (offset, character) in raw.char_indices() {
        ranges.push((
            offset..offset + character.len_utf8(),
            start.saturating_add(u32::try_from(cursor).ok()?),
        ));
        let rest = source.get(cursor..)?;
        if character == '\n' && rest.starts_with("\r\n") {
            cursor += 2;
        } else if character == '\n' && rest.starts_with('\r') {
            cursor += 1;
        } else if rest.starts_with(character) {
            cursor += character.len_utf8();
        } else {
            return None;
        }
    }
    Some(ranges)
}

pub(super) fn string_ranges(raw: &str, cooked: &str, start: u32) -> Vec<(Range<usize>, u32)> {
    let raw = &raw[1..raw.len().saturating_sub(1)];
    let bytes = raw.as_bytes();
    let mut source = 0;
    let mut ranges = Vec::new();
    for (offset, character) in cooked.char_indices() {
        while bytes.get(source) == Some(&b'\\')
            && matches!(bytes.get(source + 1), Some(b'\r' | b'\n'))
        {
            source +=
                if bytes.get(source + 1) == Some(&b'\r') && bytes.get(source + 2) == Some(&b'\n') {
                    3
                } else {
                    2
                };
        }
        ranges.push((
            offset..offset + character.len_utf8(),
            start.saturating_add(u32::try_from(source).unwrap_or(u32::MAX)),
        ));
        source += match bytes.get(source) {
            Some(b'\\') => match bytes.get(source + 1) {
                Some(b'u') if bytes.get(source + 2) == Some(&b'{') => {
                    raw[source..].find('}').map_or(2, |end| end + 1)
                }
                Some(b'u')
                    if character.len_utf8() == 4
                        && bytes.get(source + 6..source + 8) == Some(b"\\u") =>
                {
                    12
                }
                Some(b'u') => 6,
                Some(b'x') => 4,
                _ => 2,
            },
            _ => character.len_utf8(),
        };
    }
    ranges
}
