use oxc_ast::AstKind;
use oxc_span::GetSpan;
use oxc_syntax::{node::NodeId, symbol::SymbolId};
use rustc_hash::FxHashSet;

use super::{
    api_usage,
    apis::Apis,
    call_graph::Graph,
    index::Index,
    plan::{Escape, EscapeKind},
};

#[derive(Default)]
pub(super) struct Audit {
    pub consumed: FxHashSet<NodeId>,
    pub escapes: Vec<Escape>,
    pub checks: Vec<Escape>,
}

pub(super) fn audit(index: &Index<'_, '_>, apis: &Apis<'_, '_>, graph: &Graph) -> Audit {
    let scoping = index.semantic.scoping();
    let nodes = index.semantic.nodes();
    let roots = &graph.roots;
    let helpers: FxHashSet<_> = scoping
        .symbol_ids()
        .filter(|symbol| graph.helper(*symbol, apis))
        .collect();
    let mut audit = Audit::default();
    for symbol in apis.bindings.keys().chain(helpers.iter()) {
        if let Some(owner) = index
            .bindings
            .get(symbol)
            .filter(|owner| !roots.contains(owner))
        {
            audit.consumed.insert(*owner);
        }
    }
    for symbol in apis.bindings.keys() {
        for reference in scoping.get_resolved_reference_ids(*symbol) {
            let data = scoping.get_reference(*reference);
            if data.is_value()
                && let Some(owner) = index
                    .owner(data.node_id())
                    .filter(|owner| !roots.contains(owner) && index.helper_definition(*owner))
            {
                audit.consumed.insert(owner);
            }
        }
    }
    for call in &graph.native {
        let active = index
            .callable_context(call.node)
            .is_some_and(|context| graph.active.contains(&context));
        let Some(owner) = index
            .owner(call.node)
            .filter(|owner| !roots.contains(owner))
        else {
            continue;
        };
        if active || index.helper_definition(owner) {
            audit.consumed.insert(owner);
        }
    }
    loop {
        let removed: Vec<_> = audit
            .consumed
            .iter()
            .filter(|owner| {
                index.units.get(owner).is_some_and(|unit| {
                    unit.bindings.iter().any(|binding| {
                        exported(index, binding.symbol)
                            || scoping
                                .get_resolved_reference_ids(binding.symbol)
                                .iter()
                                .any(|reference| {
                                    let data = scoping.get_reference(*reference);
                                    data.is_value()
                                        && index.owner(data.node_id()).is_none_or(|owner| {
                                            !roots.contains(&owner)
                                                && !audit.consumed.contains(&owner)
                                        })
                                })
                    })
                })
            })
            .copied()
            .collect();
        if removed.is_empty() {
            break;
        }
        for owner in removed {
            audit.consumed.remove(&owner);
        }
    }
    for symbol in helpers {
        let Some(owner) = index.bindings.get(&symbol) else {
            continue;
        };
        if audit.consumed.contains(owner) {
            continue;
        }
        for reference in scoping.get_resolved_reference_ids(symbol) {
            let data = scoping.get_reference(*reference);
            if data.is_value()
                && index
                    .owner(data.node_id())
                    .is_none_or(|owner| !roots.contains(&owner) && !audit.consumed.contains(&owner))
            {
                audit.escapes.push(Escape {
                    span: nodes.kind(data.node_id()).span(),
                    symbol,
                    kind: EscapeKind::RuntimeHelper,
                });
            }
        }
        if exported(index, symbol) {
            audit.escapes.push(Escape {
                span: scoping.symbol_span(symbol),
                symbol,
                kind: EscapeKind::RuntimeHelper,
            });
        }
    }
    for node in nodes.iter() {
        let AstKind::IdentifierReference(identifier) = node.kind() else {
            continue;
        };
        let Some(reference) = identifier.reference_id.get() else {
            continue;
        };
        let data = scoping.get_reference(reference);
        let Some(symbol) = data.symbol_id().filter(|_| data.is_value()) else {
            continue;
        };
        let Some(binding) = apis.bindings.get(&symbol) else {
            continue;
        };
        let owner = index.owner(node.id());
        let allowed = owner.is_some_and(|owner| {
            graph.selected.contains(&owner) || audit.consumed.contains(&owner)
        });
        let kind = api_usage::classify(apis, node.id(), *binding);
        if allowed
            && (kind.is_none()
                || !index
                    .callable_context(node.id())
                    .is_none_or(|context| graph.active.contains(&context)))
        {
            continue;
        }
        let escape = Escape {
            span: identifier.span,
            symbol,
            kind: kind.unwrap_or(EscapeKind::NativeValue),
        };
        if allowed {
            audit.checks.push(escape);
        } else {
            audit.escapes.push(escape);
        }
    }
    audit.escapes.sort_by_key(|escape| escape.span.start);
    audit
        .escapes
        .dedup_by_key(|escape| (escape.span, escape.symbol, escape.kind));
    audit.checks.sort_by_key(|escape| escape.span.start);
    audit
}

fn exported(index: &Index<'_, '_>, symbol: SymbolId) -> bool {
    let nodes = index.semantic.nodes();
    let scoping = index.semantic.scoping();
    if scoping.symbol_scope_id(symbol) != scoping.root_scope_id() {
        return false;
    }
    nodes
        .ancestor_kinds(scoping.symbol_declaration(symbol))
        .any(|kind| {
            matches!(
                kind,
                AstKind::ExportDeclaration(_) | AstKind::ExportDefaultDeclaration(_)
            )
        })
        || scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .any(|reference| {
                let data = scoping.get_reference(*reference);
                data.is_value()
                    && nodes.ancestor_kinds(data.node_id()).any(|kind| match kind {
                        AstKind::ExportSpecifier(_) => true,
                        AstKind::ExportDefaultDeclaration(export) => {
                            export.declaration.span() == nodes.kind(data.node_id()).span()
                        }
                        _ => false,
                    })
            })
}
