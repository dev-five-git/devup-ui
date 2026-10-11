//! Demand activation extends the existing lexical index and callable graph.

use oxc_ast::{AstKind, ast::Program};
use oxc_semantic::Semantic;
use oxc_span::Span;
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
mod audit;
mod commonjs;
mod exports;
mod properties;
#[cfg(test)]
mod property_projection_tests;
mod provenance;
#[cfg(test)]
pub(crate) mod receiver_fixtures;
#[cfg(test)]
mod receiver_projection_tests;
#[cfg(test)]
mod receiver_test_support;
mod render;
pub(crate) use provenance::select_consumer;
pub(crate) use provenance::{select_reads, select_resolved};

#[derive(Clone, Copy)]
pub(super) enum Seeds<'a> {
    Native,
    Consumer(&'a [Span]),
}

pub(crate) struct View {
    pub selection: Selection,
    pub exports: Vec<(String, String)>,
    pub forwarded: Vec<(String, Demand, Span)>,
    pub reexports: Vec<Span>,
    pub default: Option<(Span, String)>,
    pub native_units: FxHashSet<NodeId>,
    pub native_imports: FxHashSet<SymbolId>,
    pub native_carrier: bool,
    symbols: FxHashMap<SymbolId, Demand>,
    units: FxHashMap<NodeId, Demand>,
    properties: FxHashMap<NodeId, properties::Properties>,
}

fn select_with_apis<'a>(
    parsed: (&Program<'a>, &Semantic<'a>),
    demand: &Demand,
    input: (&Apis<'_, 'a>, Seeds<'_>),
) -> View {
    let (apis, seeds) = input;
    let (program, semantic) = parsed;
    let index = Index::new(program, semantic);
    let mut graph = Graph::new(&index, apis);
    let selection = super::select_with_apis(program, semantic, apis);
    let mut view = View {
        native_units: selection.units.iter().map(|unit| unit.node).collect(),
        native_imports: selection
            .imports
            .iter()
            .map(|import| import.binding.symbol)
            .collect(),
        selection,
        exports: Vec::new(),
        forwarded: Vec::new(),
        reexports: Vec::new(),
        default: None,
        native_carrier: false,
        symbols: FxHashMap::default(),
        units: FxHashMap::default(),
        properties: FxHashMap::default(),
    };
    exports::seed(program, &index, demand, &mut view);
    match seeds {
        Seeds::Native => {}
        Seeds::Consumer(reads) => {
            commonjs::seed(&index, demand, &mut view);
            audit::seed(&index, reads, &mut view);
        }
    }
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
            for callable in targets::binding_callables(*symbol, apis) {
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
            if let Some(init) = commonjs::initializer(semantic.nodes().kind(*owner))
                && let Some(properties) = properties::Properties::select(init, demanded, semantic)
            {
                view.properties.insert(*owner, properties);
            }
        }
        commonjs::activate(&index, &view, (apis, &mut graph));
        let selected: FxHashSet<_> = view.units.keys().copied().collect();
        for node in semantic.nodes().iter() {
            if let AstKind::ThisExpression(this) = node.kind()
                && let Some(owner) = index
                    .owner(node.id())
                    .filter(|owner| selected.contains(owner))
                && !view.properties.get(&owner).is_some_and(|properties| {
                    properties
                        .omitted
                        .iter()
                        .any(|span| span.contains_inclusive(this.span))
                })
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
            let projected = view
                .properties
                .get(&owner)
                .and_then(|properties| properties.dependency(identifier.span))
                .cloned();
            let alias = commonjs::initializer(semantic.nodes().kind(owner))
                .and_then(|init| properties::forward(init, &view.units[&owner]))
                .and_then(|(span, demand)| (span == identifier.span).then_some(demand));
            let dependency = if let Some(demand) = projected.or(alias) {
                demand
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
        if import.source == "@vanilla-extract/css" || import.source == "@devup-ui/react" {
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
    if matches!(seeds, Seeds::Consumer(_)) {
        commonjs::requests(&index, &mut view);
    }
    view
}
