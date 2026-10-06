use super::provenance::Proof;
use crate::{css_prop::binding_of, utils::unwrap_syntax_only};
use oxc_ast::{AstKind, ast::Expression};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

pub(super) fn eval_callee(
    proof: &Proof<'_, '_>,
    expression: &Expression<'_>,
    seen: &mut FxHashSet<SymbolId>,
) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::Identifier(identifier) => match binding_of(proof.scoping, identifier) {
            None => identifier.name == "eval",
            Some(symbol) if seen.insert(symbol) => {
                let initial = match proof.nodes.kind(proof.scoping.symbol_declaration(symbol)) {
                    AstKind::VariableDeclarator(declaration) => declaration
                        .init
                        .as_ref()
                        .is_some_and(|init| eval_callee(proof, init, seen)),
                    _ => false,
                };
                initial
                    || proof
                        .scoping
                        .get_resolved_reference_ids(symbol)
                        .iter()
                        .any(|id| {
                            let reference = proof.scoping.get_reference(*id);
                            if !reference.is_write() {
                                return false;
                            }
                            let deferred = identifier.reference_id.get().is_some_and(|id| {
                                proof
                                    .nodes
                                    .ancestor_kinds(proof.scoping.get_reference(id).node_id())
                                    .any(|kind| {
                                        matches!(
                                            kind,
                                            AstKind::Function(_)
                                                | AstKind::ArrowFunctionExpression(_)
                                                | AstKind::Class(_)
                                        )
                                    })
                            });
                            match proof.nodes.parent_kind(reference.node_id()) {
                                AstKind::AssignmentExpression(assignment)
                                    if deferred
                                        || assignment.span.start < identifier.span.start =>
                                {
                                    eval_callee(proof, &assignment.right, seen)
                                }
                                _ => false,
                            }
                        })
            }
            Some(_) => false,
        },
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .last()
            .is_some_and(|last| eval_callee(proof, last, seen)),
        Expression::StaticMemberExpression(member) => {
            (matches!(member.property.name.as_str(), "call" | "apply")
                && eval_callee(proof, &member.object, seen))
                || (member.property.name == "eval"
                    && matches!(&member.object,
                    Expression::Identifier(identifier) if identifier.name == "globalThis" && binding_of(proof.scoping, identifier).is_none()))
        }
        Expression::ComputedMemberExpression(member) => {
            match unwrap_syntax_only(&member.expression) {
                Expression::StringLiteral(key)
                    if matches!(key.value.as_str(), "call" | "apply") =>
                {
                    eval_callee(proof, &member.object, seen)
                }
                Expression::StringLiteral(key) if key.value == "eval" => {
                    matches!(unwrap_syntax_only(&member.object),
                    Expression::Identifier(identifier) if identifier.name == "globalThis" && binding_of(proof.scoping, identifier).is_none())
                }
                _ => false,
            }
        }
        Expression::CallExpression(call) => matches!(unwrap_syntax_only(&call.callee),
            Expression::StaticMemberExpression(member) if member.property.name == "bind" && eval_callee(proof, &member.object, seen)),
        _ => false,
    }
}
