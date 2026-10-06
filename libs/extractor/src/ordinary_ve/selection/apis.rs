use oxc_ast::{
    AstKind,
    ast::{
        BindingPattern, Expression, ImportDeclarationSpecifier, Program, Statement,
        VariableDeclarationKind,
    },
};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use super::plan::{Binding, ImportBinding, ImportName, NativeBinding};
use crate::utils::{get_string_by_literal_expression, unwrap_syntax_only};

pub(super) struct Apis<'s, 'a> {
    pub semantic: &'s Semantic<'a>,
    pub bindings: FxHashMap<SymbolId, NativeBinding>,
    pub imports: FxHashMap<SymbolId, ImportBinding>,
}

impl<'s, 'a> Apis<'s, 'a> {
    pub fn for_package(program: &Program<'a>, semantic: &'s Semantic<'a>, package: &str) -> Self {
        let mut apis = Self {
            semantic,
            bindings: FxHashMap::default(),
            imports: FxHashMap::default(),
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
                let (imported, erased, native) = match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        let name = specifier.imported.name();
                        (
                            ImportName::Named(name.to_string()),
                            specifier.import_kind.is_type(),
                            Self::name(name.as_str()).map(|api| NativeBinding::Named { api }),
                        )
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                        (ImportName::Namespace, false, Some(NativeBinding::Namespace))
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        (ImportName::Default, false, None)
                    }
                };
                let erased = erased || import.import_kind.is_type();
                let native = native.filter(|_| !erased && import.source.value == package);
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
                    },
                );
            }
        }
        apis.follow_aliases();
        apis
    }

    fn name(name: &str) -> Option<&'static str> {
        super::super::APIS.iter().copied().find(|api| *api == name)
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
        match unwrap_syntax_only(expression) {
            Expression::Identifier(_) => self.bindings.get(&self.symbol(expression)?).copied(),
            Expression::StaticMemberExpression(member) => {
                self.member(&member.object, member.property.name.as_str())
            }
            Expression::ComputedMemberExpression(member) => self.member(
                &member.object,
                &get_string_by_literal_expression(unwrap_syntax_only(&member.expression))?,
            ),
            _ => None,
        }
    }

    pub fn member(&self, object: &Expression<'_>, name: &str) -> Option<NativeBinding> {
        match self.binding(object)? {
            NativeBinding::Namespace => Some(NativeBinding::Named {
                api: Self::name(name)?,
            }),
            NativeBinding::Named { .. } => None,
        }
    }

    fn follow_aliases(&mut self) {
        let semantic = self.semantic;
        loop {
            let before = self.bindings.len();
            for node in semantic.nodes().iter() {
                let AstKind::VariableDeclarator(declarator) = node.kind() else {
                    continue;
                };
                let AstKind::VariableDeclaration(declaration) =
                    semantic.nodes().parent_kind(node.id())
                else {
                    continue;
                };
                if declaration.kind != VariableDeclarationKind::Const || declaration.declare {
                    continue;
                }
                let Some(binding) = declarator.init.as_ref().and_then(|init| self.binding(init))
                else {
                    continue;
                };
                self.alias(&declarator.id, binding);
            }
            if self.bindings.len() == before {
                break;
            }
        }
    }

    fn alias(&mut self, pattern: &BindingPattern<'_>, binding: NativeBinding) {
        match pattern {
            BindingPattern::BindingIdentifier(id) => {
                if let Some(symbol) = id.symbol_id.get() {
                    self.bindings.entry(symbol).or_insert(binding);
                }
            }
            BindingPattern::ObjectPattern(object) => match binding {
                NativeBinding::Namespace => {
                    for property in &object.properties {
                        if let Some(api) =
                            property.key.static_name().as_deref().and_then(Self::name)
                        {
                            self.alias(&property.value, NativeBinding::Named { api });
                        }
                    }
                }
                NativeBinding::Named { .. } => {}
            },
            BindingPattern::AssignmentPattern(pattern) => self.alias(&pattern.left, binding),
            BindingPattern::ArrayPattern(_) => {}
        }
    }
}
