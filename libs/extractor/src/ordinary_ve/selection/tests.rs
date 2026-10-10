use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};

use super::{Selection, plan::Unit};

mod helpers;
mod namespace_audit;
mod native_policy;
mod owners;
mod provenance;

fn selected(source: &str) -> Selection {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    super::select(&parsed.program, &built.semantic)
}

fn names(units: &[Unit]) -> Vec<&str> {
    units
        .iter()
        .flat_map(|unit| unit.bindings.iter().map(|binding| binding.name.as_str()))
        .collect()
}

fn text(source: &str, span: Span) -> &str {
    span.source_text(source)
}
