use oxc_ast::{AstKind, ast::Program};
use oxc_semantic::Semantic;

use super::{Context, Use};

#[cfg(test)]
#[path = "commonjs_tests.rs"]
mod tests;

pub(crate) fn uses<'a>(program: &Program<'a>, semantic: &Semantic<'a>) -> Vec<(String, Use)> {
    if program
        .body
        .iter()
        .any(oxc_ast::ast::Statement::is_module_declaration)
    {
        return Vec::new();
    }
    let context = Context {
        nodes: semantic.nodes(),
        scoping: semantic.scoping(),
        style: &|_| false,
        css: None,
        init: None,
    };
    semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let read = match node.kind() {
                AstKind::IdentifierReference(identifier)
                    if identifier.name == "exports" && context.is_global(identifier) =>
                {
                    "exports"
                }
                AstKind::StaticMemberExpression(member)
                    if member.property.name == "exports"
                        && matches!(&member.object, oxc_ast::ast::Expression::Identifier(identifier)
                        if identifier.name == "module" && context.is_global(identifier)) =>
                {
                    "module.exports"
                }
                AstKind::ComputedMemberExpression(member)
                    if crate::utils::get_string_by_literal_expression(&member.expression)
                        .as_deref()
                        == Some("exports")
                        && matches!(&member.object, oxc_ast::ast::Expression::Identifier(identifier)
                        if identifier.name == "module" && context.is_global(identifier)) =>
                {
                    "module.exports"
                }
                _ => return None,
            };
            context
                .classify(node.id())
                .map(|usage| (read.to_string(), usage))
        })
        .collect()
}
