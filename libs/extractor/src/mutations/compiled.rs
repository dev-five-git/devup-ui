//! Only extractor-owned calls, identified by their lexical import, are removed.

use super::Context;
use crate::{css_prop::binding_of, stylex::StylexFunction, utils::unwrap_syntax_only};
use oxc_ast::{AstKind, ast::Expression};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

impl Context<'_, '_> {
    pub(super) fn compiled_call(&self, callee: &Expression<'_>) -> bool {
        self.compiled_call_in(callee, &mut FxHashSet::default(), None)
    }

    fn compiled_call_in(
        &self,
        callee: &Expression<'_>,
        seen: &mut FxHashSet<SymbolId>,
        inherited: Option<&str>,
    ) -> bool {
        let mut expression = callee;
        let mut member = inherited;
        loop {
            match unwrap_syntax_only(expression) {
                Expression::StaticMemberExpression(value) => {
                    member = Some(value.property.name.as_str());
                    expression = &value.object;
                }
                Expression::ComputedMemberExpression(value) => {
                    let Expression::StringLiteral(key) = unwrap_syntax_only(&value.expression)
                    else {
                        return false;
                    };
                    member = Some(key.value.as_str());
                    expression = &value.object;
                }
                Expression::CallExpression(value) => expression = &value.callee,
                Expression::Identifier(identifier) => {
                    let Some(symbol) = binding_of(self.scoping, identifier) else {
                        return false;
                    };
                    if !seen.insert(symbol) {
                        return false;
                    }
                    let declaration = self.nodes.kind(self.scoping.symbol_declaration(symbol));
                    if let AstKind::VariableDeclarator(value) = declaration
                        && matches!(self.nodes.parent_kind(self.scoping.symbol_declaration(symbol)),
                            AstKind::VariableDeclaration(declaration) if declaration.kind == oxc_ast::ast::VariableDeclarationKind::Const)
                        && !self
                            .scoping
                            .get_resolved_reference_ids(symbol)
                            .iter()
                            .any(|id| self.scoping.get_reference(*id).is_write())
                        && let Some(init) = value.init.as_ref().map(unwrap_syntax_only)
                        && matches!(
                            init,
                            Expression::Identifier(_)
                                | Expression::StaticMemberExpression(_)
                                | Expression::ComputedMemberExpression(_)
                        )
                    {
                        return self.compiled_call_in(init, seen, member);
                    }
                    if !self.is_style_reference(identifier) {
                        return false;
                    }
                    let stylex = self.nodes.ancestor_kinds(self.scoping.symbol_declaration(symbol)).any(|kind|
                        matches!(kind, AstKind::ImportDeclaration(import) if import.source.value == crate::STYLEX_PACKAGE))
                        || matches!(declaration, AstKind::VariableDeclarator(value)
                            if matches!(value.init.as_ref().map(unwrap_syntax_only), Some(Expression::CallExpression(call))
                                if matches!(call.arguments.first(), Some(oxc_ast::ast::Argument::StringLiteral(source)) if source.value == crate::STYLEX_PACKAGE)));
                    let default_styled = self.nodes.ancestor_kinds(self.scoping.symbol_declaration(symbol)).any(|kind|
                        matches!(kind, AstKind::ImportDeclaration(import) if matches!(import.source.value.as_str(), "styled-components" | "@emotion/styled")));
                    let (export, namespace): (std::borrow::Cow<'_, str>, bool) = match declaration {
                        AstKind::ImportSpecifier(value) => {
                            (value.imported.name().as_str().into(), false)
                        }
                        AstKind::ImportDefaultSpecifier(_) if default_styled => {
                            ("styled".into(), false)
                        }
                        AstKind::ImportDefaultSpecifier(_)
                        | AstKind::ImportNamespaceSpecifier(_) => ("".into(), true),
                        AstKind::VariableDeclarator(value) => {
                            let Some(Expression::CallExpression(call)) =
                                value.init.as_ref().map(unwrap_syntax_only)
                            else {
                                return false;
                            };
                            if !matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(callee)
                                if callee.name == "require" && binding_of(self.scoping, callee).is_none())
                            {
                                return false;
                            }
                            match &value.id {
                                oxc_ast::ast::BindingPattern::BindingIdentifier(_) => {
                                    ("".into(), true)
                                }
                                oxc_ast::ast::BindingPattern::ObjectPattern(pattern) => {
                                    let Some(key) = pattern
                                        .properties
                                        .iter()
                                        .find(|property| {
                                            property
                                                .value
                                                .get_identifier_name()
                                                .is_some_and(|name| name == identifier.name)
                                        })
                                        .and_then(|property| property.key.static_name())
                                    else {
                                        return false;
                                    };
                                    (key, false)
                                }
                                _ => return false,
                            }
                        }
                        _ => return false,
                    };
                    let name = if namespace {
                        member.unwrap_or("")
                    } else {
                        export.as_ref()
                    };
                    return if stylex {
                        StylexFunction::from_export_name(name).is_some()
                    } else {
                        matches!(
                            name,
                            "css" | "globalCss" | "keyframes" | "createGlobalStyle" | "styled"
                        )
                    };
                }
                _ => return false,
            }
        }
    }
}

#[cfg(test)]
#[path = "compiled_tests.rs"]
mod tests;
