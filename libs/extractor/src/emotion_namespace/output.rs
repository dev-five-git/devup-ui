use super::{Allocator, Edit, Parser, SourceType};
use crate::{ExtractOutput, source_map};
use oxc_codegen::{Codegen, CodegenOptions};
use rustc_hash::FxHashSet;
use std::path::PathBuf;

/// A module the namespace pass rewrote but that has nothing for Devup UI to
/// extract. `code` is the rewritten module; `edits`, last made first, map it
/// back to `source`.
pub(crate) struct Rewritten<'a> {
    pub filename: &'a str,
    pub code: &'a str,
    pub source: &'a str,
    pub edits: &'a [&'a [Edit]],
}

impl Rewritten<'_> {
    /// Generated from the rewritten module, so the map points at `source`.
    pub(crate) fn output(&self) -> ExtractOutput {
        let allocator = Allocator::default();
        let program = Parser::new(
            &allocator,
            self.code,
            SourceType::from_path(self.filename).unwrap_or_default(),
        )
        .parse()
        .program;
        let result = Codegen::new()
            .with_options(CodegenOptions {
                source_map_path: Some(PathBuf::from(self.filename)),
                ..Default::default()
            })
            .build(&program);
        ExtractOutput {
            styles: FxHashSet::default(),
            code: result.code,
            map: result.map.map(|map| {
                source_map::remap(map, self.code, self.source, self.edits).to_json_string()
            }),
            css_file: None,
            dependencies: Vec::new(),
        }
    }
}
