use oxc_ast::ast::{BindingPattern, Expression, ImportDeclaration, VariableDeclarator};

use super::{
    Check, StylexBinding, StylexSource, readable_code, require_source, unbound_reference,
    unwrap_syntax_only,
};

impl Check<'_> {
    pub(super) fn required(&mut self, declarator: &VariableDeclarator<'_>) -> bool {
        let Some(Expression::CallExpression(call)) =
            declarator.init.as_ref().map(unwrap_syntax_only)
        else {
            return false;
        };
        let Some(source) = require_source(call) else {
            return false;
        };
        if !matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(loader) if unbound_reference(&self.bindings.scoping, loader))
        {
            return false;
        }
        let source = StylexSource::classify(source, self.package);
        if source == StylexSource::Root {
            match &declarator.id {
                BindingPattern::BindingIdentifier(_) => {}
                BindingPattern::ObjectPattern(object)
                    if object.properties.iter().any(|property| {
                        crate::utils::get_string_by_property_key(&property.key)
                            .is_some_and(|name| name == "stylex")
                    }) => {}
                _ => return true,
            }
        }
        match source {
            StylexSource::Other => return false,
            StylexSource::TypesOnly => self.reject(
                call.span.start,
                &readable_code(&call.callee),
                "compat/stylex has only type exports; use the real stylex entrypoint",
            ),
            StylexSource::Root | StylexSource::Upstream | StylexSource::Dedicated => {}
        }
        if call.arguments.len() != 1 {
            self.reject(
                call.span.start,
                "require",
                "use one literal module-source argument in a direct loader call",
            );
        }
        match &declarator.id {
            BindingPattern::BindingIdentifier(local) => {
                if local.symbol_id.get().is_some_and(|symbol| {
                    matches!(
                        self.bindings.stylex.binding(symbol),
                        Some(StylexBinding::Invalid)
                    )
                }) {
                    self.reject(
                        local.span.start,
                        local.name.as_str(),
                        "use an advertised StyleX export",
                    );
                }
            }
            BindingPattern::ObjectPattern(object) => {
                if object.rest.is_some() && source != StylexSource::Root {
                    self.reject(
                        object.span.start,
                        "require destructuring rest",
                        "use direct named bindings without rest/default/nested patterns",
                    );
                }
                for property in &object.properties {
                    if source == StylexSource::Root
                        && crate::utils::get_string_by_property_key(&property.key)
                            .is_none_or(|name| name != "stylex")
                    {
                        continue;
                    }
                    match property.value.get_binding_identifier() {
                        Some(local) if !property.computed => {
                            if local.symbol_id.get().is_some_and(|symbol| {
                                matches!(
                                    self.bindings.stylex.binding(symbol),
                                    Some(StylexBinding::Invalid)
                                )
                            }) {
                                self.reject(local.span.start, local.name.as_str(), "use an advertised named StyleX export; the dedicated subpath has no default");
                            }
                        }
                        _ => self.reject(
                            property.span.start,
                            "require destructuring",
                            "use direct named bindings without computed/default/nested patterns",
                        ),
                    }
                }
            }
            _ => self.reject(
                declarator.span.start,
                "require binding",
                "use a namespace identifier or simple named destructuring",
            ),
        }
        true
    }

    pub(super) fn imported(&mut self, import: &ImportDeclaration<'_>) {
        if import.import_kind.is_type() {
            return;
        }
        if StylexSource::classify(import.source.value.as_str(), self.package)
            == StylexSource::TypesOnly
        {
            self.reject(
                import.source.span.start,
                import.source.value.as_str(),
                "compat/stylex has only type exports; use the real stylex entrypoint",
            );
            return;
        }
        for specifier in import.specifiers.iter().flatten() {
            if matches!(specifier, oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(named) if named.import_kind.is_type())
            {
                continue;
            }
            let local = specifier.local();
            if local.symbol_id.get().is_some_and(|symbol| {
                matches!(
                    self.bindings.stylex.binding(symbol),
                    Some(StylexBinding::Invalid)
                )
            }) {
                self.reject(local.span.start, local.name.as_str(), "use an advertised value export; compat/stylex is types-only and the dedicated subpath has no default");
            }
        }
    }

    pub(super) fn exported(&mut self, export: &oxc_ast::ast::ExportDeclaration<'_>) {
        if let oxc_ast::ast::Declaration::VariableDeclaration(declaration) = &export.declaration {
            for declarator in &declaration.declarations {
                for local in declarator.id.get_binding_identifiers() {
                    if local.symbol_id.get().is_some_and(|symbol| {
                        matches!(
                            self.bindings.stylex.binding(symbol),
                            Some(
                                StylexBinding::Root
                                    | StylexBinding::Namespace
                                    | StylexBinding::UpstreamNamespace
                                    | StylexBinding::Function(_)
                                    | StylexBinding::Invalid
                            )
                        )
                    }) {
                        self.reject(
                            local.span.start,
                            local.name.as_str(),
                            "an API namespace/function cannot be exported to runtime code",
                        );
                    }
                }
            }
        }
    }
}
