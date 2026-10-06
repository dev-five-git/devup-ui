//! Copied selections extend the existing stripping trace, before Body/Origin.

use oxc_span::Span;

use super::{Rc, Trace, Unit, Written, strip_typescript_marked};
use crate::vanilla_extract::Stylesheet;

struct Piece {
    start: usize,
    original: usize,
    copied: bool,
}

/// Evaluation-only text. Synthesized syntax is anchored, copied bytes translate.
#[derive(Default)]
pub(crate) struct Mapped {
    pub code: String,
    pieces: Vec<Piece>,
}

impl Mapped {
    pub(crate) fn copy(&mut self, source: &str, span: Span) {
        self.pieces.push(Piece {
            start: self.code.len(),
            original: usize::try_from(span.start).unwrap_or(source.len()),
            copied: true,
        });
        self.code.push_str(span.source_text(source));
    }

    pub(crate) fn synthesize(&mut self, at: u32, text: &str) {
        self.pieces.push(Piece {
            start: self.code.len(),
            original: usize::try_from(at).unwrap_or(usize::MAX),
            copied: false,
        });
        self.code.push_str(text);
    }

    fn original(&self, offset: usize) -> usize {
        let at = self.pieces.partition_point(|piece| piece.start <= offset);
        let Some(piece) = at.checked_sub(1).and_then(|at| self.pieces.get(at)) else {
            return 0;
        };
        if piece.copied {
            piece.original + offset - piece.start
        } else {
            piece.original
        }
    }
}

impl Unit {
    /// Strip a valid selected program and compose its copied ranges with every
    /// earlier source-edit layer. The producer keeps its real module identity.
    pub(crate) fn selected(
        stylesheet: Stylesheet<'_>,
        mapped: &Mapped,
    ) -> Result<Rc<Self>, String> {
        super::super::validate::check(stylesheet.filename, &mapped.code, |offset| {
            let offset = stylesheet
                .edits
                .iter()
                .fold(mapped.original(offset), |offset, edits| {
                    crate::import_alias_visit::source_offset(edits, offset)
                });
            crate::locate(stylesheet.filename, stylesheet.source, offset)
        })?;
        let stripped = strip_typescript_marked(&mapped.code, stylesheet.filename);
        let marks: Vec<_> = stripped
            .marks
            .iter()
            .map(|&(generated, selected)| (generated, mapped.original(selected)))
            .collect();
        Ok(Rc::new(Self {
            filename: stylesheet.filename.to_string(),
            script: stripped.code.clone(),
            written: Some(Written {
                source: stylesheet.source.to_string(),
                trace: Trace::new(&marks, stylesheet.edits),
                retained: None,
            }),
        }))
    }
}
