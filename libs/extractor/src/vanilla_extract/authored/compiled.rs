use oxc_ast::ast::{Program, Statement};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeSet;

use crate::{mutations::Use, ordinary_ve::selection, utils::unwrap_syntax_only};

pub(in crate::vanilla_extract) struct Origin<'a> {
    pub filename: &'a str,
    pub package: &'a str,
    pub resolver: Option<&'a crate::ModuleResolver>,
}

pub(super) struct Accounting {
    pub consumed: BTreeSet<(u32, u32)>,
    selected: selection::Selection,
    mutations: FxHashMap<SymbolId, Vec<Use>>,
}

impl Accounting {
    pub(super) fn is_native(&self, symbol: SymbolId) -> bool {
        self.selected
            .imports
            .iter()
            .any(|import| import.binding.symbol == symbol && import.native.is_some())
    }
    pub(super) fn new<'a>(
        parsed: (&Program<'a>, &Semantic<'a>),
        origin: &Origin<'_>,
        state: &FxHashSet<SymbolId>,
    ) -> Self {
        let (program, semantic) = parsed;
        let selected =
            selection::select_resolved(parsed, (origin.filename, origin.package), origin.resolver);
        let native: FxHashSet<_> = selected
            .imports
            .iter()
            .filter(|import| import.native.is_some())
            .map(|import| semantic.scoping().symbol_name(import.binding.symbol))
            .collect();
        let mutations = crate::mutations::resolved_uses(semantic, &|name| native.contains(name));
        let mut accounting = Self {
            consumed: BTreeSet::new(),
            selected,
            mutations,
        };
        for statement in &program.body {
            let Statement::ExpressionStatement(expression) = statement else {
                continue;
            };
            let call = unwrap_syntax_only(&expression.expression).span();
            if accounting.selected.roots.iter().any(|root| {
                root.span == statement.span()
                    && accounting.selected.native_calls.iter().any(|native| {
                        native.span == call && root.native_calls.contains(&native.node)
                    })
            }) && !accounting.changes(semantic, statement.span(), state)
            {
                accounting
                    .consumed
                    .insert((statement.span().start, statement.span().end));
            }
        }
        accounting
    }

    pub(super) fn changes(
        &self,
        semantic: &Semantic<'_>,
        span: Span,
        state: &FxHashSet<SymbolId>,
    ) -> bool {
        let mut regions = vec![span];
        let mut reached = FxHashSet::default();
        loop {
            let mut changed = super::regions::extend(semantic, &mut regions);
            for edge in &self.selected.helper_calls {
                if regions
                    .iter()
                    .any(|region| region.contains_inclusive(edge.span))
                    && reached.insert(edge.callable)
                {
                    regions.push(semantic.nodes().kind(edge.callable).span());
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        state.iter().any(|symbol| {
            semantic
                .scoping()
                .get_resolved_reference_ids(*symbol)
                .iter()
                .any(|id| {
                    let reference = semantic.scoping().get_reference(*id);
                    reference.is_write()
                        && super::regions::eager(semantic, reference.node_id(), &regions)
                })
                || self.mutations.get(symbol).is_some_and(|uses| {
                    uses.iter().any(|usage| {
                        let at = match usage {
                            Use::Changes { at, .. } | Use::Calls { at, .. } => *at,
                            Use::Escapes { at, into, .. } => {
                                if into.is_some() {
                                    return false;
                                }
                                *at
                            }
                        };
                        semantic
                            .scoping()
                            .get_resolved_reference_ids(*symbol)
                            .iter()
                            .any(|id| {
                                let reference = semantic.scoping().get_reference(*id);
                                semantic.nodes().kind(reference.node_id()).span().start == at
                                    && super::regions::eager(
                                        semantic,
                                        reference.node_id(),
                                        &regions,
                                    )
                            })
                    })
                })
        })
    }
}
