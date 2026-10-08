use super::{AstKind, Binding, Cow, GetSpan, Normalizer, Root, macro_name};
use oxc_ast::ast::{BindingPattern, BindingProperty, Expression, VariableDeclarationKind};
use oxc_syntax::node::NodeId;

impl Normalizer<'_, '_> {
    /// Resolve const aliases to a fixed point, including aliases in nested scopes.
    /// A `require()` result may also be held by `let` and `var`, as `CommonJS` does.
    pub(super) fn collect_bindings(&mut self) {
        loop {
            let before = self.bindings.len() + self.loaders.len();
            for node in self.semantic.nodes().iter() {
                let AstKind::VariableDeclarator(declarator) = node.kind() else {
                    continue;
                };
                let Some(init) = &declarator.init else {
                    continue;
                };
                self.collect_loader(declarator, init);
                if !self.alias_declaration(node.id(), init) {
                    continue;
                }
                match &declarator.id {
                    BindingPattern::BindingIdentifier(identifier) => {
                        if let (Some(symbol), Some(value)) =
                            (identifier.symbol_id.get(), self.value(init))
                        {
                            if matches!(value, Binding::String(_))
                                && self
                                    .semantic
                                    .scoping()
                                    .get_resolved_reference_ids(symbol)
                                    .iter()
                                    .any(|reference| {
                                        self.semantic.scoping().get_reference(*reference).is_write()
                                    })
                            {
                                continue;
                            }
                            if !matches!(value, Binding::String(_)) {
                                self.aliases.insert(init.span());
                            }
                            self.bindings.insert(symbol, value);
                        }
                    }
                    BindingPattern::ObjectPattern(pattern) => {
                        if let Some(Binding::Namespace(root)) = self.value(init) {
                            self.aliases.insert(init.span());
                            for property in &pattern.properties {
                                let key = self.property_key(property);
                                let api = key.as_deref().and_then(macro_name);
                                if key.is_some() && api.is_none() {
                                    self.runtime.insert(root);
                                }
                                if let (Some(api), BindingPattern::BindingIdentifier(identifier)) =
                                    (api, &property.value)
                                    && matches!(root, Root::Module(_))
                                    && let Some(symbol) = identifier.symbol_id.get()
                                {
                                    self.bindings.insert(symbol, Binding::Macro(api));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            if self.bindings.len() + self.loaders.len() == before {
                break;
            }
        }
    }

    fn alias_declaration(&self, declarator: NodeId, init: &Expression<'_>) -> bool {
        matches!(
            self.semantic.nodes().parent_kind(declarator),
            AstKind::VariableDeclaration(declaration) if declaration.kind == VariableDeclarationKind::Const
        ) || matches!(self.value(init), Some(Binding::Namespace(Root::Require(_))))
    }

    pub(super) fn property_key(&self, property: &BindingProperty<'_>) -> Option<String> {
        if property.computed {
            return self.string(property.key.as_expression()?);
        }
        crate::utils::get_str_by_property_key(&property.key).map(Cow::into_owned)
    }
}
