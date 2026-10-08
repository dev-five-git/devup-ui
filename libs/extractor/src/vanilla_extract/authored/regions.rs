use oxc_ast::{AstKind, ast::Expression};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};
use oxc_syntax::node::NodeId;

use crate::utils::unwrap_syntax_only;

pub(super) fn eager(semantic: &Semantic<'_>, node: NodeId, regions: &[Span]) -> bool {
    let span = semantic.nodes().kind(node).span();
    if !regions.iter().any(|region| region.contains_inclusive(span)) {
        return false;
    }
    semantic.nodes().ancestor_ids(node).all(|ancestor| {
        let kind = semantic.nodes().kind(ancestor);
        if !matches!(
            kind,
            AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
        ) {
            return true;
        }
        let function = kind.span();
        if regions.contains(&function) {
            return true;
        }
        if !regions
            .iter()
            .any(|region| region.contains_inclusive(function))
        {
            return true;
        }
        semantic
            .nodes()
            .ancestor_ids(ancestor)
            .find_map(|parent| match semantic.nodes().kind(parent) {
                AstKind::ParenthesizedExpression(_)
                | AstKind::TSAsExpression(_)
                | AstKind::TSSatisfiesExpression(_)
                | AstKind::TSNonNullExpression(_)
                | AstKind::TSTypeAssertion(_) => None,
                AstKind::CallExpression(call) => {
                    Some(unwrap_syntax_only(&call.callee).span() == function)
                }
                _ => Some(false),
            })
            .unwrap_or(false)
    })
}

pub(super) fn extend(semantic: &Semantic<'_>, regions: &mut Vec<Span>) -> bool {
    let mut changed = false;
    for node in semantic.nodes().iter() {
        let AstKind::CallExpression(call) = node.kind() else {
            continue;
        };
        if !eager(semantic, node.id(), regions) {
            continue;
        }
        let function = match unwrap_syntax_only(&call.callee) {
            Expression::ArrowFunctionExpression(function) => Some(function.span),
            Expression::FunctionExpression(function) => Some(function.span),
            Expression::Identifier(identifier) => identifier
                .reference_id
                .get()
                .and_then(|id| semantic.scoping().get_reference(id).symbol_id())
                .and_then(|symbol| {
                    match semantic
                        .nodes()
                        .kind(semantic.scoping().symbol_declaration(symbol))
                    {
                        AstKind::Function(function) => Some(function.span),
                        AstKind::VariableDeclarator(declarator) => declarator
                            .init
                            .as_ref()
                            .and_then(|init| match unwrap_syntax_only(init) {
                                Expression::ArrowFunctionExpression(function) => {
                                    Some(function.span)
                                }
                                Expression::FunctionExpression(function) => Some(function.span),
                                _ => None,
                            }),
                        _ => None,
                    }
                }),
            _ => None,
        };
        if let Some(function) = function
            && !regions.contains(&function)
        {
            regions.push(function);
            changed = true;
        }
    }
    changed
}
