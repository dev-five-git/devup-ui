use oxc_semantic::Semantic;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::{Context, Use};

/// Classify uses without rebuilding or replacing the caller's semantic identities.
pub(crate) fn resolved_uses<'a>(
    semantic: &Semantic<'a>,
    style: &dyn Fn(&str) -> bool,
) -> FxHashMap<SymbolId, Vec<Use>> {
    let scoping = semantic.scoping();
    let nodes = semantic.nodes();
    scoping
        .get_bindings(scoping.root_scope_id())
        .values()
        .filter_map(|symbol| {
            let init = match nodes.kind(scoping.symbol_declaration(*symbol)) {
                oxc_ast::AstKind::VariableDeclarator(declarator) => declarator.init.as_ref(),
                _ => None,
            };
            let context = Context {
                nodes,
                scoping,
                style,
                css: None,
                init,
            };
            let uses: Vec<_> = scoping
                .get_resolved_reference_ids(*symbol)
                .iter()
                .map(|id| scoping.get_reference(*id))
                .filter(|reference| reference.is_value())
                .filter_map(|reference| context.classify(reference.node_id()))
                .collect();
            (!uses.is_empty()).then_some((*symbol, uses))
        })
        .collect()
}
