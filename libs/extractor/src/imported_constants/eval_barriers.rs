//! Direct lexical eval is an opaque effect, never code for the build to execute.

use super::eval_identity::eval_callee;
use super::provenance::{Proof, Shape};
use crate::{css_prop::binding_of, utils::unwrap_syntax_only};
use oxc_ast::{
    AstKind,
    ast::{Expression, IdentifierReference, Program, Statement},
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

struct Barrier {
    at: u32,
    visible: FxHashSet<SymbolId>,
    deferred: bool,
}

pub(crate) struct EvalBarriers {
    barriers: Vec<Barrier>,
    deferred: Vec<Span>,
}

impl EvalBarriers {
    pub(crate) fn globals(proof: &Proof<'_, '_>, span: Span) -> Option<u32> {
        let deferred = proof.nodes.iter().any(|node| {
            matches!(
                node.kind(),
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) | AstKind::Class(_)
            ) && node.kind().span().contains_inclusive(span)
        });
        proof
            .nodes
            .iter()
            .filter_map(|node| {
                let AstKind::CallExpression(call) = node.kind() else {
                    return None;
                };
                if !eval_callee(proof, &call.callee, &mut FxHashSet::default()) {
                    return None;
                }
                let hazard_deferred = proof.nodes.ancestor_kinds(node.id()).any(|kind| {
                    matches!(
                        kind,
                        AstKind::Function(_)
                            | AstKind::ArrowFunctionExpression(_)
                            | AstKind::Class(_)
                    )
                });
                (hazard_deferred || deferred || call.span.start < span.end)
                    .then_some(call.span.start)
            })
            .min()
    }

    pub(crate) fn new(program: &Program<'_>) -> Self {
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(program)
            .semantic;
        let scoping = semantic.scoping();
        let nodes = semantic.nodes();
        let mut barriers = Vec::new();
        let mut deferred = Vec::new();
        for node in nodes.iter() {
            match node.kind() {
                AstKind::Function(_) | AstKind::ArrowFunctionExpression(_) | AstKind::Class(_) => {
                    deferred.push(node.kind().span());
                }
                AstKind::CallExpression(call) => {
                    let proof = Proof { nodes, scoping };
                    if !eval_callee(&proof, &call.callee, &mut FxHashSet::default()) {
                        continue;
                    }
                    let direct = !call.optional
                        && matches!(unwrap_syntax_only(&call.callee),
                        Expression::Identifier(callee) if callee.name == "eval" && binding_of(scoping, callee).is_none());
                    let mut names = FxHashSet::default();
                    let mut visible = FxHashSet::default();
                    if direct {
                        for scope in scoping.scope_ancestors(node.scope_id()) {
                            for (name, symbol) in scoping.get_bindings(scope) {
                                if names.insert(name.as_str()) {
                                    visible.insert(*symbol);
                                }
                            }
                        }
                        expand_reachable(&proof, &mut visible);
                    }
                    let is_deferred = nodes.ancestor_kinds(node.id()).any(|kind| {
                        matches!(
                            kind,
                            AstKind::Function(_)
                                | AstKind::ArrowFunctionExpression(_)
                                | AstKind::Class(_)
                        )
                    });
                    barriers.push(Barrier {
                        at: call.span.start,
                        visible,
                        deferred: is_deferred,
                    });
                }
                _ => {}
            }
        }
        Self { barriers, deferred }
    }

    pub(crate) fn affects(&self, symbol: Option<SymbolId>, span: Span) -> Option<u32> {
        let deferred = self
            .deferred
            .iter()
            .any(|body| body.contains_inclusive(span));
        self.barriers
            .iter()
            .filter(|barrier| {
                (barrier.deferred || deferred || barrier.at < span.end)
                    && symbol.is_none_or(|symbol| barrier.visible.contains(&symbol))
            })
            .map(|barrier| barrier.at)
            .min()
    }

    pub(crate) fn origin(&self, symbol: SymbolId) -> Option<u32> {
        self.barriers
            .iter()
            .filter(|barrier| barrier.visible.contains(&symbol))
            .map(|barrier| barrier.at)
            .min()
    }

    pub(crate) fn expression(&self, scoping: &Scoping, expression: &Expression<'_>) -> Option<u32> {
        let mut reads = Reads {
            barriers: self,
            scoping,
            span: Some(expression.span()),
            at: None,
        };
        reads.visit_expression(expression);
        reads.at
    }

    pub(crate) fn statement(&self, scoping: &Scoping, statement: &Statement<'_>) -> Option<u32> {
        let mut reads = Reads {
            barriers: self,
            scoping,
            span: None,
            at: None,
        };
        reads.visit_statement(statement);
        reads.at
    }
}

fn expand_reachable(proof: &Proof<'_, '_>, visible: &mut FxHashSet<SymbolId>) {
    let mut edges = Vec::new();
    for (_, bindings) in proof.scoping.iter_bindings() {
        for (_, symbol) in bindings {
            if matches!(proof.binding(*symbol), Shape::Primitive) {
                continue;
            }
            let declaration = proof.scoping.symbol_declaration(*symbol);
            let span = proof.nodes.kind(declaration).span();
            for node in proof
                .nodes
                .iter()
                .filter(|node| span.contains_inclusive(node.kind().span()))
            {
                if let AstKind::IdentifierReference(identifier) = node.kind()
                    && let Some(dependency) = binding_of(proof.scoping, identifier)
                    && !matches!(proof.binding(dependency), Shape::Primitive)
                {
                    edges.push((*symbol, dependency));
                }
            }
        }
    }
    loop {
        let mut updated = false;
        for (holder, dependency) in &edges {
            if visible.contains(holder) {
                updated |= visible.insert(*dependency);
            }
            if visible.contains(dependency) {
                updated |= visible.insert(*holder);
            }
        }
        if !updated {
            break;
        }
    }
}

struct Reads<'s> {
    barriers: &'s EvalBarriers,
    scoping: &'s Scoping,
    span: Option<Span>,
    at: Option<u32>,
}

impl<'a> Visit<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if let Some(at) = self.barriers.affects(
            binding_of(self.scoping, identifier),
            self.span.unwrap_or(identifier.span),
        ) {
            self.at = Some(self.at.map_or(at, |previous| previous.min(at)));
        }
    }

    fn visit_expression(&mut self, expression: &Expression<'a>) {
        walk::walk_expression(self, expression);
    }
}
