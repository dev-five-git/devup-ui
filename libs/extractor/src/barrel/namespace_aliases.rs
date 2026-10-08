//! Whole namespace aliases, followed by semantic identity regardless of declaration order.

use oxc_ast::AstKind;
use oxc_ast::ast::{BindingPattern, Expression, VariableDeclarationKind};
use rustc_hash::FxHashMap;

use super::{Namespace, Rewriter, reference_symbol};

impl Rewriter<'_, '_, '_, '_> {
    /// Collects every immutable namespace alias before checking its runtime reads.
    pub(super) fn namespace_aliases(&self, namespaces: &mut Vec<Namespace>) {
        let mut spaces: FxHashMap<_, _> = namespaces
            .iter()
            .enumerate()
            .filter_map(|(index, namespace)| Some((namespace.symbol?, index)))
            .collect();
        let nodes = self.semantic.nodes();
        loop {
            let before = spaces.len();
            for node in nodes.iter() {
                let AstKind::VariableDeclarator(declarator) = node.kind() else {
                    continue;
                };
                let (BindingPattern::BindingIdentifier(id), Some(Expression::Identifier(init))) =
                    (&declarator.id, &declarator.init)
                else {
                    continue;
                };
                let (Some(alias), Some(origin), AstKind::VariableDeclaration(declaration)) = (
                    id.symbol_id.get(),
                    reference_symbol(init, self.semantic),
                    nodes.parent_kind(node.id()),
                ) else {
                    continue;
                };
                if declaration.kind != VariableDeclarationKind::Const || spaces.contains_key(&alias)
                {
                    continue;
                }
                if let Some(&index) = spaces.get(&origin) {
                    let origin = &namespaces[index];
                    let namespace = Namespace {
                        local: id.name.to_string(),
                        symbol: Some(alias),
                        after: origin.after,
                        module: origin.module.clone(),
                        source: origin.source.clone(),
                    };
                    spaces.insert(alias, namespaces.len());
                    namespaces.push(namespace);
                }
            }
            if spaces.len() == before {
                break;
            }
        }
    }
}
