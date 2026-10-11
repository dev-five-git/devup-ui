use crate::utils::{get_string_by_literal_expression, unwrap_syntax_only};
use oxc_ast::{
    AstKind,
    ast::{Expression, VariableDeclarationKind},
};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;

pub(crate) fn resolve(expression: &Expression<'_>, semantic: &Semantic<'_>) -> Option<String> {
    constant(expression, semantic, &mut Vec::new())
}

fn constant(
    expression: &Expression<'_>,
    semantic: &Semantic<'_>,
    seen: &mut Vec<SymbolId>,
) -> Option<String> {
    if let Some(key) = get_string_by_literal_expression(unwrap_syntax_only(expression)) {
        return Some(key.into_owned());
    }
    let Expression::Identifier(id) = unwrap_syntax_only(expression) else {
        return None;
    };
    let scoping = semantic.scoping();
    let symbol = scoping.get_reference(id.reference_id.get()?).symbol_id()?;
    if seen.contains(&symbol)
        || scoping
            .get_resolved_reference_ids(symbol)
            .iter()
            .any(|reference| scoping.get_reference(*reference).is_write())
    {
        return None;
    }
    let node = scoping.symbol_declaration(symbol);
    let (node, declarator) = std::iter::once(node)
        .chain(semantic.nodes().ancestor_ids(node))
        .find_map(|node| match semantic.nodes().kind(node) {
            AstKind::VariableDeclarator(declarator) => Some((node, declarator)),
            _ => None,
        })?;
    let AstKind::VariableDeclaration(declaration) = semantic.nodes().parent_kind(node) else {
        return None;
    };
    if declaration.kind != VariableDeclarationKind::Const
        || declaration.declare
        || declarator.span.end > expression.span().start
    {
        return None;
    }
    seen.push(symbol);
    let result = constant(declarator.init.as_ref()?, semantic, seen);
    seen.pop();
    result
}
