use oxc_ast::ast::{Expression, ImportDeclaration, Program, TSType, VariableDeclarator};
use oxc_ast_visit::{Visit, walk};
use oxc_span::GetSpan;

use crate::scope::Bindings;
use crate::scope::stylex_bindings::{StylexBinding, unbound_reference};
use crate::scope::stylex_sources::{StylexSource, require_source};
use crate::utils::{build_time_error, readable_code, unwrap_syntax_only};

mod declarations;
mod exports;

pub(super) struct Check<'s> {
    bindings: &'s Bindings,
    package: &'s str,
    diagnosed: &'s [(u32, String)],
    errors: Vec<(u32, String)>,
    root_member: bool,
}

impl<'s> Check<'s> {
    pub(super) const fn new(
        bindings: &'s Bindings,
        package: &'s str,
        diagnosed: &'s [(u32, String)],
    ) -> Self {
        Self {
            bindings,
            package,
            diagnosed,
            errors: Vec::new(),
            root_member: false,
        }
    }

    pub(super) fn errors(mut self, program: &Program<'_>) -> Vec<(u32, String)> {
        self.visit_program(program);
        self.errors
    }
    fn root_loader(&self, expression: &Expression<'_>) -> bool {
        matches!(unwrap_syntax_only(expression), Expression::CallExpression(call)
            if require_source(call).is_some_and(|source| StylexSource::classify(source, self.package) == StylexSource::Root)
                && matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(loader) if unbound_reference(&self.bindings.scoping, loader)))
    }

    fn reject(&mut self, at: u32, code: &str, requirement: &str) {
        if self.diagnosed.iter().any(|(offset, _)| *offset == at) {
            return;
        }
        self.errors
            .push((at, build_time_error("StyleX API", code, requirement)));
    }
}

impl<'a> Visit<'a> for Check<'_> {
    fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
        self.imported(import);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if self.required(declarator) {
            return;
        }
        if let Some(local) = declarator.id.get_binding_identifier()
            && let Some(symbol) = local.symbol_id.get()
            && matches!(
                self.bindings.stylex.binding(symbol),
                Some(
                    StylexBinding::Root
                        | StylexBinding::RootDefault
                        | StylexBinding::Namespace
                        | StylexBinding::UpstreamNamespace
                        | StylexBinding::Function(_)
                )
            )
            && let Some(init) = &declarator.init
            && self.bindings.stylex_binding(init).is_some()
        {
            return;
        }
        walk::walk_variable_declarator(self, declarator);
    }

    fn visit_expression(&mut self, expression: &Expression<'a>) {
        if matches!(expression, Expression::CallExpression(call)
            if self.diagnosed.iter().any(|(offset, _)| *offset >= call.span.start && *offset < call.span.end))
        {
            return;
        }
        match self.bindings.stylex_binding(expression) {
            Some(
                StylexBinding::Namespace
                | StylexBinding::UpstreamNamespace
                | StylexBinding::Function(_)
                | StylexBinding::Invalid,
            ) => {
                self.reject(expression.span().start, &readable_code(expression), "API calls must be compiled directly; a remaining API/namespace/function escape or dynamic member cannot run at runtime");
            }
            Some(StylexBinding::Root) => {
                if !self.root_member
                    || matches!(expression, Expression::Identifier(identifier)
                    if self.bindings.symbol(identifier).is_some_and(|symbol| !super::super::stylex_bindings::unchanged(&self.bindings.scoping, symbol)))
                {
                    self.reject(expression.span().start, &readable_code(expression), "read runtime root exports directly and keep the module unchanged; its compile-only stylex namespace cannot escape");
                } else {
                    walk::walk_expression(self, expression);
                }
            }
            Some(StylexBinding::RootDefault) | None => walk::walk_expression(self, expression),
        }
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        if let Some(source) = require_source(call)
            && (StylexSource::classify(source, self.package).is_api_module()
                || StylexSource::classify(source, self.package) == StylexSource::Root
                    && !self.root_member)
            && matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(loader) if unbound_reference(&self.bindings.scoping, loader))
        {
            self.reject(call.span.start, &readable_code(&call.callee), "bind the loader result to an unchanged namespace or simple named imports before calling an API");
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_ts_type(&mut self, _: &TSType<'a>) {}

    fn visit_identifier_reference(&mut self, identifier: &oxc_ast::ast::IdentifierReference<'a>) {
        if self.bindings.symbol(identifier).is_some_and(|symbol| {
            match self.bindings.stylex.binding(symbol) {
                Some(StylexBinding::Root) => !self.root_member,
                Some(
                    StylexBinding::Namespace
                    | StylexBinding::UpstreamNamespace
                    | StylexBinding::Function(_)
                    | StylexBinding::Invalid,
                ) => true,
                Some(StylexBinding::RootDefault) | None => false,
            }
        }) {
            self.reject(
                identifier.span.start,
                identifier.name.as_str(),
                "an API namespace/function cannot be exported or passed to runtime code",
            );
        }
    }

    fn visit_export_declaration(&mut self, export: &oxc_ast::ast::ExportDeclaration<'a>) {
        self.exported(export);
        walk::walk_export_declaration(self, export);
    }

    fn visit_export_named_declaration(
        &mut self,
        export: &oxc_ast::ast::ExportNamedDeclaration<'a>,
    ) {
        self.named_export(export);
    }

    fn visit_export_from_declaration(&mut self, export: &oxc_ast::ast::ExportFromDeclaration<'a>) {
        self.source_export(export);
    }

    fn visit_export_all_declaration(&mut self, export: &oxc_ast::ast::ExportAllDeclaration<'a>) {
        self.star_export(export);
    }

    fn visit_static_member_expression(
        &mut self,
        member: &oxc_ast::ast::StaticMemberExpression<'a>,
    ) {
        if matches!(
            self.bindings.stylex_binding(&member.object),
            Some(StylexBinding::Function(_))
        ) {
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

    fn visit_computed_member_expression(
        &mut self,
        member: &oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        if matches!(
            self.bindings.stylex_binding(&member.object),
            Some(StylexBinding::Function(_))
        ) {
            self.visit_expression(&member.expression);
            return;
        }
        let loader = self.root_loader(&member.object);
        if loader
            && crate::utils::get_string_by_literal_expression(&member.expression)
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

    fn visit_expression_statement(&mut self, statement: &oxc_ast::ast::ExpressionStatement<'a>) {
        let previous = self.root_member;
        self.root_member |= self.root_loader(&statement.expression);
        self.visit_expression(&statement.expression);
        self.root_member = previous;
    }

    fn visit_jsx_member_expression(&mut self, member: &oxc_ast::ast::JSXMemberExpression<'a>) {
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
