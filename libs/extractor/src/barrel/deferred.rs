use std::cell::RefCell;

/// A render-only read whose actual native target still needs proof.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeferredDefault {
    /// Original extraction and root importer, before source rewriting.
    pub extraction: String,
    /// Original request whose selected forwarding path must be verified.
    pub request: String,
    /// Export path read from the original request.
    pub exports: Vec<String>,
    /// Original one-based source line.
    pub line: usize,
    /// Original one-based UTF-16 source column.
    pub column: usize,
    /// Semantic demand established during existing barrel analysis.
    pub demand: RenderDemand,
    /// Import attributes are retained as an unsupported admission condition.
    pub attributes: bool,
    /// Importer at the synchronous traversal's deferred cut.
    pub cut_importer: String,
    /// Request at the deferred cut, not a native target certificate.
    pub cut_request: String,
    /// Export deliberately left for native verification.
    pub cut_export: String,
}

/// Only semantic component-position reads may be deferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderDemand {
    RenderOnly,
}

pub(super) struct RootRead {
    pub request: String,
    pub exports: Vec<String>,
    pub position: usize,
    pub attributes: bool,
}

pub(crate) struct DeferredPolicy<'a> {
    pub extensions: &'a [String],
    pub records: &'a RefCell<Vec<DeferredDefault>>,
}

impl RootRead {
    pub(super) fn defer(&self, source: (&str, &str), cut: (&str, &str)) -> DeferredDefault {
        let (extraction, code) = source;
        let prefix = &code[..self.position];
        let (mut line, mut column) = (1, 1);
        let mut previous_cr = false;
        for character in prefix.chars() {
            match character {
                '\n' if previous_cr => {}
                '\r' | '\n' | '\u{2028}' | '\u{2029}' => {
                    line += 1;
                    column = 1;
                }
                _ => column += character.len_utf16(),
            }
            previous_cr = character == '\r';
        }
        DeferredDefault {
            extraction: extraction.to_string(),
            request: self.request.clone(),
            exports: self.exports.clone(),
            line,
            column,
            demand: RenderDemand::RenderOnly,
            attributes: self.attributes,
            cut_importer: cut.0.to_string(),
            cut_request: cut.1.to_string(),
            cut_export: "default".to_string(),
        }
    }
}
