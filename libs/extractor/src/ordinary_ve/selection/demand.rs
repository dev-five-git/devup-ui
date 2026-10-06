//! Demand activation extends the existing lexical index and callable graph.

use oxc_ast::{AstKind, ast::Program};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use oxc_syntax::{node::NodeId, symbol::SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

use super::{
    apis::Apis,
    call_graph::Graph,
    closure,
    index::Index,
    plan::{ImportName, MemberDemand, Selection},
    targets,
};
use crate::module_loader::demand::Demand;
mod exports;
mod properties;
mod render;

pub(crate) struct View {
    pub selection: Selection,
    pub exports: Vec<(String, String)>,
    pub forwarded: Vec<(String, Demand, Span)>,
    pub reexports: Vec<Span>,
    pub default: Option<(Span, String)>,
    symbols: FxHashMap<SymbolId, Demand>,
    units: FxHashMap<NodeId, Demand>,
    properties: FxHashMap<NodeId, properties::Properties>,
}

pub(crate) fn select_for_package<'a>(
    parsed: (&Program<'a>, &Semantic<'a>),
    demand: &Demand,
    package: &str,
) -> View {
    let (program, semantic) = parsed;
    let index = Index::new(program, semantic);
    let apis = Apis::for_package(program, semantic, package);
    let mut graph = Graph::new(&index, &apis);
    let mut view = View {
        selection: super::select_for_package(program, semantic, package),
        exports: Vec::new(),
        forwarded: Vec::new(),
        reexports: Vec::new(),
        default: None,
        symbols: FxHashMap::default(),
        units: FxHashMap::default(),
        properties: FxHashMap::default(),
    };
    exports::seed(program, &index, demand, &mut view);
    for root in &view.selection.roots {
        view.units.insert(root.owner, Demand::whole());
    }
    loop {
        let before_symbols = view.symbols.clone();
        let before_units = view.units.clone();
        for (symbol, demand) in &view.symbols {
            if let Some(owner) = index.bindings.get(symbol) {
                view.units
                    .entry(*owner)
                    .or_default()
                    .merge(&exports::binding_demand(&index, *symbol, demand));
            }
            for callable in targets::binding_callables(*symbol, &apis) {
                graph.active.extend(graph.reachable(callable));
            }
            for reference in semantic.scoping().get_resolved_reference_ids(*symbol) {
                let reference = semantic.scoping().get_reference(*reference);
                if reference.is_write()
                    && index.callable_context(reference.node_id()).is_none()
                    && let Some(owner) = index.owner(reference.node_id())
                {
                    view.units.entry(owner).or_default().merge(&Demand::whole());
                }
            }
        }
        view.properties.clear();
        for (owner, demanded) in &view.units {
            graph.active.extend(graph.reachable(*owner));
            if let AstKind::VariableDeclarator(declarator) = semantic.nodes().kind(*owner)
                && let Some(init) = &declarator.init
                && let Some(properties) = properties::Properties::select(init, demanded)
            {
                view.properties.insert(*owner, properties);
            }
        }
        let selected: FxHashSet<_> = view.units.keys().copied().collect();
        for node in semantic.nodes().iter() {
            if let AstKind::ThisExpression(_) = node.kind()
                && let Some(owner) = index
                    .owner(node.id())
                    .filter(|owner| selected.contains(owner))
                && let AstKind::VariableDeclarator(declarator) = semantic.nodes().kind(owner)
                && declarator.init.as_ref().is_some_and(|init| {
                    matches!(
                        crate::utils::unwrap_syntax_only(init),
                        oxc_ast::ast::Expression::ObjectExpression(_)
                    )
                })
                && index.callable_context(node.id()).is_some_and(|context| {
                    graph.active.contains(&context)
                        && matches!(semantic.nodes().kind(context), AstKind::Function(_))
                })
            {
                let mut receiver = Demand::default();
                match closure::demand(&index, node.id()) {
                    MemberDemand::Path(path) => {
                        receiver.insert(&path);
                    }
                    MemberDemand::Whole => {
                        receiver.insert(&[]);
                    }
                }
                view.units.entry(owner).or_default().merge(&receiver);
            }
            let AstKind::IdentifierReference(identifier) = node.kind() else {
                continue;
            };
            let Some(owner) = index
                .owner(node.id())
                .filter(|owner| selected.contains(owner))
            else {
                continue;
            };
            if view.properties.get(&owner).is_some_and(|properties| {
                properties
                    .omitted
                    .iter()
                    .any(|span| span.contains_inclusive(identifier.span))
            }) {
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
            let data = semantic.scoping().get_reference(reference);
            if !data.is_value() {
                continue;
            }
            let Some(symbol) = data.symbol_id() else {
                continue;
            };
            let unit = &index.units[&owner];
            let local = unit
                .span
                .contains_inclusive(semantic.scoping().symbol_span(symbol));
            if local && !unit.bindings.iter().any(|binding| binding.symbol == symbol) {
                continue;
            }
            let member = closure::demand(&index, node.id());
            let dependency = if let AstKind::VariableDeclarator(declarator) =
                semantic.nodes().kind(owner)
                && declarator.init.as_ref().is_some_and(|init| {
                    crate::utils::unwrap_syntax_only(init).span() == identifier.span
                }) {
                view.units[&owner].clone()
            } else {
                match member {
                    MemberDemand::Whole => Demand::whole(),
                    MemberDemand::Path(path) => {
                        let mut result = Demand::default();
                        result.insert(&path);
                        result
                    }
                }
            };
            view.symbols.entry(symbol).or_default().merge(&dependency);
        }
        if view.symbols == before_symbols && view.units == before_units {
            break;
        }
    }
    view.selection.units = index.ordered(&view.units.keys().copied().collect());
    view.selection.imports = apis
        .imports
        .values()
        .filter(|import| {
            import.native.is_some() || view.symbols.contains_key(&import.binding.symbol)
        })
        .cloned()
        .collect();
    view.selection
        .imports
        .sort_by_key(|import| import.specifier.start);
    for import in &view.selection.imports {
        if import.native.is_some() {
            continue;
        }
        let child = view
            .symbols
            .get(&import.binding.symbol)
            .cloned()
            .unwrap_or_default();
        let demand = match &import.imported {
            ImportName::Named(name) => Demand::prefixed(name, &child),
            ImportName::Default => Demand::prefixed("default", &child),
            ImportName::Namespace => child,
        };
        view.forwarded
            .push((import.source.clone(), demand, import.specifier));
    }
    view
}
