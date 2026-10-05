//! Live import reads, including CSS exports whose values only the bundler knows.

use oxc_ast::{AstKind, ast::ImportDeclarationSpecifier};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

pub(super) enum BindingSource<'a> {
    Module(&'a str),
    Css(&'a str),
}

#[derive(Default)]
pub(super) struct ImportBindings {
    symbols: FxHashMap<SymbolId, String>,
    names: FxHashMap<String, String>,
}

impl ImportBindings {
    pub(super) fn link(
        &mut self,
        specifiers: &[ImportDeclarationSpecifier<'_>],
        source: BindingSource<'_>,
    ) {
        for specifier in specifiers {
            let binding = match &source {
                BindingSource::Module(module) => match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        format!("{module}[{:?}]", specifier.imported.name())
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        format!("{module}[\"default\"]")
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => module.to_string(),
                },
                BindingSource::Css(module) => match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        format!("{module}[{:?}]", specifier.imported.name())
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                    | ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => module.to_string(),
                },
            };
            let local = specifier.local();
            self.names.insert(local.name.to_string(), binding.clone());
            if let Some(symbol) = local.symbol_id.get() {
                self.symbols.insert(symbol, binding);
            }
        }
    }

    pub(super) fn exported(&self, local: String) -> String {
        self.names.get(&local).cloned().unwrap_or(local)
    }

    pub(super) fn rewrites(
        &self,
        semantic: &Semantic<'_>,
        script: &str,
    ) -> Vec<(u32, u32, String)> {
        let mut replacements = Vec::new();
        for (symbol, binding) in &self.symbols {
            for reference in semantic.scoping().get_resolved_reference_ids(*symbol) {
                let node = semantic.scoping().get_reference(*reference).node_id();
                let span = semantic.nodes().kind(node).span();
                let parent = semantic.nodes().parent_kind(node);
                let expression = binding;
                let replacement = match parent {
                    AstKind::ObjectProperty(property) if property.shorthand => format!(
                        "{}: {expression}",
                        &script[usize::try_from(span.start).unwrap_or(script.len())
                            ..usize::try_from(span.end).unwrap_or(script.len())]
                    ),
                    AstKind::ExportSpecifier(_) => continue,
                    _ => expression.clone(),
                };
                replacements.push((span.start, span.end, replacement));
            }
        }
        replacements
    }
}
