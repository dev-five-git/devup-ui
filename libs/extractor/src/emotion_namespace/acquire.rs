//! The ways a module can get hold of `@emotion/css` other than a static ES
//! import. Each one hands out a runtime object, so styling members read from it
//! are build errors; recognition is binding-aware, so a local `require`,
//! `module` or loader function of the same spelling is left alone.
use super::{AstKind, Normalizer, Root};
use oxc_ast::ast::{
    AwaitExpression, BindingPattern, CallExpression, Expression, IdentifierReference,
    ImportDeclaration, ImportDeclarationSpecifier, VariableDeclarator,
};

const EMOTION: &str = "@emotion/css";

const DYNAMIC_IMPORT_REQUIREMENT: &str = "a dynamic import hands out a runtime namespace; `await` it directly for runtime-only members, or use `import * as ns from '@emotion/css'`";

impl Normalizer<'_, '_> {
    /// The namespace `expression` acquires, when it is a recognized acquisition
    /// of the Emotion root.
    pub(super) fn acquisition(&self, expression: &Expression<'_>) -> Option<Root> {
        match expression.get_inner_expression() {
            Expression::CallExpression(call) => self.require_call(call),
            Expression::AwaitExpression(wait) => self.awaited_import(wait),
            _ => None,
        }
    }

    pub(super) fn require_call(&self, call: &CallExpression<'_>) -> Option<Root> {
        let [argument] = call.arguments.as_slice() else {
            return None;
        };
        (self.loader(&call.callee) && self.names_emotion(argument.as_expression()?))
            .then_some(Root::Require(call.span))
    }

    pub(super) fn awaited_import(&self, wait: &AwaitExpression<'_>) -> Option<Root> {
        let Expression::ImportExpression(import) = wait.argument.get_inner_expression() else {
            return None;
        };
        self.names_emotion(&import.source)
            .then_some(Root::Require(wait.span))
    }

    fn names_emotion(&self, expression: &Expression<'_>) -> bool {
        self.string(expression).as_deref() == Some(EMOTION)
    }

    fn is_global(&self, identifier: &IdentifierReference<'_>, name: &str) -> bool {
        identifier.name == name
            && identifier.reference_id.get().is_some_and(|id| {
                self.semantic
                    .scoping()
                    .get_reference(id)
                    .symbol_id()
                    .is_none()
            })
    }

    /// Whether `expression` is a function that loads modules: the global
    /// `require`, `module.require`, a `createRequire(...)` result, or a binding
    /// holding one of them.
    fn loader(&self, expression: &Expression<'_>) -> bool {
        match expression.get_inner_expression() {
            Expression::Identifier(identifier) => {
                self.is_global(identifier, "require")
                    || self
                        .symbol(expression)
                        .is_some_and(|symbol| self.loaders.contains(&symbol))
            }
            Expression::StaticMemberExpression(member) => {
                member.property.name == "require"
                    && matches!(member.object.get_inner_expression(), Expression::Identifier(object) if self.is_global(object, "module"))
            }
            Expression::CallExpression(call) => self.create_require(&call.callee),
            _ => false,
        }
    }

    fn create_require(&self, callee: &Expression<'_>) -> bool {
        match callee.get_inner_expression() {
            Expression::Identifier(_) => self
                .symbol(callee)
                .is_some_and(|symbol| self.module_api.contains(&symbol)),
            Expression::StaticMemberExpression(member) => {
                member.property.name == "createRequire"
                    && self
                        .symbol(&member.object)
                        .is_some_and(|symbol| self.module_api.contains(&symbol))
            }
            _ => false,
        }
    }

    pub(super) fn collect_loader(
        &mut self,
        declarator: &VariableDeclarator<'_>,
        init: &Expression<'_>,
    ) {
        if let BindingPattern::BindingIdentifier(identifier) = &declarator.id
            && let Some(symbol) = identifier.symbol_id.get()
            && self.loader(init)
        {
            self.loaders.insert(symbol);
        }
    }

    /// Remember the bindings of `createRequire` and of the `module` namespace.
    pub(super) fn collect_module_api(&mut self, import: &ImportDeclaration<'_>) {
        if !matches!(import.source.value.as_str(), "module" | "node:module")
            || import.import_kind.is_type()
        {
            return;
        }
        for specifier in import.specifiers.iter().flatten() {
            let value_import = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(named) => {
                    named.imported.name() == "createRequire" && !named.import_kind.is_type()
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_)
                | ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => true,
            };
            if value_import && let Some(symbol) = specifier.local().symbol_id.get() {
                self.module_api.insert(symbol);
            }
        }
    }

    /// A dynamic import that is not directly awaited hands the namespace to
    /// code this pass cannot follow.
    pub(super) fn reject_unawaited_imports(&mut self) {
        let semantic = self.semantic;
        for node in semantic.nodes().iter() {
            let AstKind::ImportExpression(import) = node.kind() else {
                continue;
            };
            let expression = self.expression_node(node.id());
            if self.names_emotion(&import.source)
                && !matches!(
                    semantic.nodes().parent_kind(expression),
                    AstKind::AwaitExpression(_)
                )
            {
                self.error(import.span, DYNAMIC_IMPORT_REQUIREMENT);
            }
        }
    }
}
