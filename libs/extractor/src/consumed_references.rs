//! Semantic references removed by successful constant inlining.

use oxc_ast::ast::{Expression, IdentifierReference};
use oxc_ast_visit::Visit;
use oxc_semantic::Scoping;
use oxc_syntax::{reference::ReferenceId, symbol::SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

/// Counts erased references once, even when a retained AST node is revisited.
pub(super) struct ConsumedReferences<'s> {
    scoping: &'s Scoping,
    seen: FxHashSet<ReferenceId>,
    by_symbol: FxHashMap<SymbolId, usize>,
}

impl<'s> ConsumedReferences<'s> {
    pub(super) fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            seen: FxHashSet::default(),
            by_symbol: FxHashMap::default(),
        }
    }

    pub(super) fn count(&self, symbol: SymbolId) -> usize {
        self.by_symbol.get(&symbol).copied().unwrap_or(0)
    }

    /// Call only for a subtree about to be erased, never for a folding probe.
    pub(super) fn consume(&mut self, expression: &Expression<'_>) {
        self.visit_expression(expression);
    }
}

impl<'a> Visit<'a> for ConsumedReferences<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if let Some(reference) = identifier.reference_id.get()
            && let Some(symbol) = self.scoping.get_reference(reference).symbol_id()
            && self.seen.insert(reference)
        {
            *self.by_symbol.entry(symbol).or_default() += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_ast::ast::Statement;

    #[test]
    fn references_count_once_when_a_computed_subtree_is_revisited() {
        // Given: two distinct references to one binding in a computed member.
        let allocator = oxc_allocator::Allocator::default();
        let program = oxc_parser::Parser::new(
            &allocator,
            "const holder = {}; holder[holder.key];",
            oxc_span::SourceType::ts(),
        )
        .parse()
        .program;
        let scoping = oxc_semantic::SemanticBuilder::new()
            .build(&program)
            .semantic
            .into_scoping();
        let symbol = scoping
            .symbol_ids()
            .find(|symbol| scoping.symbol_name(*symbol) == "holder")
            .unwrap_or_else(|| panic!("fixture must declare holder"));
        let Some(Statement::ExpressionStatement(statement)) = program.body.last() else {
            panic!("fixture must end with the computed read");
        };
        let mut references = ConsumedReferences::new(&scoping);
        // When: the same semantic subtree is accounted more than once.
        for _ in 0..2 {
            references.consume(&statement.expression);
        }
        // Then: distinct operands count, repeated visits do not.
        assert_eq!(references.count(symbol), 2);
    }
}
