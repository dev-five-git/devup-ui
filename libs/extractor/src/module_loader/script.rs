//! The script a stylesheet is evaluated as, and where each part of it was
//! written

use std::rc::Rc;

use crate::import_alias_visit::Edit;
use crate::source_map::{Lines, Trace};
#[cfg(test)]
use crate::vanilla_extract::strip_typescript;
use crate::vanilla_extract::strip_typescript_marked;

mod explain;
mod retained;
#[cfg(test)]
mod tests;

pub(crate) use explain::SCRIPT_PATH;

/// A module as written, and how its script traces back to it
struct Written {
    source: String,
    trace: Trace,
    retained: Option<Vec<(usize, usize)>>,
}

/// A module turned into a script: its TypeScript stripped
pub(crate) struct Unit {
    filename: String,
    script: String,
    /// `None` for a module the build generated, which no author wrote
    written: Option<Written>,
}

impl Unit {
    /// The script of `code`, the `source` of `filename` after the layers of
    /// `edits`, last made first
    pub(crate) fn written(
        filename: &str,
        code: &str,
        source: &str,
        edits: &[&[Edit]],
    ) -> Result<Rc<Self>, String> {
        super::validate(crate::vanilla_extract::Stylesheet {
            filename,
            code,
            source,
            edits,
        })?;
        let stripped = strip_typescript_marked(code, filename);
        Ok(Rc::new(Self {
            filename: filename.to_string(),
            script: stripped.code.clone(),
            written: Some(Written {
                source: source.to_string(),
                trace: Trace::new(&stripped.marks, edits),
                retained: None,
            }),
        }))
    }

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
    fn locate(&self, offset: usize) -> Option<String> {
        let piece = &self.pieces[self.pieces.partition_point(|piece| piece.start <= offset) - 1];
        let script = if piece.copied {
            piece.script + offset - piece.start
        } else {
            piece.script
        };
        self.unit.locate(script)
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
    fn locate(&self, lines: &Lines<'_>, line: u32, column: u32) -> Option<String> {
        let offset = lines.code_point_offset(line.saturating_sub(1), column.saturating_sub(1));
        let (start, _, origin) = self
            .parts
            .iter()
            .find(|(start, end, _)| (*start..*end).contains(&offset))?;
        origin.locate(offset - start)
    }

    /// The error the engine gave for this script, its positions in the code as
    /// written; `fallback` names the file when no position is
    pub(crate) fn explain(&self, error: &str, fallback: &str) -> String {
        let lines = Lines::new(&self.text);
        explain::describe(
            error,
            |line, column| self.locate(&lines, line, column),
            &format!("{fallback}:1:1"),
        )
    }
}
