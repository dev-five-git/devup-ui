use oxc_ast::ast::{BindingPattern, Expression, VariableDeclarator};

use super::Bindings;
use super::stylex_bindings::{StylexBinding, unbound_reference};
use super::stylex_sources::{StylexSource, require_source};
use crate::utils::get_string_by_property_key;

impl Bindings {
    /// Declare what `declarator` binds when it requires `package` or the JSX
    /// runtime: `const { css } = require('@devup-ui/react')`
    pub fn require(&mut self, declarator: &VariableDeclarator<'_>, package: &str) {
        let Some(Expression::CallExpression(call)) = declarator
            .init
            .as_ref()
            .map(crate::utils::unwrap_syntax_only)
        else {
            return;
        };
        let Some(source) = require_source(call) else {
            return;
        };
        let Expression::Identifier(callee) = crate::utils::unwrap_syntax_only(&call.callee) else {
            return;
        };
        if !unbound_reference(&self.scoping, callee) {
            return;
        }
        let source_kind = StylexSource::classify(source, package);
        if source_kind.is_api_module() || source_kind == StylexSource::TypesOnly {
            self.require_stylex(declarator, source_kind);
            return;
        }
        if call.arguments.len() != 1 {
            return;
        }
        let from_package = if matches!(source, "react/jsx-runtime" | "react") {
            false
        } else if source == package {
            true
        } else {
            return;
        };
        match &declarator.id {
            BindingPattern::BindingIdentifier(binding) if from_package => {
                self.namespace(binding.symbol_id.get());
                self.stylex
                    .insert(binding.symbol_id.get(), StylexBinding::Root);
            }
            BindingPattern::BindingIdentifier(binding) => {
                self.jsx_namespace(binding.symbol_id.get());
            }
            BindingPattern::ObjectPattern(object) => {
                let exports = object.properties.iter().filter_map(|prop| {
                    Some((
                        get_string_by_property_key(&prop.key)?,
                        prop.value.get_binding_identifier()?,
                    ))
                });
                for (name, local) in exports {
                    if from_package {
                        if name == "stylex" {
                            self.stylex_namespace(local.symbol_id.get());
                        } else {
                            self.export(local.symbol_id.get(), &name);
                        }
                    } else {
                        self.jsx_import(local, name);
                    }
                }
            }
            _ => {}
        }
    }

    fn require_stylex(&mut self, declarator: &VariableDeclarator<'_>, source: StylexSource) {
        match &declarator.id {
            BindingPattern::BindingIdentifier(binding) => {
                self.stylex.bind_namespace(binding.symbol_id.get(), source);
            }
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    if let (Some(name), Some(local)) = (
                        get_string_by_property_key(&property.key),
                        property.value.get_binding_identifier(),
                    ) {
                        self.stylex
                            .bind_export(local.symbol_id.get(), source, &name);
                    }
                }
            }
            _ => {}
        }
    }
}
