//! The script a stylesheet is evaluated as, and where each part of it was
//! written

use std::rc::Rc;

use crate::source_map::{Lines, Trace};
#[cfg(test)]
use crate::vanilla_extract::strip_typescript;

mod explain;
mod retained;
#[cfg(test)]
mod tests;
mod written;

pub(crate) use explain::SCRIPT_PATH;

/// A module as written, and how its script traces back to it
struct Written {
    source: String,
    trace: Trace,
    retained: Option<Vec<(usize, usize)>>,
    instrumentation: Vec<(usize, bool)>,
}

/// A module turned into a script: its TypeScript stripped
pub(crate) struct Unit {
    filename: String,
    script: String,
    /// `None` for a module the build generated, which no author wrote
    written: Option<Written>,
}

impl Unit {
    #[cfg(test)]
    pub(crate) fn generated(filename: &str, code: &str) -> Rc<Self> {
        Rc::new(Self {
            filename: filename.to_string(),
            script: strip_typescript(code, filename),
            written: None,
        })
    }

    pub(crate) fn filename(&self) -> &str {
        &self.filename
    }

    pub(crate) fn script(&self) -> &str {
        &self.script
    }

    pub(crate) fn original(&self) -> &str {
        self.written
            .as_ref()
            .map_or(&self.script, |written| &written.source)
    }

    /// `filename:line:column` of the byte `offset` of the script as written,
    /// `None` where the build generated it
    fn locate(&self, offset: usize) -> Option<String> {
        let written = self.written.as_ref()?;
        if written.retained.as_ref().is_some_and(|ranges| {
            !ranges
                .iter()
                .any(|(start, end)| (*start..*end).contains(&offset))
        }) {
            return None;
        }
        let offset = written.trace.resolve(&self.script, &written.source, offset);
        Some(crate::locate(&self.filename, &written.source, offset))
    }

    /// Where the byte `offset` of the script is, down to the file for generated
    /// code
    pub(crate) fn place(&self, offset: usize) -> String {
        self.locate(offset)
            .unwrap_or_else(|| format!("{}:1:1", self.filename))
    }
}

/// A part of a module's body, up to the next one
struct Piece {
    start: usize,
    script: usize,
    /// Whether the text is the script's, so a position in it moves with it
    copied: bool,
}

/// What a module's body was made of
pub(crate) struct Origin {
    unit: Rc<Unit>,
    pieces: Vec<Piece>,
}

impl Origin {
    fn script_offset(&self, offset: usize) -> usize {
        let piece = &self.pieces[self.pieces.partition_point(|piece| piece.start <= offset) - 1];
        if piece.copied {
            piece.script + offset - piece.start
        } else {
            piece.script
        }
    }

    fn locate(&self, offset: usize) -> Option<String> {
        self.unit.locate(self.script_offset(offset))
    }

    fn frame(&self, offset: usize) -> explain::FrameOrigin {
        let script = self.script_offset(offset);
        if self.unit.written.as_ref().is_some_and(|written| {
            let at = written
                .instrumentation
                .partition_point(|(start, _)| *start <= script);
            at.checked_sub(1)
                .is_some_and(|at| written.instrumentation[at].1)
        }) {
            explain::FrameOrigin::Instrumentation
        } else {
            self.locate(offset).map_or(
                explain::FrameOrigin::Generated,
                explain::FrameOrigin::Authored,
            )
        }
    }
}

/// The body of a module, built from its script and the code made to go with it
pub(crate) struct Body {
    text: String,
    pieces: Vec<Piece>,
}

impl Body {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            text: String::with_capacity(capacity),
            pieces: vec![Piece {
                start: 0,
                script: 0,
                copied: false,
            }],
        }
    }

    /// Adds `script[from..to]`
    pub(crate) fn copy(&mut self, script: &str, from: usize, to: usize) {
        self.pieces.push(Piece {
            start: self.text.len(),
            script: from,
            copied: true,
        });
        self.text.push_str(&script[from..to]);
    }

    /// Adds `text` made for the code at byte `at` of the script
    pub(crate) fn synthesize(&mut self, at: usize, text: &str) {
        self.pieces.push(Piece {
            start: self.text.len(),
            script: at,
            copied: false,
        });
        self.text.push_str(text);
    }

    pub(crate) fn finish(self, unit: &Rc<Unit>) -> (String, Rc<Origin>) {
        (
            self.text,
            Rc::new(Origin {
                unit: unit.clone(),
                pieces: self.pieces,
            }),
        )
    }
}

#[cfg(test)]
mod coverage_tests;

/// The script an evaluation runs: the loaded modules, then the stylesheet
#[derive(Default)]
pub(crate) struct Script {
    pub text: String,
    /// `(start, end, origin)` of the bodies of modules in `text`; the rest is
    /// code the build generated
    parts: Vec<(usize, usize, Rc<Origin>)>,
}

impl Script {
    pub(crate) fn authored(&self, offset: usize) -> bool {
        self.parts.iter().any(|(start, end, origin)| {
            (*start..*end).contains(&offset)
                && matches!(
                    origin.frame(offset - start),
                    explain::FrameOrigin::Authored(_)
                )
        })
    }

    pub(crate) fn generated(&mut self, text: &str) {
        self.text.push_str(text);
    }

    pub(crate) fn body(&mut self, text: &str, origin: &Rc<Origin>) {
        let start = self.text.len();
        self.text.push_str(text);
        self.parts.push((start, self.text.len(), origin.clone()));
    }

    /// `filename:line:column` of the script as written for a position the
    /// engine printed, `None` in code the build generated
    fn frame(&self, lines: &Lines<'_>, line: u32, column: u32) -> explain::FrameOrigin {
        let offset = lines.code_point_offset(line.saturating_sub(1), column.saturating_sub(1));
        let Some((start, _, origin)) = self
            .parts
            .iter()
            .find(|(start, end, _)| (*start..*end).contains(&offset))
        else {
            return explain::FrameOrigin::Generated;
        };
        origin.frame(offset - start)
    }

    /// The error the engine gave for this script, its positions in the code as
    /// written; `fallback` names the file when no position is
    pub(crate) fn explain(&self, error: &str, fallback: &str) -> String {
        let lines = Lines::new(&self.text);
        if self.parts.is_empty() {
            return explain::describe(error, |_, _| None, &format!("{fallback}:1:1"));
        }
        explain::describe_frames(
            error,
            |line, column| self.frame(&lines, line, column),
            &format!("{fallback}:1:1"),
        )
    }
}
