//! One explicit policy for callees whose arguments cannot be mutated or retained.

use crate::css_prop::binding_of;
use crate::imported_constants::provenance::{Proof, Shape};
use oxc_ast::{
    AstKind,
    ast::{CallExpression, Expression, UnaryExpression},
};
use oxc_span::GetSpan;

/// Semantic global identity, including literal computed members. A write or
/// escape of the global anywhere in the module invalidates its builtin policy.
pub(crate) fn global<'e>(
    proof: &Proof<'_, '_>,
    callee: &'e Expression<'_>,
) -> Option<(&'e str, &'e str)> {
    let (identifier, member) = match crate::utils::unwrap_syntax_only(callee) {
        Expression::Identifier(identifier) => (identifier, ""),
        Expression::StaticMemberExpression(member) => {
            let Expression::Identifier(object) = &member.object else {
                return None;
            };
            (object, member.property.name.as_str())
        }
        Expression::ComputedMemberExpression(member) => {
            let (Expression::Identifier(object), Expression::StringLiteral(key)) =
                (&member.object, &member.expression)
            else {
                return None;
            };
            (object, key.value.as_str())
        }
        _ => return None,
    };
    if binding_of(proof.scoping, identifier).is_some() {
        return None;
    }
    pristine(proof, identifier.name.as_str()).then_some((identifier.name.as_str(), member))
}

pub(crate) fn pristine(proof: &Proof<'_, '_>, name: &str) -> bool {
    for reference in proof
        .scoping
        .root_unresolved_references()
        .get(name)
        .into_iter()
        .flatten()
    {
        let reference = proof.scoping.get_reference(*reference);
        if reference.is_write() {
            return false;
        }
        let mut node = reference.node_id();
        loop {
            let span = proof.nodes.kind(node).span();
            let parent = proof.nodes.parent_id(node);
            match proof.nodes.kind(parent) {
                AstKind::StaticMemberExpression(member) if member.object.span() == span => {}
                AstKind::ComputedMemberExpression(member) if member.object.span() == span => {}
                AstKind::AssignmentExpression(assignment) if assignment.left.span() == span => {
                    return false;
                }
                AstKind::UpdateExpression(_)
                | AstKind::UnaryExpression(UnaryExpression {
                    operator: oxc_syntax::operator::UnaryOperator::Delete,
                    ..
                })
                | AstKind::VariableDeclarator(_)
                | AstKind::ObjectProperty(_)
                | AstKind::ArrayExpression(_) => return false,
                AstKind::CallExpression(call) if call.callee.span() != span => return false,
                _ => break,
            }
            node = parent;
        }
    }
    true
}

/// Callbacks and user coercion/serialization hooks are not read-only proofs.
pub(super) fn reads_arguments(proof: &Proof<'_, '_>, call: &CallExpression<'_>) -> bool {
    let Some(callee) = global(proof, &call.callee) else {
        return false;
    };
    let arguments_plain = || {
        if !pristine(proof, "Object") || !pristine(proof, "Array") {
            return false;
        }
        call.arguments.iter().all(|argument| {
            argument
                .as_expression()
                .is_some_and(|argument| proof.expression(argument).plain())
        })
    };
    match callee {
        ("JSON", "stringify") | ("Object", "keys" | "freeze") => {
            call.arguments.len() == 1 && arguments_plain()
        }
        ("String" | "Number", "") => call.arguments.len() <= 1 && arguments_plain(),
        (
            "console",
            "log" | "info" | "warn" | "error" | "debug" | "trace" | "dir" | "table" | "assert"
            | "count" | "countReset" | "time" | "timeLog" | "timeEnd" | "group" | "groupCollapsed"
            | "groupEnd",
        )
        | ("Array", "isArray") => arguments_plain(),
        ("Math", _) | ("Boolean" | "parseInt" | "parseFloat" | "isNaN" | "isFinite", "") => {
            call.arguments.iter().all(|argument| {
                argument
                    .as_expression()
                    .is_some_and(|argument| matches!(proof.expression(argument), Shape::Primitive))
            })
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "console_tests.rs"]
mod console_tests;
