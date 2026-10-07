use oxc_ast::AstKind;
use oxc_span::{GetSpan, Span};
use oxc_syntax::{node::NodeId, symbol::SymbolId};
use rustc_hash::FxHashSet;

use super::{
    apis::Apis,
    call_graph::Graph,
    index::Index,
    plan::{ImportDemand, MemberDemand, Read},
};

#[derive(Default)]
pub(super) struct Closure {
    pub imports: FxHashSet<SymbolId>,
    pub reads: Vec<Read>,
    pub demands: Vec<ImportDemand>,
}

pub(super) fn collect(index: &Index<'_, '_>, apis: &Apis<'_, '_>, graph: &mut Graph) -> Closure {
    let mut closure = Closure::default();
    let mut pending: Vec<_> = graph.roots.iter().copied().collect();
    let nodes = index.semantic.nodes();
    let scoping = index.semantic.scoping();
    while let Some(owner) = pending.pop() {
        if !graph.selected.insert(owner) {
            continue;
        }
        let Some(unit) = index.units.get(&owner) else {
            continue;
        };
        if !matches!(unit.kind, super::plan::UnitKind::Function { .. }) {
            let active = graph.reachable(owner);
            graph.active.extend(active);
        }
        for node in nodes.iter() {
            let AstKind::IdentifierReference(identifier) = node.kind() else {
                continue;
            };
            if index.owner(node.id()) != Some(owner) {
                continue;
            }
            if index
                .callable_context(node.id())
                .is_some_and(|context| !graph.active.contains(&context))
            {
                continue;
            }
            let Some(reference) = identifier.reference_id.get() else {
                continue;
            };
            let data = scoping.get_reference(reference);
            if !data.is_value() {
                continue;
            }
            let symbol = data.symbol_id();
            closure.reads.push(Read {
                span: identifier.span,
                symbol,
                name: identifier.name.to_string(),
                write: data.is_write(),
            });
            let Some(symbol) = symbol else { continue };
            if unit.span.contains_inclusive(scoping.symbol_span(symbol)) {
                continue;
            }
            if apis.imports.contains_key(&symbol) {
                closure.imports.insert(symbol);
                let member = demand(index, node.id());
                if matches!(member, MemberDemand::Whole)
                    && let Some(shape) = apis.shapes.get(&symbol)
                    && matches!(
                        shape.as_ref(),
                        crate::barrel::native::Shape::Namespace(_)
                            | crate::barrel::native::Shape::PackageNamespace(_)
                    )
                {
                    for path in shape.paths() {
                        closure.demands.push(ImportDemand {
                            symbol,
                            read: identifier.span,
                            member: MemberDemand::Path(path),
                        });
                    }
                } else {
                    closure.demands.push(ImportDemand {
                        symbol,
                        read: identifier.span,
                        member,
                    });
                }
            } else if let Some(dependency) = index.bindings.get(&symbol) {
                pending.push(*dependency);
            }
        }
    }
    closure.reads.sort_by_key(|read| read.span.start);
    closure.demands.sort_by_key(|demand| demand.read.start);
    closure
}

pub(super) fn demand(index: &Index<'_, '_>, node: NodeId) -> MemberDemand {
    let nodes = index.semantic.nodes();
    let mut span: Span = nodes.kind(node).span();
    let mut path = Vec::new();
    for ancestor in nodes.ancestor_ids(node) {
        let kind = nodes.kind(ancestor);
        match kind {
            AstKind::StaticMemberExpression(member) if member.object.span() == span => {
                path.push(member.property.name.to_string());
            }
            AstKind::ComputedMemberExpression(member) if member.object.span() == span => {
                let Some(key) = super::static_key::resolve(&member.expression, index.semantic)
                else {
                    return MemberDemand::Whole;
                };
                path.push(key);
            }
            AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSInstantiationExpression(_) => {}
            _ => break,
        }
        span = kind.span();
    }
    if path.is_empty() {
        MemberDemand::Whole
    } else {
        MemberDemand::Path(path)
    }
}
