//! Originating hazards of transitive function dependencies for located errors.

use super::{Change, ChangeSite, ModuleScope, Modules};
use crate::css_prop::binding_of;
use oxc_ast::AstKind;
use oxc_ast::ast::IdentifierReference;
use oxc_ast_visit::Visit;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;
use std::rc::Rc;

impl ModuleScope<'_, '_> {
    pub(super) fn dependency_change(
        &mut self,
        modules: &mut Modules<'_>,
        name: &str,
    ) -> Option<Rc<Change>> {
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(self.program)
            .semantic;
        let symbol = semantic.scoping().get_root_binding(name.into())?;
        let mut pending = vec![symbol];
        let mut seen = FxHashSet::default();
        let mut found = Vec::new();
        while let Some(symbol) = pending.pop() {
            if !seen.insert(symbol) {
                continue;
            }
            if semantic.scoping().symbol_scope_id(symbol) == semantic.scoping().root_scope_id()
                && let Some(change) = self.change(modules, semantic.scoping().symbol_name(symbol))
            {
                found.push(change);
            }
            let mut reads = Dependencies {
                scoping: semantic.scoping(),
                symbols: Vec::new(),
            };
            match semantic
                .nodes()
                .kind(semantic.scoping().symbol_declaration(symbol))
            {
                AstKind::VariableDeclarator(declarator) => {
                    if let Some(init) = &declarator.init {
                        reads.visit_expression(init);
                    }
                }
                AstKind::Function(function) => {
                    if let Some(body) = &function.body {
                        reads.visit_function_body(body);
                    }
                }
                _ => {}
            }
            pending.extend(reads.symbols);
        }
        found.sort_by(|left, right| {
            match (&left.site, &right.site) {
                (ChangeSite::Here(left), ChangeSite::Here(right)) => left.cmp(right),
                (ChangeSite::In(left), ChangeSite::In(right)) => left.cmp(right),
                (ChangeSite::Here(_), ChangeSite::In(_)) => std::cmp::Ordering::Less,
                (ChangeSite::In(_), ChangeSite::Here(_)) => std::cmp::Ordering::Greater,
            }
            .then(left.name.cmp(&right.name))
        });
        found.into_iter().next()
    }
}

struct Dependencies<'s> {
    scoping: &'s Scoping,
    symbols: Vec<SymbolId>,
}

impl<'a> Visit<'a> for Dependencies<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        self.symbols.extend(binding_of(self.scoping, identifier));
    }
    fn visit_ts_type(&mut self, _: &oxc_ast::ast::TSType<'a>) {}
}
