use oxc_ast::ast::{Expression, IdentifierReference, Program};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::stylex::StylexFunction;
use crate::utils::unwrap_syntax_only;

mod collect;

#[derive(Clone, Debug)]
pub(crate) enum StylexBinding {
    Root,
    RootDefault,
    UpstreamNamespace,
    Namespace,
    Function(StylexFunction),
    Invalid,
}

#[derive(Default)]
pub(crate) struct StylexBindings {
    bindings: FxHashMap<SymbolId, StylexBinding>,
}

impl StylexBindings {
    pub(crate) fn collect(program: &Program<'_>, scoping: &Scoping, package: &str) -> Self {
        collect::bindings(program, scoping, package)
    }

    pub(crate) fn insert(&mut self, symbol: Option<SymbolId>, binding: StylexBinding) {
        if let Some(symbol) = symbol {
            self.bindings.insert(symbol, binding);
        }
    }

    pub(crate) fn binding(&self, symbol: SymbolId) -> Option<&StylexBinding> {
        self.bindings.get(&symbol)
    }

    pub(crate) fn bind_export(
        &mut self,
        symbol: Option<SymbolId>,
        source: super::stylex_sources::StylexSource,
        name: &str,
    ) {
        if let Some(binding) = collect::exported(source, name) {
            self.insert(symbol, binding);
        }
    }

    pub(crate) fn bind_namespace(
        &mut self,
        symbol: Option<SymbolId>,
        source: super::stylex_sources::StylexSource,
    ) {
        if let Some(binding) = collect::namespace(source) {
            self.insert(symbol, binding);
        }
    }

    pub(crate) fn resolve(
        &self,
        expression: &Expression<'_>,
        symbol: &dyn Fn(&IdentifierReference<'_>) -> Option<SymbolId>,
    ) -> Option<StylexBinding> {
        match unwrap_syntax_only(expression) {
            Expression::Identifier(identifier) => self.bindings.get(&symbol(identifier)?).cloned(),
            Expression::StaticMemberExpression(member) => {
                match self.resolve(&member.object, symbol)? {
                    StylexBinding::Root => {
                        (member.property.name == "stylex").then_some(StylexBinding::Namespace)
                    }
                    StylexBinding::RootDefault => {
                        (member.property.name == "stylex").then_some(StylexBinding::Invalid)
                    }
                    StylexBinding::UpstreamNamespace if member.property.name == "default" => {
                        Some(StylexBinding::Namespace)
                    }
                    StylexBinding::Namespace | StylexBinding::UpstreamNamespace => Some(
                        StylexFunction::from_export_name(member.property.name.as_str())
                            .map_or(StylexBinding::Invalid, StylexBinding::Function),
                    ),
                    StylexBinding::Function(_) => None,
                    StylexBinding::Invalid => Some(StylexBinding::Invalid),
                }
            }
            Expression::ComputedMemberExpression(member) => {
                match self.resolve(&member.object, symbol)? {
                    StylexBinding::Root => {
                        match crate::utils::get_string_by_literal_expression(&member.expression) {
                            Some(name) if name.as_ref() != "stylex" => None,
                            Some(_) | None => Some(StylexBinding::Invalid),
                        }
                    }
                    StylexBinding::RootDefault => {
                        crate::utils::get_string_by_literal_expression(&member.expression)
                            .filter(|name| name.as_ref() == "stylex")
                            .map(|_| StylexBinding::Invalid)
                    }
                    StylexBinding::Function(_) => None,
                    StylexBinding::Namespace
                    | StylexBinding::UpstreamNamespace
                    | StylexBinding::Invalid => Some(StylexBinding::Invalid),
                }
            }
            Expression::ChainExpression(chain) => match &chain.expression {
                oxc_ast::ast::ChainElement::CallExpression(call) => self
                    .resolve(&call.callee, symbol)
                    .map(|_| StylexBinding::Invalid),
                oxc_ast::ast::ChainElement::StaticMemberExpression(member) => {
                    match self.resolve(&member.object, symbol)? {
                        StylexBinding::Root | StylexBinding::RootDefault
                            if member.property.name != "stylex" =>
                        {
                            None
                        }
                        StylexBinding::Root
                        | StylexBinding::RootDefault
                        | StylexBinding::Namespace
                        | StylexBinding::UpstreamNamespace
                        | StylexBinding::Function(_)
                        | StylexBinding::Invalid => Some(StylexBinding::Invalid),
                    }
                }
                oxc_ast::ast::ChainElement::ComputedMemberExpression(member) => {
                    match self.resolve(&member.object, symbol)? {
                        StylexBinding::Root | StylexBinding::RootDefault
                            if crate::utils::get_string_by_literal_expression(
                                &member.expression,
                            )
                            .is_some_and(|name| name.as_ref() != "stylex") =>
                        {
                            None
                        }
                        StylexBinding::Root
                        | StylexBinding::RootDefault
                        | StylexBinding::Namespace
                        | StylexBinding::UpstreamNamespace
                        | StylexBinding::Function(_)
                        | StylexBinding::Invalid => Some(StylexBinding::Invalid),
                    }
                }
                oxc_ast::ast::ChainElement::PrivateFieldExpression(member) => self
                    .resolve(&member.object, symbol)
                    .map(|_| StylexBinding::Invalid),
                oxc_ast::ast::ChainElement::TSNonNullExpression(inner) => self
                    .resolve(&inner.expression, symbol)
                    .map(|_| StylexBinding::Invalid),
            },
            _ => None,
        }
    }

    pub(crate) fn function(
        &self,
        expression: &Expression<'_>,
        symbol: &dyn Fn(&IdentifierReference<'_>) -> Option<SymbolId>,
    ) -> Option<StylexFunction> {
        match self.resolve(expression, symbol)? {
            StylexBinding::Function(function) => Some(function),
            StylexBinding::Root
            | StylexBinding::RootDefault
            | StylexBinding::Namespace
            | StylexBinding::UpstreamNamespace
            | StylexBinding::Invalid => None,
        }
    }
}

pub(crate) fn symbol(scoping: &Scoping, identifier: &IdentifierReference<'_>) -> Option<SymbolId> {
    scoping
        .get_reference(identifier.reference_id.get()?)
        .symbol_id()
}

pub(crate) fn unbound_reference(scoping: &Scoping, identifier: &IdentifierReference<'_>) -> bool {
    identifier
        .reference_id
        .get()
        .is_some_and(|reference| scoping.get_reference(reference).symbol_id().is_none())
}

pub(crate) fn unchanged(scoping: &Scoping, symbol: SymbolId) -> bool {
    scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .all(|reference| {
            let flags = scoping.get_reference(*reference).flags();
            !flags.is_write() && !flags.is_member_write_target()
        })
}

#[cfg(test)]
mod tests;
