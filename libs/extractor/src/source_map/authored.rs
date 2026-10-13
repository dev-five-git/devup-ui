use std::borrow::Cow;

use oxc_sourcemap::{SourceMap, Token};
use oxc_span::Span;

use super::Lines;
use crate::{
    import_alias_visit::{Edit, source_offset},
    vanilla_extract::Stylesheet,
};

/// Copied authored regions, including the edits made before retention.
#[derive(Default)]
pub(crate) struct Authored {
    edits: Vec<Vec<Edit>>,
    regions: Vec<(usize, usize, usize)>,
}

impl Authored {
    pub(crate) fn new(stylesheet: Stylesheet<'_>) -> Self {
        Self {
            edits: stylesheet
                .edits
                .iter()
                .map(|layer| layer.to_vec())
                .collect(),
            regions: Vec::new(),
        }
    }

    pub(crate) fn record(&mut self, start: usize, written: (&str, Span), code: &str) {
        let (text, span) = written;
        let copied = span.source_text(code);
        if let Some(at) = text.find(copied) {
            self.regions.push((
                start + at,
                start + at + copied.len(),
                usize::try_from(span.start).unwrap_or(usize::MAX),
            ));
        }
    }

    /// Keep generated locations on source zero and explicitly trace authored tokens on source one.
    pub(crate) fn remap<'a>(
        self,
        map: SourceMap<'a>,
        input: (&str, &'a str, usize),
    ) -> SourceMap<'a> {
        let (parsed, source, base) = input;
        let (parsed_lines, original_lines) = (Lines::new(parsed), Lines::new(source));
        let mut parts = map.into_parts();
        parts.tokens = parts
            .tokens
            .iter()
            .map(|token| {
                let offset = parsed_lines.offset(token.get_src_line(), token.get_src_col());
                let Some(local) = offset.checked_sub(base) else {
                    return *token;
                };
                let Some(&(start, _, original)) = self
                    .regions
                    .iter()
                    .find(|(start, end, _)| (*start..*end).contains(&local))
                else {
                    return *token;
                };
                let offset = self
                    .edits
                    .iter()
                    .fold(original + local - start, |offset, edits| {
                        source_offset(edits, offset)
                    });
                let (line, column) = original_lines.position(offset);
                Token::new(
                    token.get_dst_line(),
                    token.get_dst_col(),
                    line,
                    column,
                    Some(1),
                    token.get_name_id(),
                )
            })
            .collect();
        parts.sources.push(parts.sources[0].clone());
        parts.source_contents.push(Some(Cow::Borrowed(source)));
        SourceMap::from_parts(parts)
    }
}
