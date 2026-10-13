//! Lexical slices only: no source execution, generated code, or loader requests.

use oxc_ast::ast::Program;
use oxc_semantic::Semantic;
use rustc_hash::FxHashSet;

mod api_policy;
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
pub(crate) mod static_key;
mod targets;

pub(crate) use plan::Selection;

/// Select native initialization owners and their individual lexical inputs.
/// `semantic` must describe this unmodified program with AST nodes enabled.
#[cfg(test)]
pub(crate) fn select<'a>(program: &Program<'a>, semantic: &Semantic<'a>) -> Selection {
    select_for_package(program, semantic, "@vanilla-extract/css")
}

#[cfg(test)]
pub(crate) fn select_for_package<'a>(
    program: &Program<'a>,
    semantic: &Semantic<'a>,
    package: &str,
) -> Selection {
    let apis = apis::Apis::for_package(program, semantic, package);
    select_with_apis(program, semantic, &apis)
}

pub(crate) fn select_resolved<'a>(
    parsed: (&Program<'a>, &Semantic<'a>),
    origin: (&str, &str),
    resolver: Option<&crate::ModuleResolver>,
) -> Selection {
    let facts = crate::barrel::native::Facts::resolve(parsed.0, origin, resolver);
    let apis = apis::Apis::with_facts(parsed.0, parsed.1, facts);
    select_with_apis(parsed.0, parsed.1, &apis)
}

fn select_with_apis<'a>(
    program: &Program<'a>,
    semantic: &Semantic<'a>,
    apis: &apis::Apis<'_, 'a>,
) -> Selection {
    let index = index::Index::new(program, semantic);
    let mut graph = call_graph::Graph::new(&index, apis);
    let root_units = index.ordered(&graph.roots);
    let closure = closure::collect(&index, apis, &mut graph);
    let audit = escapes::audit(&index, apis, &graph);
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
    for import in &mut imports {
        if import.native == Some(plan::NativeBinding::Namespace) {
            import.preserved = semantic
                .scoping()
                .get_resolved_reference_ids(import.binding.symbol)
                .iter()
                .any(|reference| {
                    let data = semantic.scoping().get_reference(*reference);
                    data.is_value()
                        && index.owner(data.node_id()).is_none_or(|owner| {
                            !graph.roots.contains(&owner) && !audit.consumed.contains(&owner)
                        })
                        && api_usage::classify(apis, data.node_id(), plan::NativeBinding::Namespace)
                            .is_permitted()
                });
        }
    }
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
        crate::mutations::resolved_uses(semantic, &|name| native_names.contains(name))
            .into_iter()
            .filter_map(|(symbol, uses)| {
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
    let checked = api_policy::errors(program, apis);
    let provenance_errors = checked
        .errors
        .into_iter()
        .filter(|(span, _)| {
            !index.units.values().any(|unit| {
                unit.span.contains_inclusive(*span)
                    && audit.consumed.contains(&unit.node)
                    && !graph.selected.contains(&unit.node)
            })
        })
        .collect();
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
        dependencies: apis.dependencies.clone(),
        provenance_errors,
        provenance_failure: checked.failure,
    }
}

#[cfg(test)]
mod tests;
