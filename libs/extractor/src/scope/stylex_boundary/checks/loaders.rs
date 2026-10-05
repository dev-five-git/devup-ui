use oxc_ast::ast::{CallExpression, Expression, ImportExpression};

use super::{Check, StylexSource, require_source, unbound_reference, unwrap_syntax_only};
use crate::utils::{get_string_by_literal_expression, readable_code};

impl Check<'_> {
    pub(super) fn root_loader(&self, expression: &Expression<'_>) -> bool {
        matches!(unwrap_syntax_only(expression), Expression::CallExpression(call)
            if require_source(call).is_some_and(|source| StylexSource::classify(source, self.package) == StylexSource::Root)
                && matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(loader) if unbound_reference(&self.bindings.scoping, loader)))
    }

    pub(super) fn runtime_loader(&mut self, call: &CallExpression<'_>) {
        if let Some(source) = require_source(call)
            && matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(loader) if unbound_reference(&self.bindings.scoping, loader))
        {
            match StylexSource::classify(source, self.package) {
                StylexSource::TypesOnly => self.reject(call.span.start, &readable_code(&call.callee), "compat/stylex has only type exports; use the real stylex entrypoint"),
                StylexSource::Root if self.root_member => {},
                StylexSource::Root | StylexSource::Upstream | StylexSource::Dedicated => self.reject(call.span.start, &readable_code(&call.callee), "bind the loader result to an unchanged namespace or simple named imports before calling an API"),
                StylexSource::Other => {},
            }
        }
    }

    pub(super) fn dynamic_loader(&mut self, import: &ImportExpression<'_>) {
        if get_string_by_literal_expression(&import.source).is_some_and(|source| {
            StylexSource::classify(&source, self.package) == StylexSource::TypesOnly
        }) {
            self.reject(
                import.span.start,
                "import(...)",
                "compat/stylex has only type exports; use the real stylex entrypoint",
            );
        }
    }
}
