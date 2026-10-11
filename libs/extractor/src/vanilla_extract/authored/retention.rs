use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;

use super::{Piece, analysis::Index, safety::Disposition};

pub(super) fn binding(
    input: (SymbolId, &str, &Index<'_, '_>),
    state: (
        &mut FxHashMap<SymbolId, Disposition>,
        &mut FxHashSet<String>,
    ),
    pieces: &mut BTreeMap<(u32, u32), Piece>,
) {
    let (symbol, export, index) = input;
    let (dispositions, retained) = state;
    let mut pending = vec![symbol];
    let mut visited = FxHashSet::default();
    while let Some(symbol) = pending.pop() {
        if !visited.insert(symbol) {
            continue;
        }
        let Some(units) = index.units.get(&symbol) else {
            continue;
        };
        for unit in units {
            for sibling in &unit.symbols {
                dispositions.insert(*sibling, Disposition::Retained);
                retained.insert(index.semantic.scoping().symbol_name(*sibling).to_string());
                pending.push(*sibling);
            }
            pieces
                .entry((unit.span.start, unit.span.end))
                .or_insert_with(|| Piece {
                    span: unit.span,
                    text: unit.text.clone(),
                    export: export.to_string(),
                });
        }
    }
}

pub(super) fn exports(
    index: &Index<'_, '_>,
    dispositions: &FxHashMap<SymbolId, Disposition>,
    pieces: &mut BTreeMap<(u32, u32), Piece>,
) -> FxHashSet<String> {
    let mut restored = FxHashSet::default();
    for (name, export) in &index.exports {
        if export
            .target
            .is_some_and(|symbol| matches!(dispositions.get(&symbol), Some(Disposition::Retained)))
        {
            restored.insert(name.clone());
            if !export.text.is_empty() {
                pieces
                    .entry((export.span.start, export.span.end))
                    .or_insert_with(|| Piece {
                        span: export.span,
                        text: export.text.clone(),
                        export: name.clone(),
                    });
            }
        }
    }
    restored
}
