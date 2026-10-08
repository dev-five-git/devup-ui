use oxc_ast::AstKind;
use oxc_span::GetSpan;
use oxc_syntax::node::NodeId;

use super::{
    apis::Apis,
    plan::{EscapeKind, NativeBinding},
};

pub(super) enum Usage {
    Ordinary,
    NativeAllowed,
    Escape(EscapeKind),
}

impl Usage {
    pub(super) const fn is_permitted(&self) -> bool {
        match self {
            Self::Ordinary | Self::NativeAllowed => true,
            Self::Escape(_) => false,
        }
    }
}

pub(super) fn classify(apis: &Apis<'_, '_>, node: NodeId, binding: NativeBinding) -> Usage {
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
            | AstKind::TSInstantiationExpression(_)
            | AstKind::ObjectProperty(_)
            | AstKind::ObjectExpression(_) => {}
            AstKind::StaticMemberExpression(member) if member.object.span() == span => {
                let Some(member_binding) =
                    apis.member(&member.object, member.property.name.as_str())
                else {
                    return if binding != NativeBinding::Namespace
                        || !matches!(
                            apis.shape(&member.object).as_deref(),
                            Some(crate::barrel::native::Shape::Namespace(members))
                                if !members.contains_key(member.property.name.as_str())
                        ) {
                        Usage::Escape(EscapeKind::NativeValue)
                    } else {
                        Usage::Ordinary
                    };
                };
                binding = member_binding;
            }
            AstKind::ComputedMemberExpression(member) if member.object.span() == span => {
                let Some(key) = apis.key(&member.expression) else {
                    return Usage::Escape(EscapeKind::DynamicNamespace);
                };
                let Some(member_binding) = apis.member(&member.object, &key) else {
                    return if binding != NativeBinding::Namespace
                        || !matches!(
                            apis.shape(&member.object).as_deref(),
                            Some(crate::barrel::native::Shape::Namespace(members))
                                if !members.contains_key(&key)
                        ) {
                        Usage::Escape(EscapeKind::NativeValue)
                    } else {
                        Usage::Ordinary
                    };
                };
                binding = member_binding;
            }
            AstKind::CallExpression(call) if call.callee.span() == span => {
                return match binding {
                    NativeBinding::Named { .. } => Usage::NativeAllowed,
                    NativeBinding::Namespace => Usage::Escape(EscapeKind::NativeValue),
                };
            }
            AstKind::VariableDeclarator(declarator)
                if declarator
                    .init
                    .as_ref()
                    .is_some_and(|init| init.span() == span) =>
            {
                return if declarator.id.get_binding_identifiers().iter().all(|id| {
                    id.symbol_id
                        .get()
                        .is_some_and(|symbol| apis.bindings.contains_key(&symbol))
                }) {
                    Usage::NativeAllowed
                } else {
                    Usage::Escape(EscapeKind::NativeValue)
                };
            }
            _ => return Usage::Escape(EscapeKind::NativeValue),
        }
        span = kind.span();
    }
    Usage::Escape(EscapeKind::NativeValue)
}
