use oxc_ast::ast::{ComputedMemberExpression, JSXMemberExpression, StaticMemberExpression};
use oxc_ast_visit::{Visit, walk};

use super::{Check, StylexBinding};
use crate::stylex::StylexFunction;
use crate::utils::{get_string_by_literal_expression, readable_code};

impl Check<'_> {
    pub(super) fn static_member(&mut self, member: &StaticMemberExpression<'_>) {
        if let Some(StylexBinding::Function(function)) =
            self.bindings.stylex_binding(&member.object)
        {
            if matches!(member.property.name.as_str(), "call" | "apply" | "bind") {
                self.reject_function(
                    &function,
                    (
                        member.span.start,
                        &format!("{}.{}", readable_code(&member.object), member.property.name),
                    ),
                    "invocation/binding member",
                );
            }
            return;
        }
        if matches!(
            self.bindings.stylex_binding(&member.object),
            Some(StylexBinding::Namespace | StylexBinding::UpstreamNamespace)
        ) && let Some(function) = StylexFunction::from_export_name(member.property.name.as_str())
        {
            self.reject_function(
                &function,
                (
                    member.span.start,
                    &format!("{}.{}", readable_code(&member.object), member.property.name),
                ),
                self.form,
            );
            return;
        }
        let loader = self.root_loader(&member.object);
        if loader && member.property.name == "stylex" {
            self.reject(member.span.start, "require(...).stylex", "bind the root loader to an unchanged namespace or simple stylex destructuring before using its API");
            return;
        }
        let previous = self.root_member;
        self.root_member = loader
            || matches!(
                self.bindings.stylex_binding(&member.object),
                Some(StylexBinding::Root)
            );
        self.visit_expression(&member.object);
        self.root_member = previous;
    }

    pub(super) fn computed_member(&mut self, member: &ComputedMemberExpression<'_>) {
        if let Some(StylexBinding::Function(function)) =
            self.bindings.stylex_binding(&member.object)
        {
            if get_string_by_literal_expression(&member.expression)
                .is_none_or(|name| matches!(name.as_ref(), "call" | "apply" | "bind"))
            {
                self.reject_function(
                    &function,
                    (
                        member.span.start,
                        &format!(
                            "{}[{}]",
                            readable_code(&member.object),
                            readable_code(&member.expression)
                        ),
                    ),
                    "computed member",
                );
            }
            self.visit_expression(&member.expression);
            return;
        }
        let loader = self.root_loader(&member.object);
        if loader
            && get_string_by_literal_expression(&member.expression)
                .is_none_or(|name| name.as_ref() == "stylex")
        {
            self.reject(
                member.span.start,
                "require(...)[...]",
                "bind the root module before selecting its compile-only stylex API",
            );
            return;
        }
        let previous = self.root_member;
        self.root_member = loader
            || matches!(
                self.bindings.stylex_binding(&member.object),
                Some(StylexBinding::Root)
            );
        self.visit_expression(&member.object);
        self.root_member = previous;
        self.visit_expression(&member.expression);
    }

    pub(super) fn jsx_member(&mut self, member: &JSXMemberExpression<'_>) {
        let previous = self.root_member;
        self.root_member = true;
        if member.property.name == "stylex"
            && matches!(&member.object, oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(root)
                if self.bindings.symbol(root).is_some_and(|symbol| matches!(self.bindings.stylex.binding(symbol), Some(StylexBinding::Root))))
        {
            self.reject(
                member.span.start,
                "stylex JSX member",
                "compile-only API namespaces cannot be rendered or escape as values",
            );
        }
        walk::walk_jsx_member_expression(self, member);
        self.root_member = previous;
    }
}
