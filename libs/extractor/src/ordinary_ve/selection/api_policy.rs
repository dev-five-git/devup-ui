use super::apis::Apis;
use crate::{barrel::native::Shape, mutations::Use};
use oxc_ast::{AstKind, ast::Program};
use oxc_span::{GetSpan, Span};

pub(super) struct Checked {
    pub errors: Vec<(Span, String)>,
    pub failure: Option<String>,
}

pub(super) fn errors(program: &Program<'_>, apis: &Apis<'_, '_>) -> Checked {
    let mut errors = apis.errors.clone();
    let mut failure = apis.failure.clone();
    let scoping = apis.semantic.scoping();
    for symbol in apis.bindings.keys() {
        for reference in scoping.get_resolved_reference_ids(*symbol) {
            let data = scoping.get_reference(*reference);
            if data.is_write() || data.flags().is_member_write_target() {
                errors.push((apis.semantic.nodes().kind(data.node_id()).span(), format!("native API binding `{}` may be changed outside its exact initialization slice", scoping.symbol_name(*symbol))));
            }
        }
    }
    let names: rustc_hash::FxHashSet<_> = apis
        .bindings
        .keys()
        .map(|symbol| scoping.symbol_name(*symbol))
        .collect();
    for (name, uses) in crate::mutations::uses(program, &|name| names.contains(name), None) {
        let Some(symbol) = scoping.get_root_binding(name.as_str().into()) else {
            continue;
        };
        let Some(binding) = apis.bindings.get(&symbol) else {
            continue;
        };
        for usage in uses {
            let at = match usage {
                Use::Changes { at, .. } => at,
                Use::Escapes {
                    at,
                    into: None,
                    path,
                } => {
                    if scoping
                        .get_resolved_reference_ids(symbol)
                        .iter()
                        .any(|reference| {
                            let data = scoping.get_reference(*reference);
                            apis.semantic.nodes().kind(data.node_id()).span().start == at
                                && super::api_usage::classify(apis, data.node_id(), *binding)
                                    .is_permitted()
                        })
                    {
                        continue;
                    }
                    let mut shape = apis.shapes.get(&symbol).cloned();
                    for key in path {
                        if let Some(key) = key {
                            shape = shape.and_then(|shape| shape.member(&key));
                        } else {
                            break;
                        }
                    }
                    if shape.is_none() {
                        continue;
                    }
                    at
                }
                Use::Escapes { into: Some(_), .. } | Use::Calls { .. } => continue,
            };
            errors.push((Span::new(at, at), format!("native API binding `{name}` may be changed outside its exact initialization slice")));
        }
    }
    for node in apis.semantic.nodes().iter() {
        let AstKind::ComputedMemberExpression(member) = node.kind() else {
            continue;
        };
        if !matches!(
            apis.shape(&member.object).as_deref(),
            Some(Shape::Namespace(_) | Shape::PackageNamespace(_))
        ) {
            continue;
        }
        if let Some(symbol) = apis.symbol(&member.expression) {
            for reference in scoping.get_resolved_reference_ids(symbol) {
                let data = scoping.get_reference(*reference);
                if data.is_write() {
                    errors.push((
                        apis.semantic.nodes().kind(data.node_id()).span(),
                        "native API key may be changed outside its exact initialization slice"
                            .to_string(),
                    ));
                }
            }
        }
        if let Some(key) = apis.key(&member.expression)
            && let Some(shape) = apis
                .shape(&member.object)
                .and_then(|shape| shape.member(&key))
        {
            match shape.as_ref() {
                Shape::Failed(message) => errors.push((member.span, message.clone())),
                Shape::OriginalFailure(message) => {
                    failure.get_or_insert_with(|| message.clone());
                }
                Shape::Api(_) | Shape::Namespace(_) | Shape::PackageNamespace(_) => {}
            }
        }
    }
    for node in apis.semantic.nodes().iter() {
        if let AstKind::StaticMemberExpression(member) = node.kind()
            && let Some(shape) = apis
                .shape(&member.object)
                .and_then(|shape| shape.member(member.property.name.as_str()))
        {
            match shape.as_ref() {
                Shape::Failed(message) => errors.push((member.span, message.clone())),
                Shape::OriginalFailure(message) => {
                    failure.get_or_insert_with(|| message.clone());
                }
                Shape::Api(_) | Shape::Namespace(_) | Shape::PackageNamespace(_) => {}
            }
        }
    }
    errors.sort_by_key(|(span, _)| span.start);
    errors.dedup();
    Checked { errors, failure }
}
