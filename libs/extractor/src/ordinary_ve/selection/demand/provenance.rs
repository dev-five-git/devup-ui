use super::{Apis, Demand, View};
use oxc_ast::{AstKind, ast::Program};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;

pub(crate) fn select_resolved<'a>(
    parsed: (&Program<'a>, &Semantic<'a>),
    demand: &Demand,
    origin: (&str, &str, Option<&crate::ModuleResolver>),
) -> View {
    carrier(parsed, (demand, super::Seeds::Native), origin)
}

pub(crate) fn select_consumer<'a>(
    parsed: (&Program<'a>, &Semantic<'a>),
    demand: &Demand,
    origin: (&str, &str, Option<&crate::ModuleResolver>),
) -> View {
    carrier(parsed, (demand, super::Seeds::Consumer(&[])), origin)
}

fn carrier<'a>(
    parsed: (&Program<'a>, &Semantic<'a>),
    input: (&Demand, super::Seeds<'_>),
    origin: (&str, &str, Option<&crate::ModuleResolver>),
) -> View {
    let mut facts = crate::barrel::native::Facts::resolve(parsed.0, (origin.0, origin.1), origin.2);
    let native_carrier = facts.prove_carrier(
        (parsed.0, parsed.1.source_text()),
        (origin.0, origin.1),
        (input.0, origin.2),
    );
    let apis = Apis::with_facts(parsed.0, parsed.1, facts);
    let mut view = super::select_with_apis(parsed, input.0, (&apis, input.1));
    view.native_carrier = native_carrier;
    let semantic = parsed.1;
    view.selection.escapes.retain(|escape| {
        if escape.kind != super::super::plan::EscapeKind::NativeValue
            || !view.symbols.contains_key(&escape.symbol)
            || !apis.bindings.contains_key(&escape.symbol)
        {
            return true;
        }
        if view.default.as_ref().is_some_and(|(span, _)| {
            span.contains_inclusive(escape.span)
                && parsed.0.body.iter().any(|statement| {
                    matches!(statement, oxc_ast::ast::Statement::ExportDefaultDeclaration(export)
                        if export.declaration.span() == *span
                            && export.declaration.as_expression().is_some_and(|expression|
                                apis.binding(expression).is_some()))
                })
        }) {
            return false;
        }
        !semantic.nodes().iter().any(|node| {
            node.kind().span() == escape.span
                && (semantic
                    .nodes()
                    .ancestor_kinds(node.id())
                    .any(|kind| matches!(kind, AstKind::ExportSpecifier(_)))
                    || (view
                        .selection
                        .units
                        .iter()
                        .any(|unit| unit.span.contains_inclusive(escape.span))
                        && apis.bindings.get(&escape.symbol).is_some_and(|binding| {
                            super::super::api_usage::classify(&apis, node.id(), *binding)
                                .is_permitted()
                        })))
        })
    });
    view
}

pub(crate) fn select_reads<'a>(
    parsed: (&Program<'a>, &Semantic<'a>),
    input: (&Demand, &[oxc_span::Span]),
    origin: (&str, &str, Option<&crate::ModuleResolver>),
) -> View {
    let facts = crate::barrel::native::Facts::resolve(parsed.0, (origin.0, origin.1), origin.2);
    let apis = Apis::with_facts(parsed.0, parsed.1, facts);
    super::select_with_apis(parsed, input.0, (&apis, super::Seeds::Consumer(input.1)))
}
