use oxc_ast::{AstKind, ast::Expression};
use oxc_semantic::Semantic;
use oxc_syntax::node::NodeId;

pub(super) fn values<'a>(semantic: &Semantic<'a>, callable: NodeId) -> Vec<&'a Expression<'a>> {
    let nodes = semantic.nodes();
    if let AstKind::ArrowFunctionExpression(arrow) = nodes.kind(callable)
        && let Some(expression) = arrow.body.as_expression()
    {
        return vec![expression];
    }
    nodes
        .iter()
        .filter_map(|node| {
            let AstKind::ReturnStatement(statement) = node.kind() else {
                return None;
            };
            let owner = nodes.ancestor_ids(node.id()).find(|ancestor| {
                matches!(
                    nodes.kind(*ancestor),
                    AstKind::Function(_) | AstKind::ArrowFunctionExpression(_)
                )
            });
            if owner == Some(callable) {
                statement.argument.as_ref()
            } else {
                None
            }
        })
        .collect()
}
