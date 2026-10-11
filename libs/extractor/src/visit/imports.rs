//! What the bindings an import declares stand for, when the import gives
//! values: a type import, whole or by specifier, binds nothing the build
//! compiles, and stays where it is written.

use super::DevupVisitor;
use oxc_ast::ast::{ImportDeclaration, ImportDeclarationSpecifier};

const CREATE_ELEMENT: &str = "createElement";

impl DevupVisitor<'_> {
    /// Declare the bindings a value import of `react` makes: `createElement`
    /// by name, and the module imported whole, whose `createElement` it holds
    pub(super) fn react_import(&mut self, it: &ImportDeclaration<'_>) {
        for specifier in it.specifiers.iter().flatten() {
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(import)
                    if !import.import_kind.is_type()
                        && import.imported.to_string() == CREATE_ELEMENT =>
                {
                    self.bindings
                        .jsx_import(&import.local, CREATE_ELEMENT.to_string());
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
                    self.bindings.jsx_namespace(default.local.symbol_id.get());
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                    self.bindings.jsx_namespace(namespace.local.symbol_id.get());
                }
                ImportDeclarationSpecifier::ImportSpecifier(_) => {}
            }
        }
    }
}
