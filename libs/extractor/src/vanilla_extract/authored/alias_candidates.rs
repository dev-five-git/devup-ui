use oxc_ast::{
    AstKind,
    ast::{Expression, VariableDeclarationKind},
};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{analysis::Index, safety::Disposition};
use crate::utils::unwrap_syntax_only;

pub(super) struct Candidate {
    pub symbol: SymbolId,
    pub target: Option<SymbolId>,
    pub inputs: FxHashSet<SymbolId>,
    pub site: Span,
}

pub(super) fn candidates(
    index: &Index<'_, '_>,
    dispositions: &FxHashMap<SymbolId, Disposition>,
) -> Vec<Candidate> {
    let scoping = index.semantic.scoping();
    let mut candidates = Vec::new();
    for (symbol, units) in &index.units {
        let Some(unit) = units.first() else { continue };
        if !matches!(dispositions.get(symbol), Some(Disposition::Removed)) {
            continue;
        }
        let declaration = scoping.symbol_declaration(*symbol);
        let declarator = std::iter::once(declaration)
            .chain(index.semantic.nodes().ancestor_ids(declaration))
            .find_map(|node| match index.semantic.nodes().kind(node) {
                AstKind::VariableDeclarator(declarator) if declarator.span == unit.span => {
                    Some(declarator)
                }
                _ => None,
            });
        let target = declarator.and_then(|declarator| {
            if units.len() != 1
                || declarator.id.get_binding_identifier().is_none_or(|id| id.symbol_id.get() != Some(*symbol))
                || !matches!(index.semantic.nodes().parent_kind(declarator.node_id.get()),
                    AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const)
            { return None; }
            let Expression::Identifier(identifier) = unwrap_syntax_only(declarator.init.as_ref()?) else { return None };
            identifier.reference_id.get().and_then(|id| scoping.get_reference(id).symbol_id())
        });
        let site = declarator
            .and_then(|declarator| declarator.init.as_ref().map(GetSpan::span))
            .unwrap_or(unit.span);
        let inputs = dispositions
            .keys()
            .filter(|input| {
                scoping
                    .get_resolved_reference_ids(**input)
                    .iter()
                    .any(|id| {
                        let reference = scoping.get_reference(*id);
                        reference.is_value()
                            && units.iter().any(|unit| {
                                unit.span.contains_inclusive(
                                    index.semantic.nodes().kind(reference.node_id()).span(),
                                )
                            })
                    })
            })
            .copied()
            .collect();
        candidates.push(Candidate {
            symbol: *symbol,
            target,
            inputs,
            site,
        });
    }
    candidates.sort_by_key(|candidate| candidate.site.start);
    candidates
}
