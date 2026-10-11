//! Current-extraction dependency outputs, separate from the entry's style set.
use std::{error::Error, rc::Rc};

use crate::{ExtractOption, ExtractOutput, ModuleResolver};

#[cfg(test)]
mod tests;

/// An already compiled dependency under its original, uncanonicalized owner.
#[derive(Debug)]
pub struct StylesheetArtifact {
    /// The actual resolver or native producer filename.
    pub filename: String,
    /// Native code here is lowered styling code, not an authored carrier module.
    pub output: ExtractOutput,
}

/// One successful extraction, with dependency completion order preserved.
#[derive(Debug)]
pub struct ExtractGraphOutput {
    /// The existing per-file output, without dependency-style union.
    pub entry: ExtractOutput,
    /// Immutable extraction-local handles; the entry is never included.
    pub artifacts: Vec<Rc<StylesheetArtifact>>,
}

/// Extracts an entry and retains its successfully compiled dependency outputs.
///
/// # Errors
/// Returns the original located extraction failure, including incompatible
/// duplicate-owner payloads. No partial graph is returned.
pub fn extract_graph(
    filename: &str,
    code: &str,
    option: ExtractOption,
    source_map: bool,
    resolver: Option<&ModuleResolver>,
) -> Result<ExtractGraphOutput, Box<dyn Error>> {
    let result = crate::extract_source(filename, code, None, false, option, source_map, resolver)?;
    Ok(ExtractGraphOutput {
        entry: result.output,
        artifacts: result.artifacts.into_items(filename),
    })
}

#[derive(Clone, Default)]
pub(crate) struct Artifacts {
    items: Vec<Rc<StylesheetArtifact>>,
}

impl Artifacts {
    pub(crate) fn insert(&mut self, artifact: Rc<StylesheetArtifact>) -> Result<(), String> {
        if let Some(previous) = self
            .items
            .iter()
            .find(|item| item.filename == artifact.filename)
        {
            let old = &previous.output;
            let new = &artifact.output;
            if old.styles != new.styles
                || old.code != new.code
                || old.map != new.map
                || old.css_file != new.css_file
                || old.dependencies != new.dependencies
            {
                return Err(format!(
                    "{}:1:1: incompatible compiled outputs for one raw stylesheet owner. Fix: retain one immutable source and initialization schedule per extraction",
                    artifact.filename
                ));
            }
            return Ok(());
        }
        self.items.push(artifact);
        Ok(())
    }

    pub(crate) fn merge(&mut self, artifacts: Self) -> Result<(), String> {
        for artifact in artifacts.items {
            self.insert(artifact)?;
        }
        Ok(())
    }

    fn into_items(self, entry: &str) -> Vec<Rc<StylesheetArtifact>> {
        self.items
            .into_iter()
            .filter(|item| item.filename != entry)
            .collect()
    }
}
