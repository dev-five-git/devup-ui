use super::{Shape, Walker};
use oxc_ast::ast::{ImportDeclarationSpecifier, Program, Statement};
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{collections::BTreeSet, rc::Rc};

#[derive(Default)]
pub(crate) struct Facts {
    pub bindings: FxHashMap<SymbolId, Rc<Shape>>,
    pub dependencies: BTreeSet<String>,
    pub errors: Vec<(Span, String)>,
    pub failure: Option<String>,
}

impl Facts {
    pub(crate) fn resolve(
        program: &Program<'_>,
        origin: (&str, &str),
        resolver: Option<&crate::ModuleResolver>,
    ) -> Self {
        let (filename, package) = origin;
        let no_modules = |_: &str, _: &str| None;
        let mut walker = Walker {
            resolver: resolver.unwrap_or(&no_modules),
            package,
            native: true,
            modules: FxHashMap::default(),
            dependencies: BTreeSet::new(),
        };
        let mut facts = Self::default();
        for statement in &program.body {
            let Statement::ImportDeclaration(import) = statement else {
                continue;
            };
            for specifier in import.specifiers.iter().flatten() {
                let name = match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        Some(specifier.imported.name().to_string())
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        Some("default".to_string())
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => None,
                };
                let Some(symbol) = specifier.local().symbol_id.get() else {
                    continue;
                };
                let terminal =
                    walker.native_import(import.source.value.as_str(), filename, name.as_deref());
                if let Some(shape) = walker.native_shape(terminal, &mut FxHashSet::default()) {
                    match shape.as_ref() {
                        Shape::Failed(message) => {
                            facts.errors.push((specifier.span(), message.clone()));
                        }
                        Shape::OriginalFailure(message) => {
                            facts.failure.get_or_insert_with(|| message.clone());
                        }
                        Shape::Api(_) | Shape::Namespace(_) | Shape::PackageNamespace(_) => {
                            facts.bindings.insert(symbol, shape);
                        }
                    }
                }
            }
        }
        facts.dependencies = walker.dependencies;
        facts
    }
}
