use oxc_ast::AstKind;
use oxc_ast::ast::{
    BindingIdentifier, BindingPattern, Expression, Program, Statement, VariableDeclarationKind,
};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::reference_symbol;

pub(super) struct Demand<'s, 'a> {
    semantic: &'s Semantic<'a>,
    factories: FxHashSet<SymbolId>,
}

impl<'s, 'a> Demand<'s, 'a> {
    pub(super) fn unused(&self, binding: &BindingIdentifier<'_>) -> bool {
        let semantic = self.semantic;
        binding.symbol_id.get().is_some_and(|symbol| {
            semantic
                .scoping()
                .get_resolved_reference_ids(symbol)
                .iter()
                .all(|id| !semantic.scoping().get_reference(*id).is_value())
        })
    }

    pub(super) fn new(semantic: &'s Semantic<'a>, program: &Program<'_>) -> Self {
        let factories: FxHashSet<_> = program
            .body
            .iter()
            .filter_map(|statement| {
                let Statement::ImportDeclaration(import) = statement else {
                    return None;
                };
                matches!(
                    import.source.value.as_str(),
                    "react/jsx-runtime" | "react/jsx-dev-runtime" | "react"
                )
                .then_some(import)
            })
            .flat_map(|import| import.specifiers.iter().flatten())
            .filter_map(|specifier| {
                let oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(named) = specifier
                else {
                    return None;
                };
                matches!(
                    named.imported.name().as_str(),
                    "jsx" | "jsxs" | "jsxDEV" | "createElement"
                )
                .then(|| named.local.symbol_id.get())
                .flatten()
            })
            .collect();
        Self {
            semantic,
            factories,
        }
    }

    pub(super) fn render_only(&self, binding: &BindingIdentifier<'_>) -> bool {
        binding
            .symbol_id
            .get()
            .is_some_and(|symbol| self.renders(symbol, &mut FxHashSet::default()))
    }

    fn renders(&self, symbol: SymbolId, seen: &mut FxHashSet<SymbolId>) -> bool {
        if !seen.insert(symbol) {
            return false;
        }
        let semantic = self.semantic;
        let nodes = semantic.nodes();
        semantic.scoping().get_resolved_reference_ids(symbol).iter().all(|id| {
        let reference = semantic.scoping().get_reference(*id);
        if !reference.is_value() {return true}
        if reference.flags().is_write() {return false}
        let node = reference.node_id();
        let span = nodes.kind(node).span();
        match nodes.parent_kind(node) {
            AstKind::JSXOpeningElement(element) => element.name.span() == span,
            AstKind::JSXClosingElement(element) => element.name.span() == span,
            AstKind::VariableDeclarator(declarator) if declarator.init.as_ref().is_some_and(|init| init.span() == span) => {
                let BindingPattern::BindingIdentifier(binding) = &declarator.id else {return false};
                matches!(nodes.parent_kind(nodes.parent_id(node)), AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const)
                    && binding.symbol_id.get().is_some_and(|alias| self.renders(alias, seen))
            }
            AstKind::CallExpression(call) if call.arguments.first().is_some_and(|argument| argument.span() == span) => {
                matches!(&call.callee, Expression::Identifier(callee) if reference_symbol(callee, semantic).is_some_and(|symbol| self.factories.contains(&symbol)))
            }
            _ => false,
        }
    })
    }
}

pub(super) fn selected(source: &str, extensions: &[String]) -> bool {
    let path = source.split(['?', '#']).next().unwrap_or(source);
    extensions
        .iter()
        .any(|extension| path.to_ascii_lowercase().ends_with(extension))
}
