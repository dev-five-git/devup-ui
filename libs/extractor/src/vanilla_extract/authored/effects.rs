use oxc_ast::ast::{Program, Statement};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;

use super::{Piece, analysis::Index, compiled::Accounting, retention, safety::Disposition};

pub(super) fn retain(
    input: (&Program<'_>, &Index<'_, '_>, &Accounting),
    state: (
        &mut FxHashMap<SymbolId, Disposition>,
        &mut FxHashSet<String>,
        &FxHashSet<SymbolId>,
    ),
    pieces: &mut BTreeMap<(u32, u32), Piece>,
) {
    let (program, index, accounting) = input;
    let (dispositions, retained, related) = state;
    for units in index.units.values() {
        for unit in units {
            let eager = index.semantic.nodes().iter().any(|node| {
                matches!(node.kind(),
                    oxc_ast::AstKind::VariableDeclarator(declarator) if declarator.span == unit.span
                )
            });
            if eager
                && accounting.changes(index.semantic, unit.span, related)
                && let Some(symbol) = unit.symbols.first()
            {
                let name = index.semantic.scoping().symbol_name(*symbol);
                retention::binding((*symbol, name, index), (dispositions, retained), pieces);
            }
        }
    }
    for statement in &program.body {
        if statement.as_declaration().is_some()
            || matches!(
                statement,
                Statement::ImportDeclaration(_)
                    | Statement::ExportDeclaration(_)
                    | Statement::ExportNamedDeclaration(_)
                    | Statement::ExportDefaultDeclaration(_)
                    | Statement::ExportFromDeclaration(_)
                    | Statement::ExportAllDeclaration(_)
            )
        {
            continue;
        }
        let span = statement.span();
        if accounting.consumed.contains(&(span.start, span.end)) {
            continue;
        }
        let export = related
            .iter()
            .filter_map(|symbol| {
                index
                    .semantic
                    .scoping()
                    .get_resolved_reference_ids(*symbol)
                    .iter()
                    .find_map(|id| {
                        let reference = index.semantic.scoping().get_reference(*id);
                        (reference.is_value()
                            && span.contains_inclusive(
                                index.semantic.nodes().kind(reference.node_id()).span(),
                            ))
                        .then(|| index.semantic.scoping().symbol_name(*symbol).to_string())
                    })
            })
            .min();
        if let Some(export) = export {
            pieces
                .entry((span.start, span.end))
                .or_insert_with(|| Piece {
                    span,
                    text: span.source_text(index.code).to_string(),
                    export,
                });
        }
    }
}
