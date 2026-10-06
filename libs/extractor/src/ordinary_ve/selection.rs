//! Lexical slices only: no source execution, generated code, or loader requests.

use oxc_ast::ast::Program;
use oxc_semantic::Semantic;
use rustc_hash::FxHashSet;

mod api_usage;
mod apis;
mod binding_path;
mod call_graph;
mod closure;
pub(crate) mod demand;
mod escapes;
mod index;
pub(crate) mod plan;
mod returns;
mod targets;

pub(crate) use plan::Selection;

/// Select native initialization owners and their individual lexical inputs.
/// `semantic` must describe this unmodified program with AST nodes enabled.
pub(crate) fn select<'a>(program: &Program<'a>, semantic: &Semantic<'a>) -> Selection {
    select_for_package(program, semantic, "@vanilla-extract/css")
}

pub(crate) fn select_for_package<'a>(
    program: &Program<'a>,
    semantic: &Semantic<'a>,
    package: &str,
) -> Selection {
    let index = index::Index::new(program, semantic);
    let apis = apis::Apis::for_package(program, semantic, package);
    let mut graph = call_graph::Graph::new(&index, &apis);
    let root_units = index.ordered(&graph.roots);
    let closure = closure::collect(&index, &apis, &mut graph);
    let audit = escapes::audit(&index, &apis, &graph);
    let roots = root_units
        .iter()
        .map(|unit| plan::Root {
            owner: unit.node,
            span: unit.span,
            native_calls: graph.native_sites(&graph.reachable(unit.node)),
            captures: unit.bindings.clone(),
        })
        .collect();
    let mut imports: Vec<_> = apis
        .imports
        .values()
        .filter(|import| {
            import.native.is_some() || closure.imports.contains(&import.binding.symbol)
        })
        .cloned()
        .collect();
    imports.sort_by_key(|import| import.specifier.start);
    let selected_symbols: FxHashSet<_> = closure
        .reads
        .iter()
        .filter_map(|read| read.symbol)
        .collect();
    let native_names: FxHashSet<_> = apis
        .bindings
        .keys()
        .map(|symbol| semantic.scoping().symbol_name(*symbol))
        .collect();
    let mut mutations: Vec<_> =
        crate::mutations::uses(program, &|name| native_names.contains(name), None)
            .into_iter()
            .filter_map(|(name, uses)| {
                let symbol = semantic.scoping().get_root_binding(name.as_str().into())?;
                selected_symbols.contains(&symbol).then_some((symbol, uses))
            })
            .flat_map(|(symbol, uses)| {
                uses.into_iter()
                    .map(move |usage| plan::Mutation { symbol, usage })
            })
            .collect();
    mutations.sort_by_key(|mutation| match &mutation.usage {
        crate::mutations::Use::Changes { at, .. }
        | crate::mutations::Use::Calls { at, .. }
        | crate::mutations::Use::Escapes { at, .. } => *at,
    });
    Selection {
        units: index.ordered(&graph.selected),
        roots,
        imports,
        demands: closure.demands,
        reads: closure.reads,
        mutations,
        native_calls: graph
            .native
            .into_iter()
            .filter(|call| {
                index
                    .callable_context(call.node)
                    .or_else(|| index.owner(call.node))
                    .is_some_and(|context| graph.active.contains(&context))
            })
            .collect(),
        helper_calls: graph
            .edges
            .into_iter()
            .filter(|edge| graph.active.contains(&edge.caller))
            .collect(),
        consumed: index.ordered(&audit.consumed),
        escapes: audit.escapes,
        checks: audit.checks,
        reserved_names: semantic
            .scoping()
            .symbol_names()
            .map(str::to_string)
            .chain(
                semantic
                    .scoping()
                    .root_unresolved_references()
                    .keys()
                    .map(ToString::to_string),
            )
            .collect(),
    }
}

#[cfg(test)]
mod tests;
