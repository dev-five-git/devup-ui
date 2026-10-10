pub(crate) struct Cursor<'a>(pub(crate) &'a [u8]);

impl<'a> Cursor<'a> {
    pub(crate) fn take(&mut self, length: usize) -> Option<&'a [u8]> {
        let (head, tail) = self.0.split_at_checked(length)?;
        self.0 = tail;
        Some(head)
    }

    pub(crate) fn byte(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }

    pub(crate) fn count(&mut self) -> Option<usize> {
        usize::try_from(u64::from_be_bytes(self.take(8)?.try_into().ok()?)).ok()
    }

    pub(crate) fn text(&mut self) -> Option<&'a str> {
        let length = self.count()?;
        std::str::from_utf8(self.take(length)?).ok()
    }
}

pub(crate) fn name_matches(content: &css::content_name::ContentName, name: &str) -> bool {
    name.rfind(|character: char| character.is_ascii_uppercase())
        .and_then(|marker| marker.checked_sub(1))
        .is_some_and(|domain| content.name(&name[..domain]) == name)
}

pub(crate) fn hex_bytes(value: &str) -> Option<Vec<u8>> {
    let digit = |byte| match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    };
    let (chunks, remainder) = value.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return None;
    }
    chunks
        .iter()
        .map(|pair| Some(digit(pair[0])? * 16 + digit(pair[1])?))
        .collect()
}
