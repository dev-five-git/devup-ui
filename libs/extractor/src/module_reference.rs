use std::collections::HashMap;

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{Argument, Statement},
};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

/// An original module reference's one-based source coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleReferenceLocation {
    pub line: usize,
    pub column: usize,
}

/// Locate all matching imports, re-exports and unshadowed literal `require` calls.
///
/// Positions point at the original specifier literal, in source order. Empty
/// results mean no matching AST reference exists, not a synthetic `1:1` site.
#[must_use]
pub fn original_module_reference_locations(
    filename: &str,
    source: &str,
    specifier: &str,
) -> Vec<ModuleReferenceLocation> {
    original_module_references(filename, source)
        .remove(specifier)
        .unwrap_or_default()
}

/// The locations of every module the source references, by specifier, from one
/// parse: what `original_module_reference_locations` finds for each of them.
#[must_use]
pub fn original_module_references(
    filename: &str,
    source: &str,
) -> HashMap<String, Vec<ModuleReferenceLocation>> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename).unwrap_or_else(|_| SourceType::ts());
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let program = parsed.program;
    let mut offsets = Vec::new();
    for statement in &program.body {
        let literal = match statement {
            Statement::ImportDeclaration(import) => Some(&import.source),
            Statement::ExportFromDeclaration(export) => Some(&export.source),
            Statement::ExportAllDeclaration(export) => Some(&export.source),
            _ => None,
        };
        if let Some(literal) = literal {
            offsets.extend(
                literal
                    .value
                    .as_str()
                    .map(|value| (literal.span.start, value)),
            );
        }
    }
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&program)
        .semantic;
    if let Some(references) = semantic
        .scoping()
        .root_unresolved_references()
        .get("require")
    {
        for reference in references {
            let node = semantic.scoping().get_reference(*reference).node_id();
            if let AstKind::CallExpression(call) = semantic.nodes().parent_kind(node)
                && let [Argument::StringLiteral(literal)] = call.arguments.as_slice()
            {
                offsets.extend(
                    literal
                        .value
                        .as_str()
                        .map(|value| (literal.span.start, value)),
                );
            }
        }
    }
    offsets.sort_unstable();
    offsets.dedup();
    let mut references = HashMap::<String, Vec<ModuleReferenceLocation>>::new();
    let mut location = ModuleReferenceLocation { line: 1, column: 1 };
    let mut chars = source.char_indices().peekable();
    for (offset, specifier) in offsets {
        let offset = usize::try_from(offset).unwrap_or(usize::MAX);
        while let Some(&(index, character)) = chars.peek() {
            if index >= offset {
                break;
            }
            chars.next();
            match character {
                '\r' => {
                    if chars.peek().is_some_and(|&(_, next)| next == '\n') {
                        chars.next();
                    }
                    location.line += 1;
                    location.column = 1;
                }
                '\n' | '\u{2028}' | '\u{2029}' => {
                    location.line += 1;
                    location.column = 1;
                }
                _ => location.column += 1,
            }
        }
        references
            .entry(specifier.to_string())
            .or_default()
            .push(location);
    }
    references
}

#[cfg(test)]
#[path = "module_reference_tests.rs"]
mod tests;
