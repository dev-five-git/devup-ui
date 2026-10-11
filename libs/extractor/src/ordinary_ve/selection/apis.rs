use crate::barrel::native::{Facts, Shape};
use oxc_ast::ast::{Expression, ImportDeclarationSpecifier, Program, Statement};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;
use std::{collections::BTreeSet, rc::Rc};
#[path = "api_aliases.rs"]
mod aliases;

use super::plan::{Binding, ImportBinding, ImportName, NativeBinding};
use crate::utils::unwrap_syntax_only;

pub(super) struct Apis<'s, 'a> {
    pub semantic: &'s Semantic<'a>,
    pub bindings: FxHashMap<SymbolId, NativeBinding>,
    pub imports: FxHashMap<SymbolId, ImportBinding>,
    pub shapes: FxHashMap<SymbolId, Rc<Shape>>,
    pub dependencies: BTreeSet<String>,
    pub errors: Vec<(oxc_span::Span, String)>,
    pub failure: Option<String>,
}

impl<'s, 'a> Apis<'s, 'a> {
    #[cfg(test)]
    pub fn for_package(program: &Program<'a>, semantic: &'s Semantic<'a>, package: &str) -> Self {
        Self::with_facts(
            program,
            semantic,
            Facts::resolve(program, ("", package), None),
        )
    }

    pub fn with_facts(program: &Program<'a>, semantic: &'s Semantic<'a>, facts: Facts) -> Self {
        let mut apis = Self {
            semantic,
            bindings: FxHashMap::default(),
            imports: FxHashMap::default(),
            shapes: facts.bindings,
            dependencies: facts.dependencies,
            errors: facts.errors,
            failure: facts.failure,
        };
        for statement in &program.body {
            let Statement::ImportDeclaration(import) = statement else {
                continue;
            };
            for specifier in import.specifiers.iter().flatten() {
                let local = specifier.local();
                let Some(symbol) = local.symbol_id.get() else {
                    continue;
                };
                let (imported, erased) = match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        let name = specifier.imported.name();
                        (
                            ImportName::Named(name.to_string()),
                            specifier.import_kind.is_type(),
                        )
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                        (ImportName::Namespace, false)
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        (ImportName::Default, false)
                    }
                };
                let erased = erased || import.import_kind.is_type();
                if erased && apis.shapes.contains_key(&symbol) {
                    for reference in semantic.scoping().get_resolved_reference_ids(symbol) {
                        let data = semantic.scoping().get_reference(*reference);
                        if data.is_value() {
                            apis.errors.push((
                                semantic.nodes().kind(data.node_id()).span(),
                                "native API input comes from a type-only import".to_string(),
                            ));
                        }
                    }
                }
                let native = apis
                    .shapes
                    .get(&symbol)
                    .and_then(|shape| Self::native(shape))
                    .filter(|_| !erased);
                if let Some(native) = native {
                    apis.bindings.insert(symbol, native);
                }
                apis.imports.insert(
                    symbol,
                    ImportBinding {
                        binding: Binding {
                            symbol,
                            span: local.span,
                            name: local.name.to_string(),
                        },
                        declaration: import.span,
                        specifier: specifier.span(),
                        source: import.source.value.to_string(),
                        imported,
                        erased,
                        native,
                        preserved: false,
                    },
                );
            }
        }
        apis.follow_aliases();
        apis
    }

    const fn native(shape: &Shape) -> Option<NativeBinding> {
        match shape {
            Shape::Api(api) => Some(NativeBinding::Named { api }),
            Shape::Namespace(_) | Shape::PackageNamespace(_) => Some(NativeBinding::Namespace),
            Shape::Failed(_) | Shape::OriginalFailure(_) => None,
        }
    }

    pub fn symbol(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(identifier) = unwrap_syntax_only(expression) else {
            return None;
        };
        self.semantic
            .scoping()
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
    }

    pub fn binding(&self, expression: &Expression<'_>) -> Option<NativeBinding> {
        self.shape(expression).as_deref().and_then(Self::native)
    }

    pub fn shape(&self, expression: &Expression<'_>) -> Option<Rc<Shape>> {
        match unwrap_syntax_only(expression) {
            Expression::Identifier(_) => self.shapes.get(&self.symbol(expression)?).cloned(),
            Expression::StaticMemberExpression(member) => self
                .shape(&member.object)?
                .member(member.property.name.as_str()),
            Expression::ComputedMemberExpression(member) => self
                .shape(&member.object)?
                .member(&self.key(&member.expression)?),
            _ => None,
        }
    }

    pub fn member(&self, object: &Expression<'_>, name: &str) -> Option<NativeBinding> {
        self.shape(object)?
            .member(name)
            .as_deref()
            .and_then(Self::native)
    }
}
