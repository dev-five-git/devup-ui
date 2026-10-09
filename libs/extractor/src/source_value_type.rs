//! Source-only proofs for the value an element will assign at runtime.

use std::{cell::RefCell, collections::BTreeSet};

use oxc_ast::ast::{Expression, IdentifierReference, Program};
use oxc_span::{GetSpan, Span};
use rustc_hash::FxHashMap;

use crate::ModuleResolver;

#[path = "source_value_type_annotation.rs"]
mod annotation;
#[cfg(test)]
#[path = "source_value_type_binding_tests.rs"]
mod binding_tests;
#[path = "source_value_type_collect.rs"]
mod collect;
#[path = "source_value_type_exports.rs"]
mod exports;
#[path = "source_value_type_graph.rs"]
mod graph;
#[cfg(test)]
#[path = "source_value_type_linux_gap_tests.rs"]
mod linux_gap_tests;
#[path = "source_value_type_model.rs"]
mod model;
#[path = "source_value_type_resolve.rs"]
mod resolve;
#[path = "source_value_type_syntax.rs"]
mod syntax;
#[cfg(test)]
#[path = "source_value_type_tests.rs"]
mod tests;

/// What the source proves, without evaluating its value or loading a TS checker.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum ValueType {
    Number,
    String,
    NonNumericString,
    #[default]
    Unproven,
}

#[derive(Default)]
struct Facts {
    expressions: FxHashMap<Span, ValueType>,
    marked: FxHashMap<u32, (u32, ValueType)>,
    bound_references: FxHashMap<u32, String>,
}

thread_local! {
    static FACTS: RefCell<Facts> = RefCell::default();
}

/// Restores the outer extraction's proofs, including on early return.
pub(crate) struct Scope(Facts);

impl Scope {
    /// Capture original expressions before extraction rewrites their spans/code.
    pub(crate) fn enter(
        program: &Program<'_>,
        filename: &str,
        resolver: Option<&ModuleResolver>,
        package: &str,
    ) -> (Self, BTreeSet<String>) {
        let model = collect::model(program);
        let mut graph = graph::Graph::new(resolver, package);
        let mut facts = Facts::default();
        for (span, node) in &model.expressions {
            let proof = graph.resolve(&model, filename, node).value_type();
            facts.expressions.insert(*span, proof);
            let entry = facts.marked.entry(span.start).or_insert((span.end, proof));
            if span.end > entry.0 {
                *entry = (span.end, proof);
            }
        }
        facts.bound_references = model.bound_references;
        let scope = Self(FACTS.with_borrow_mut(|outer| std::mem::replace(outer, facts)));
        (scope, graph.dependencies)
    }
}

/// Resolve cloned lexical reads by original symbol/span facts, not spelling alone.
pub(crate) fn is_bound(identifier: &IdentifierReference<'_>) -> bool {
    FACTS.with_borrow(|facts| {
        facts
            .bound_references
            .get(&crate::provenance::source_offset(identifier.span.start))
            .is_some_and(|name| name == identifier.name.as_str())
    })
}

impl Drop for Scope {
    fn drop(&mut self) {
        FACTS.with_borrow_mut(|facts| *facts = std::mem::take(&mut self.0));
    }
}

/// Classify a source expression; generated/unresolved values remain unproven.
pub(crate) fn classify(expression: &Expression<'_>) -> ValueType {
    let span = expression.span();
    FACTS
        .with_borrow(|facts| {
            facts.expressions.get(&span).copied().or_else(|| {
                let start = crate::provenance::source_offset(span.start);
                (start != span.start)
                    .then(|| facts.marked.get(&start).map(|(_, proof)| *proof))
                    .flatten()
            })
        })
        .unwrap_or_default()
}
