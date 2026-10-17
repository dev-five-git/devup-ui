use oxc_ast::ast::{Argument, BindingPattern, Expression, VariableDeclarator};

use super::Bindings;
use crate::utils::get_string_by_property_key;

impl Bindings {
    /// Declare what `declarator` binds when it requires `package` or the JSX
    /// runtime: `const { css } = require('@devup-ui/react')`
    pub fn require(&mut self, declarator: &VariableDeclarator<'_>, package: &str) {
        let Some(Expression::CallExpression(call)) = &declarator.init else {
            return;
        };
        let (Expression::Identifier(callee), [Argument::StringLiteral(source)]) =
            (&call.callee, call.arguments.as_slice())
        else {
            return;
        };
        if callee.name != "require" || self.symbol(callee).is_some() {
            return;
        }
        let from_package = if source.value == "react/jsx-runtime" {
            false
        } else if source.value == package {
            true
        } else {
            return;
        };
        match &declarator.id {
            BindingPattern::BindingIdentifier(binding) if from_package => {
                self.namespace(binding.symbol_id.get());
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
                        self.export(local.symbol_id.get(), &name);
                    } else {
                        self.jsx_import(local, name);
                    }
                }
            }
            _ => {}
        }
    }
}
