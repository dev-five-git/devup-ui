//! Which bindings of a file stand for the APIs the build compiles away.
//! A name decides nothing: a call, an element or a member is compiled only
//! when the binding it reads is the import (or an alias of it) the file
//! declares, so a parameter, a `const` or a function sharing the name stays
//! the code it is.

mod class_names;
mod lookup;
mod required;
pub(crate) mod stylex_bindings;
mod stylex_boundary;
pub(crate) mod stylex_sources;
mod visits;

use std::rc::Rc;

use oxc_ast::ast::{BindingIdentifier, Expression, IdentifierReference, VariableDeclarator};
use oxc_ast_visit::{Visit, VisitMut};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::component::ExportVariableKind;
use crate::util_type::UtilType;

pub use class_names::ClassNamesSymbols;
pub use visits::requires;

/// What stands for `namespace.styled` once the visit has read it as a binding:
/// no source text spells it, so no binding can share it
pub const NAMESPACE_STYLED: &str = "<namespace>.styled";

/// The bindings of a program that stand for compiled APIs, by symbol
#[derive(Default)]
pub struct Bindings {
    scoping: Rc<Scoping>,
    /// Components the package exports
    imports: FxHashMap<SymbolId, ExportVariableKind>,
    /// The package imported whole, whose members are its exports
    namespaces: FxHashSet<SymbolId>,
    util_imports: FxHashMap<SymbolId, Rc<UtilType>>,
    styled_imports: FxHashSet<SymbolId>,
    /// Functions building elements from a type and props, by the export
    jsx_imports: FxHashMap<SymbolId, String>,
    jsx_namespaces: FxHashSet<SymbolId>,
    global_components: FxHashSet<SymbolId>,
    class_names_components: FxHashSet<SymbolId>,
    stylex: stylex_bindings::StylexBindings,
    /// The bindings of reads whose copies lost their references, by where
    /// they were written
    recovered: FxHashMap<(u32, u32), SymbolId>,
    /// What the build removes the declaration of, which only its calls and
    /// elements may read
    compiled: FxHashSet<SymbolId>,
    /// The `const newCss = css` declarations that go with them
    aliases: FxHashSet<SymbolId>,
}

impl Bindings {
    pub fn scope(&mut self, scoping: Rc<Scoping>) {
        self.scoping = scoping;
    }

    /// The binding `identifier` reads: by its reference, or by where it was
    /// written when a copy of it lost the reference
    pub fn symbol(&self, identifier: &IdentifierReference<'_>) -> Option<SymbolId> {
        identifier.reference_id.get().map_or_else(
            || {
                self.recovered
                    .get(&(identifier.span.start, identifier.span.end))
                    .copied()
            },
            |reference| self.scoping.get_reference(reference).symbol_id(),
        )
    }

    /// Whether `identifier` reads a binding of the module's top level, or a
    /// global, and not a local of a function or block sharing the name
    pub fn reads_module(&self, identifier: &IdentifierReference<'_>) -> bool {
        self.symbol(identifier).is_none_or(|symbol| {
            self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
        })
    }

    /// Remember the bindings the reads in `expression` stand for, before code
    /// that copies it without their references, which the visit then reads
    /// again
    pub fn remember(&mut self, expression: &Expression<'_>) {
        let mut reads = visits::Remember::new(self);
        reads.visit_expression(expression);
        let recovered = reads.into_recovered();
        self.recovered.extend(recovered);
    }

    pub(crate) fn module_binding(&self, name: &str) -> Option<SymbolId> {
        self.scoping
            .get_root_binding(name.into())
            .filter(|symbol| self.unchanged(*symbol))
    }

    /// Declare that `symbol` binds what the package exports as `imported`;
    /// `false` when the package exports nothing the build compiles by it
    pub fn export(&mut self, symbol: Option<SymbolId>, imported: &str) -> bool {
        symbol.is_some_and(|symbol| {
            if let Ok(kind) = imported.parse::<ExportVariableKind>() {
                self.imports.insert(symbol, kind);
            } else if let Some(kind) = UtilType::from_str_opt(imported) {
                self.util_imports.insert(symbol, Rc::new(kind));
            } else if imported == "styled" {
                self.styled_imports.insert(symbol);
            } else {
                return false;
            }
            true
        })
    }

    /// Declare that `symbol` binds the package imported whole
    pub fn namespace(&mut self, symbol: Option<SymbolId>) {
        self.namespaces.extend(symbol);
    }

