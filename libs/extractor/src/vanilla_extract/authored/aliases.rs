use oxc_ast::ast::{Program, Statement};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;

use super::alias_candidates::candidates;
use super::{Piece, analysis::Index, safety::Disposition};
use crate::vanilla_extract::Stylesheet;

pub(super) fn retain(
    input: (Stylesheet<'_>, &Program<'_>, &Index<'_, '_>),
    state: (
        &mut FxHashMap<SymbolId, Disposition>,
        &mut FxHashSet<String>,
    ),
    output: (
        &mut BTreeMap<(u32, u32), Piece>,
        &super::compiled::Accounting,
    ),
) -> Result<(), String> {
    let (stylesheet, program, index) = input;
    let (dispositions, retained) = state;
    let (pieces, accounting) = output;
    let candidates = candidates(index, dispositions);
    let mut exact: FxHashSet<_> = dispositions
        .iter()
        .filter_map(|(symbol, disposition)| {
            matches!(disposition, Disposition::Retained).then_some(*symbol)
        })
        .collect();
    let mut possible = exact.clone();
    loop {
        let mut changed = false;
        for candidate in &candidates {
            if candidate
                .target
                .is_some_and(|target| exact.contains(&target))
            {
                changed |= exact.insert(candidate.symbol);
            }
            if candidate
                .inputs
                .iter()
                .any(|input| possible.contains(input))
            {
                changed |= possible.insert(candidate.symbol);
            }
        }
        if !changed {
            break;
        }
    }
    let mut effects: Vec<_> = program
        .body
        .iter()
        .filter(|statement| {
            let span = statement.span();
            statement.as_declaration().is_none()
                && !accounting.consumed.contains(&(span.start, span.end))
                && !matches!(
                    statement,
                    Statement::ImportDeclaration(_)
                        | Statement::ExportDeclaration(_)
                        | Statement::ExportNamedDeclaration(_)
                        | Statement::ExportDefaultDeclaration(_)
                        | Statement::ExportFromDeclaration(_)
                        | Statement::ExportAllDeclaration(_)
                )
        })
        .map(GetSpan::span)
        .collect();
    effects.extend(index.units.values().flatten().filter_map(|unit| {
        accounting
            .changes(index.semantic, unit.span, &possible)
            .then_some(unit.span)
    }));
    let mut required: FxHashSet<_> = candidates
        .iter()
        .filter(|candidate| {
            possible.contains(&candidate.symbol)
                && index
                    .semantic
                    .scoping()
                    .get_resolved_reference_ids(candidate.symbol)
                    .iter()
                    .any(|id| {
                        let reference = index.semantic.scoping().get_reference(*id);
                        let span = index.semantic.nodes().kind(reference.node_id()).span();
                        reference.is_value()
                            && (effects.iter().any(|effect| effect.contains_inclusive(span))
                                || pieces
                                    .values()
                                    .any(|piece| piece.span.contains_inclusive(span)))
                    })
        })
        .map(|candidate| candidate.symbol)
        .collect();
    loop {
        let mut changed = false;
        for candidate in &candidates {
            if required.contains(&candidate.symbol) {
                for input in &candidate.inputs {
                    if possible.contains(input) {
                        changed |= required.insert(*input);
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut errors = Vec::new();
    for candidate in &candidates {
        if !required.contains(&candidate.symbol) {
            continue;
        }
        let name = index.semantic.scoping().symbol_name(candidate.symbol);
        if !exact.contains(&candidate.symbol) {
            errors.push((candidate.site.start, format!(
                "ordinary alias `{name}` cannot use `{}` at build time: retained-state setup is not an exact const binding alias. Fix: use a const alias of the retained binding before mutating it",
                candidate.site.source_text(index.code)
            )));
            continue;
        }
        super::retention::binding(
            (candidate.symbol, name, index),
            (dispositions, retained),
            pieces,
        );
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(crate::located_errors(
            stylesheet.filename,
            stylesheet.source,
            stylesheet.edits,
            errors,
        ))
    }
}

pub(super) fn related(
    index: &Index<'_, '_>,
    dispositions: &FxHashMap<SymbolId, Disposition>,
) -> FxHashSet<SymbolId> {
    let candidates = candidates(index, dispositions);
    let mut related: FxHashSet<_> = dispositions
        .iter()
        .filter_map(|(symbol, disposition)| {
            matches!(disposition, Disposition::Retained).then_some(*symbol)
        })
        .collect();
    loop {
        let mut changed = false;
        for candidate in &candidates {
            if candidate.inputs.iter().any(|input| related.contains(input)) {
                changed |= related.insert(candidate.symbol);
            }
        }
        if !changed {
            return related;
        }
    }
}
