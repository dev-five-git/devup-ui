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
        if let Some(literal) = literal
            && literal.value == specifier
        {
            offsets.push(literal.span.start);
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
                && literal.value == specifier
            {
                offsets.push(literal.span.start);
            }
        }
    }
    offsets.sort_unstable();
    offsets.dedup();
    offsets
        .into_iter()
        .filter_map(|offset| {
            let prefix = source.get(..usize::try_from(offset).ok()?)?;
            let mut location = ModuleReferenceLocation { line: 1, column: 1 };
            let mut chars = prefix.chars().peekable();
            while let Some(character) = chars.next() {
                match character {
                    '\r' => {
                        if chars.peek() == Some(&'\n') {
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
            Some(location)
        })
        .collect()
}

#[cfg(test)]
#[path = "module_reference_tests.rs"]
mod tests;
