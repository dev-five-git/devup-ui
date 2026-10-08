use oxc_ast::ast::{Expression, ImportDeclaration, Program, TSType, VariableDeclarator};
use oxc_ast_visit::{Visit, walk};
use oxc_span::GetSpan;

use crate::scope::Bindings;
use crate::scope::stylex_bindings::{StylexBinding, unbound_reference};
use crate::scope::stylex_sources::{StylexSource, require_source};
use crate::utils::{readable_code, unwrap_syntax_only};

mod declarations;
mod diagnostics;
mod exports;
mod loaders;
mod members;

pub(super) struct Check<'s> {
    bindings: &'s Bindings,
    package: &'s str,
    diagnosed: &'s [(u32, String)],
    errors: Vec<(u32, String)>,
    root_member: bool,
    form: &'static str,
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
            form: "function value escape",
        }
    }

    pub(super) fn errors(mut self, program: &Program<'_>) -> Vec<(u32, String)> {
        self.visit_program(program);
        self.errors
    }
}

impl<'a> Visit<'a> for Check<'_> {
    fn visit_import_declaration(&mut self, import: &ImportDeclaration<'a>) {
        self.imported(import);
    }

    fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
        if self.required(declarator) {
            self.visit_binding_pattern(&declarator.id);
            if let Some(Expression::CallExpression(call)) =
                declarator.init.as_ref().map(unwrap_syntax_only)
            {
                self.visit_arguments(&call.arguments);
            }
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
            Some(StylexBinding::Function(function)) => {
                self.reject_function(
                    &function,
                    (expression.span().start, &readable_code(expression)),
                    self.form,
                );
            }
            Some(
                StylexBinding::Namespace
                | StylexBinding::UpstreamNamespace
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
        self.runtime_loader(call);
        walk::walk_call_expression(self, call);
    }

    fn visit_ts_type(&mut self, _: &TSType<'a>) {}

    fn visit_identifier_reference(&mut self, identifier: &oxc_ast::ast::IdentifierReference<'a>) {
        if let Some(symbol) = self.bindings.symbol(identifier)
            && let Some(StylexBinding::Function(function)) = self.bindings.stylex.binding(symbol)
        {
            self.reject_function(
                function,
                (identifier.span.start, identifier.name.as_str()),
                self.form,
            );
            return;
        }
        if self.bindings.symbol(identifier).is_some_and(|symbol| {
            match self.bindings.stylex.binding(symbol) {
                Some(StylexBinding::Root) => !self.root_member,
                Some(
                    StylexBinding::Namespace
                    | StylexBinding::UpstreamNamespace
                    | StylexBinding::Invalid,
                ) => true,
                Some(StylexBinding::Function(_) | StylexBinding::RootDefault) | None => false,
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
        self.static_member(member);
    }

    fn visit_computed_member_expression(
        &mut self,
        member: &oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        self.computed_member(member);
    }

    fn visit_expression_statement(&mut self, statement: &oxc_ast::ast::ExpressionStatement<'a>) {
        let previous = self.root_member;
        self.root_member |= self.root_loader(&statement.expression);
        self.visit_expression(&statement.expression);
        self.root_member = previous;
    }

    fn visit_jsx_member_expression(&mut self, member: &oxc_ast::ast::JSXMemberExpression<'a>) {
        self.jsx_member(member);
    }

    fn visit_arguments(&mut self, arguments: &oxc_allocator::Vec<'a, oxc_ast::ast::Argument<'a>>) {
        let previous = self.form;
        self.form = "argument passing";
        walk::walk_arguments(self, arguments);
        self.form = previous;
    }

    fn visit_object_property(&mut self, property: &oxc_ast::ast::ObjectProperty<'a>) {
        let previous = self.form;
        self.form = "object storage";
        walk::walk_object_property(self, property);
        self.form = previous;
    }

    fn visit_array_expression(&mut self, array: &oxc_ast::ast::ArrayExpression<'a>) {
        let previous = self.form;
        self.form = "array storage";
        walk::walk_array_expression(self, array);
        self.form = previous;
    }

    fn visit_import_expression(&mut self, import: &oxc_ast::ast::ImportExpression<'a>) {
        self.dynamic_loader(import);
        walk::walk_import_expression(self, import);
    }
}
