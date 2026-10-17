//! Reject invalid source before stripping can discard the failing program.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

use crate::import_alias_visit::source_offset;
use crate::vanilla_extract::Stylesheet;

pub(crate) fn validate(input: Stylesheet<'_>) -> Result<(), String> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(input.filename).unwrap_or_else(|_| SourceType::ts());
    let parsed = Parser::new(&allocator, input.code, source_type).parse();
    let semantic = SemanticBuilder::new()
        .with_check_syntax_error(true)
        .build(&parsed.program);
    if let Some(error) = parsed
        .diagnostics
        .iter()
        .chain(semantic.diagnostics.iter())
        .next()
    {
        let offset = error.labels.first().map_or(0, |label| {
            usize::try_from(label.offset()).unwrap_or(input.code.len())
        });
        let offset = input
            .edits
            .iter()
            .fold(offset, |at, edits| source_offset(edits, at));
        return Err(format!(
            "{}: JS execution error: SyntaxError: {error}. Fix: correct the syntax at this location",
            crate::locate(input.filename, input.source, offset)
        ));
    }
    Ok(())
}
