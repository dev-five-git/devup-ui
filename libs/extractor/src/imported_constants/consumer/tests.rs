use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::SourceType;

use super::ReadPlan;

mod guards;
mod invariants;
mod slots;

pub(super) fn parsed(source: &str, check: impl FnOnce(&Program<'_>, &Semantic<'_>)) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    check(&parsed.program, &built.semantic);
}

fn planned(source: &str) -> ReadPlan {
    let mut result = None;
    parsed(source, |program, semantic| {
        result = Some(super::plan(
            program,
            semantic,
            &crate::ExtractOption::default(),
        ));
    });
    result.unwrap_or_else(|| panic!("parser callback did not produce a plan"))
}
