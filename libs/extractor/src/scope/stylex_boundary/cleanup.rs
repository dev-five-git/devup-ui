use oxc_ast::ast::{BindingPattern, Expression, ImportDeclarationSpecifier, Program, Statement};
use oxc_ast_visit::{Visit, VisitMut, walk_mut};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use crate::scope::Bindings;
use crate::scope::stylex_bindings::{StylexBinding, unbound_reference};
use crate::scope::stylex_sources::{StylexSource, require_source};
use crate::utils::unwrap_syntax_only;

pub(super) fn remove(bindings: &Bindings, program: &mut Program<'_>, package: &str) {
    loop {
        let mut used = Used {
            bindings,
            symbols: FxHashSet::default(),
        };
        used.visit_program(program);
        let mut remove = Remove {
            bindings,
            package,
            used: used.symbols,
            changed: false,
        };
        remove.visit_program(program);
        if !remove.changed {
            break;
        }
    }
}

struct Used<'s> {
    bindings: &'s Bindings,
    symbols: FxHashSet<SymbolId>,
}

impl<'a> Visit<'a> for Used<'_> {
    fn visit_variable_declarator(&mut self, declarator: &oxc_ast::ast::VariableDeclarator<'a>) {
        if declarator
            .id
            .get_binding_identifier()
            .and_then(|local| local.symbol_id.get())
            .is_some_and(|symbol| {
                matches!(
                    self.bindings.stylex.binding(symbol),
                    Some(
                        StylexBinding::Namespace
                            | StylexBinding::UpstreamNamespace
                            | StylexBinding::Function(_)
                            | StylexBinding::Invalid
                    )
                )
            })
        {
            return;
        }
        oxc_ast_visit::walk::walk_variable_declarator(self, declarator);
    }
    fn visit_identifier_reference(&mut self, identifier: &oxc_ast::ast::IdentifierReference<'a>) {
        self.symbols.extend(self.bindings.symbol(identifier));
    }
    fn visit_ts_type(&mut self, _: &oxc_ast::ast::TSType<'a>) {}
}

struct Remove<'s> {
    bindings: &'s Bindings,
    package: &'s str,
    used: FxHashSet<SymbolId>,
    changed: bool,
}

impl Remove<'_> {
    fn consumed(&self, symbol: Option<SymbolId>) -> bool {
        symbol.is_some_and(|symbol| match self.bindings.stylex.binding(symbol) {
            Some(
                StylexBinding::Root
                | StylexBinding::Namespace
                | StylexBinding::UpstreamNamespace
                | StylexBinding::Function(_)
                | StylexBinding::Invalid,
            ) => !self.used.contains(&symbol),
            Some(StylexBinding::RootDefault) | None => false,
        })
    }
}

impl<'a> VisitMut<'a> for Remove<'_> {
    fn visit_statements(&mut self, statements: &mut oxc_allocator::Vec<'a, Statement<'a>>) {
        walk_mut::walk_statements(self, statements);
        let before = statements.len();
        statements.retain_mut(|statement| {
            if let Statement::ImportDeclaration(import) = statement {
                if import.import_kind.is_type() { return true; }
                let had_values = import.specifiers.as_ref().is_some_and(|specifiers| !specifiers.is_empty());
                if let Some(specifiers) = &mut import.specifiers {
                    let before = specifiers.len();
                    specifiers.retain(|specifier| {
                        matches!(specifier, ImportDeclarationSpecifier::ImportSpecifier(named) if named.import_kind.is_type())
                            || !self.consumed(specifier.local().symbol_id.get())
                    });
                    self.changed |= specifiers.len() != before;
                    if !specifiers.is_empty() { return true; }
                }
                return match StylexSource::classify(import.source.value.as_str(), self.package) {
                    StylexSource::Upstream | StylexSource::Dedicated | StylexSource::TypesOnly => false,
                    StylexSource::Root => !had_values,
                    StylexSource::Other => true,
                };
            }
            if let Statement::VariableDeclaration(declaration) = statement {
                return !declaration.declarations.is_empty();
            }
            if let Statement::ExportDeclaration(export) = statement
                && let oxc_ast::ast::Declaration::VariableDeclaration(declaration) = &export.declaration {
                return !declaration.declarations.is_empty();
            }
            true
        });
        self.changed |= statements.len() != before;
    }

    fn visit_variable_declarators(
        &mut self,
        declarators: &mut oxc_allocator::Vec<'a, oxc_ast::ast::VariableDeclarator<'a>>,
    ) {
        walk_mut::walk_variable_declarators(self, declarators);
        let before = declarators.len();
        declarators.retain_mut(|declarator| {
            let Some(init) = &declarator.init else { return true; };
            let required = matches!(unwrap_syntax_only(init), Expression::CallExpression(call)
                if require_source(call).is_some_and(|source| StylexSource::classify(source, self.package) != StylexSource::Other)
                    && matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(loader) if unbound_reference(&self.bindings.scoping, loader)));
            match &mut declarator.id {
                BindingPattern::BindingIdentifier(local) => !self.consumed(local.symbol_id.get()),
                BindingPattern::ObjectPattern(object) if required => {
                    let before = object.properties.len();
                    object.properties.retain(|property| !self.consumed(property.value.get_binding_identifier().and_then(|local| local.symbol_id.get())));
                    self.changed |= object.properties.len() != before;
                    !object.properties.is_empty() || object.rest.is_some()
                }
                _ => true,
            }
        });
        self.changed |= declarators.len() != before;
    }
}
