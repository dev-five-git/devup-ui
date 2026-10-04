//! Offsets of generated text, traced back to the text it was made from

use oxc_sourcemap::SourceMap;

use super::Lines;
use crate::import_alias_visit::{Edit, source_offset};

#[cfg(test)]
mod tests;

/// `(offset in generated, offset in parsed)` for every token of `map`, the map
/// of `generated` made from `parsed`; in ascending order
pub(crate) fn marks(map: &SourceMap<'_>, generated: &str, parsed: &str) -> Vec<(usize, usize)> {
    let (generated_lines, parsed_lines) = (Lines::new(generated), Lines::new(parsed));
    map.get_tokens()
        .map(|token| {
            (
                generated_lines.offset(token.get_dst_line(), token.get_dst_col()),
                parsed_lines.offset(token.get_src_line(), token.get_src_col()),
            )
        })
        .collect()
}

/// Where the text of a stage came from in the source as written
pub(crate) struct Trace {
    /// `(offset in generated, offset in source)`, ascending, the start of the
    /// generated text first
    marks: Vec<(usize, usize)>,
}

impl Trace {
    /// `marks` tie the generated text to the code it was parsed from; each layer
    /// of `edits`, last made first, maps that code back to the source
    pub(crate) fn new(marks: &[(usize, usize)], edits: &[&[Edit]]) -> Self {
        let back = |offset| {
            edits
                .iter()
                .fold(offset, |offset, edits| source_offset(edits, offset))
        };
        Self {
            marks: std::iter::once((0, back(0)))
                .chain(
                    marks
                        .iter()
                        .map(|&(generated, parsed)| (generated, back(parsed))),
                )
                .collect(),
        }
    }

    /// The offset in `source` of the byte `offset` of `generated`: the one of
    /// the closest mark before it, moved on by the text both share after it
    pub(crate) fn resolve(&self, generated: &str, source: &str, offset: usize) -> usize {
        let at = self.marks.partition_point(|(from, _)| *from <= offset);
        let (from, to) = self.marks[at - 1];
        let shared = generated.as_bytes().get(from..offset).unwrap_or_default();
        if source.as_bytes().get(to..to + shared.len()) == Some(shared) {
            to + shared.len()
        } else {
            to
        }
    }
}
