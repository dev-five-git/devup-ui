use oxc_ast::{AstKind, ast::Expression};
use oxc_span::GetSpan;

use super::{Apis, Demand, Graph, Index, MemberDemand, View, closure, targets};

#[cfg(test)]
#[path = "commonjs_prefix_tests.rs"]
mod prefix_tests;
#[cfg(test)]
#[path = "commonjs_request_tests.rs"]
mod request_tests;
#[cfg(test)]
#[path = "commonjs_seed_tests.rs"]
mod seed_tests;
#[cfg(test)]
#[path = "commonjs_test_support.rs"]
mod test_support;

pub(super) fn activate<'a>(
    index: &Index<'_, 'a>,
    view: &View,
    context: (&Apis<'_, 'a>, &mut Graph),
) {
    let (apis, graph) = context;
    for owner in view.units.keys() {
        let kind = index.semantic.nodes().kind(*owner);
        if !matches!(kind, AstKind::ExpressionStatement(_)) {
            continue;
        }
        let Some(init) = initializer(kind) else {
            continue;
        };
        for callable in targets::callables(init, apis) {
            let span = index.semantic.nodes().kind(callable).span();
            if !view.properties.get(owner).is_some_and(|properties| {
                properties
                    .omitted
                    .iter()
                    .any(|omitted| omitted.contains_inclusive(span))
            }) {
                graph.active.extend(graph.reachable(callable));
            }
        }
    }
}

pub(super) fn initializer(kind: AstKind<'_>) -> Option<&Expression<'_>> {
    match kind {
        AstKind::VariableDeclarator(declarator) => declarator.init.as_ref(),
        AstKind::ExpressionStatement(statement) => match &statement.expression {
            Expression::AssignmentExpression(assignment) => Some(&assignment.right),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn seed(index: &Index<'_, '_>, demand: &Demand, view: &mut View) {
    if (!demand.whole && demand.members.is_empty())
        || index.semantic.nodes().iter().any(|node| matches!(node.kind(), AstKind::Program(program) if program.body.iter().any(oxc_ast::ast::Statement::is_module_declaration)))
    { return; }
    for unit in index.units.values() {
        let AstKind::ExpressionStatement(statement) = index.semantic.nodes().kind(unit.node) else {
            continue;
        };
        let Expression::AssignmentExpression(assignment) = &statement.expression else {
            continue;
        };
        let Some(member) = assignment
            .left
            .as_simple_assignment_target()
            .and_then(|left| left.as_member_expression())
        else {
            continue;
        };
        let Some(root) = crate::css_prop::root_reference(member.object()) else {
            continue;
        };
        if !matches!(root.name.as_str(), "module" | "exports")
            || root.reference_id.get().is_some_and(|reference| {
                index
                    .semantic
                    .scoping()
                    .get_reference(reference)
                    .symbol_id()
                    .is_some()
            })
        {
            continue;
        }
        let Some(mut path) = member
            .static_property_name()
            .map(|key| vec![Some(key.to_string())])
        else {
            view.selection.provenance_errors.push((
                assignment.span,
                "required CommonJS export needs an exact static key".to_string(),
            ));
            continue;
        };
        let mut object = member.object();
        loop {
            match object {
                Expression::StaticMemberExpression(member) => {
                    path.push(Some(member.property.name.to_string()));
                    object = &member.object;
                }
                Expression::ComputedMemberExpression(member) => {
                    path.push(super::super::static_key::resolve(
                        &member.expression,
                        index.semantic,
                    ));
                    object = &member.object;
                }
                _ => break,
            }
        }
        path.reverse();
        if root.name == "module" {
            if path.first().and_then(|key| key.as_deref()) != Some("exports") {
                continue;
            }
            path.remove(0);
        }
        let Some(path) = path.into_iter().collect::<Option<Vec<_>>>() else {
            view.selection.provenance_errors.push((
                assignment.span,
                "required CommonJS export needs an exact static path".to_string(),
            ));
            continue;
        };
        let mut selected = Some(demand);
        for key in &path {
            selected = selected.and_then(|selected| selected.child(key));
        }
        if let Some(selected) = selected {
            let mut selected = selected.clone();
            if path.is_empty()
                && !selected.whole
                && let Some(default) = selected.members.remove("default")
            {
                selected.merge(&default);
            }
            view.units.entry(unit.node).or_default().merge(&selected);
        }
    }
}

pub(super) fn requests(index: &Index<'_, '_>, view: &mut View) {
    for node in index.semantic.nodes().iter() {
        let AstKind::CallExpression(call) = node.kind() else {
            continue;
        };
        let Expression::Identifier(require) = &call.callee else {
            continue;
        };
        if require.name != "require"
            || require.reference_id.get().is_some_and(|reference| {
                index
                    .semantic
                    .scoping()
                    .get_reference(reference)
                    .symbol_id()
                    .is_some()
            })
        {
            continue;
        }
        let [oxc_ast::ast::Argument::StringLiteral(source)] = call.arguments.as_slice() else {
            continue;
        };
        let Some(owner) = index
            .owner(node.id())
            .filter(|owner| view.units.contains_key(owner))
        else {
            continue;
        };
        if view.properties.get(&owner).is_some_and(|properties| {
            properties
                .omitted
                .iter()
                .any(|span| span.contains_inclusive(call.span))
        }) {
            continue;
        }
        let demand = match closure::demand(index, node.id()) {
            MemberDemand::Path(path) => {
                let mut demand = Demand::default();
                demand.insert(&path);
                demand
            }
            MemberDemand::Whole => initializer(index.semantic.nodes().kind(owner))
                .filter(|init| init.span() == call.span)
                .map_or_else(Demand::whole, |_| view.units[&owner].clone()),
        };
        view.forwarded
            .push((source.value.to_string(), demand, source.span));
    }
}