    /// Declare that `symbol` binds the module building elements from a type
    /// and props
    pub fn jsx_namespace(&mut self, symbol: Option<SymbolId>) {
        self.jsx_namespaces.extend(symbol);
    }

    /// Declare that `local` binds the function building elements `imported`
    pub fn jsx_import(&mut self, local: &BindingIdentifier<'_>, imported: String) {
        if let Some(symbol) = local.symbol_id.get() {
            self.jsx_imports.insert(symbol, imported);
        }
    }

    pub fn global_component(&mut self, symbol: Option<SymbolId>) {
        self.global_components.extend(symbol);
    }

    pub fn class_names_component(&mut self, symbol: Option<SymbolId>) {
        self.class_names_components.extend(symbol);
    }

    pub fn stylex_namespace(&mut self, symbol: Option<SymbolId>) {
        self.stylex
            .insert(symbol, stylex_bindings::StylexBinding::Namespace);
    }

    pub(crate) fn discover_stylex(&mut self, program: &oxc_ast::ast::Program<'_>, package: &str) {
        self.stylex = stylex_bindings::StylexBindings::collect(program, &self.scoping, package);
    }

    pub(crate) fn stylex_binding(
        &self,
        expression: &Expression<'_>,
    ) -> Option<stylex_bindings::StylexBinding> {
        self.stylex.resolve(expression, &|id| self.symbol(id))
    }

    /// Declare that the build removes the declaration of `symbol`
    pub fn compile(&mut self, symbol: Option<SymbolId>) {
        self.compiled.extend(symbol);
    }

    pub fn is_compiled(&self) -> bool {
        !self.compiled.is_empty()
    }

    /// Declare an alias of what `declarator` initializes from, when it binds
    /// one: `const newCss = css`
    pub fn alias(&mut self, declarator: &VariableDeclarator<'_>) {
        if let Some(Expression::Identifier(init)) = &declarator.init
            && let Some(from) = self.symbol(init)
            && let Some(symbol) = declarator
                .id
                .get_binding_identifier()
                .and_then(|id| id.symbol_id.get())
            && self.copy(from, symbol)
        {
            self.compiled.insert(symbol);
            self.aliases.insert(symbol);
        }
    }

    fn copy(&mut self, from: SymbolId, to: SymbolId) -> bool {
        if let Some(util) = self.util_imports.get(&from).cloned() {
            self.util_imports.insert(to, util);
        } else if self.styled_imports.contains(&from) {
            self.styled_imports.insert(to);
        } else if let Some(kind) = self.imports.get(&from).cloned() {
            self.imports.insert(to, kind);
        } else {
            return false;
        }
        true
    }

    /// Remove the declarations of the aliases from `program`, wherever they
    /// stand
    pub fn remove_aliases(&self, program: &mut oxc_ast::ast::Program<'_>) {
        if !self.aliases.is_empty() {
            visits::RemoveAliases::new(&self.aliases).visit_program(program);
        }
    }

    /// Where `program` still reads what the build compiled away, with the
    /// name read: by the binding each read stands for, not by its name
    pub fn compiled_reads(&self, program: &oxc_ast::ast::Program<'_>) -> Vec<(u32, String)> {
        let mut reads = visits::CompiledReads::new(self, false);
        reads.visit_program(program);
        reads.into_found()
    }

    /// Where `expression`, about to be written as styles, reads what the
    /// build compiled away other than by calling or rendering it: those
    /// reads would survive as names of nothing
    pub fn lowered_reads(&self, expression: &Expression<'_>) -> Vec<(u32, String)> {
        if self.compiled.is_empty() {
            return Vec::new();
        }
        let mut reads = visits::CompiledReads::new(self, true);
        reads.visit_expression(expression);
        reads.into_found()
    }

    /// [`Self::lowered_reads`] of the attributes of an element
    pub fn lowered_attribute_reads(
        &self,
        attributes: &[oxc_ast::ast::JSXAttributeItem<'_>],
    ) -> Vec<(u32, String)> {
        if self.compiled.is_empty() {
            return Vec::new();
        }
        let mut reads = visits::CompiledReads::new(self, true);
        for attribute in attributes {
            reads.visit_jsx_attribute_item(attribute);
        }
        reads.into_found()
    }
}

#[cfg(test)]
mod tests;
