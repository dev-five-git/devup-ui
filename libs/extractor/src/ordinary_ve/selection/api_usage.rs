use oxc_ast::AstKind;
use oxc_span::GetSpan;
use oxc_syntax::node::NodeId;

use super::{
    apis::Apis,
    plan::{EscapeKind, NativeBinding},
};
use crate::utils::{get_string_by_literal_expression, unwrap_syntax_only};

pub(super) fn classify(
    apis: &Apis<'_, '_>,
    node: NodeId,
    binding: NativeBinding,
) -> Option<EscapeKind> {
    let nodes = apis.semantic.nodes();
    let mut span = nodes.kind(node).span();
    let mut binding = binding;
    for ancestor in nodes.ancestor_ids(node) {
        let kind = nodes.kind(ancestor);
        match kind {
            AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSInstantiationExpression(_) => {}
            AstKind::StaticMemberExpression(member) if member.object.span() == span => {
                let Some(member_binding) =
                    apis.member(&member.object, member.property.name.as_str())
                else {
                    return Some(EscapeKind::NativeValue);
                };
                binding = member_binding;
            }
            AstKind::ComputedMemberExpression(member) if member.object.span() == span => {
                if get_string_by_literal_expression(unwrap_syntax_only(&member.expression))
                    .is_none()
                {
                    return Some(EscapeKind::DynamicNamespace);
                }
                let key = get_string_by_literal_expression(unwrap_syntax_only(&member.expression))?;
                let Some(member_binding) = apis.member(&member.object, &key) else {
                    return Some(EscapeKind::NativeValue);
                };
                binding = member_binding;
            }
            AstKind::CallExpression(call) if call.callee.span() == span => {
                return match binding {
                    NativeBinding::Named { .. } => None,
                    NativeBinding::Namespace => Some(EscapeKind::NativeValue),
                };
            }
            AstKind::VariableDeclarator(declarator)
                if declarator
                    .init
                    .as_ref()
                    .is_some_and(|init| init.span() == span) =>
            {
                return (!declarator.id.get_binding_identifiers().iter().all(|id| {
                    id.symbol_id
                        .get()
                        .is_some_and(|symbol| apis.bindings.contains_key(&symbol))
                }))
                .then_some(EscapeKind::NativeValue);
            }
            _ => return Some(EscapeKind::NativeValue),
        }
        span = kind.span();
    }
    Some(EscapeKind::NativeValue)
}
