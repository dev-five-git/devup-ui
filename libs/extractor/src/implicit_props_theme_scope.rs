//! Binding ownership for the implicit callback-props detector.

use oxc_ast::ast::{
    ArrowFunctionExpression, BindingIdentifier, Class, Expression, Function, Statement,
    VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::Visit;
use oxc_syntax::scope::ScopeFlags;

use super::Roots;

/// Only binding names are collected; pattern defaults never introduce bindings here.
pub(super) struct Bindings<'n> {
    roots: Roots<'n>,
    pub(super) arguments: bool,
    pub(super) rest: bool,
}

impl<'n> Bindings<'n> {
    pub(super) const fn new(roots: Roots<'n>) -> Self {
        Self {
            roots,
            arguments: false,
            rest: false,
        }
    }

    /// Direct declarations belong to this block, not to its catch/loop/switch children.
    pub(super) fn lexical<'a>(roots: Roots<'n>, statements: &[Statement<'a>]) -> Self {
        let mut bindings = Self::new(roots);
        for statement in statements {
            match statement {
                Statement::VariableDeclaration(declaration) => {
                    bindings.lexical_declaration(declaration);
                }
                Statement::FunctionDeclaration(function) => {
                    if let Some(id) = &function.id {
                        bindings.visit_binding_identifier(id);
                    }
                }
                Statement::ClassDeclaration(class) => {
                    if let Some(id) = &class.id {
                        bindings.visit_binding_identifier(id);
                    }
                }
                _ => {}
            }
        }
        bindings
    }

    pub(super) fn lexical_declaration(&mut self, declaration: &VariableDeclaration<'_>) {
        if declaration.kind != VariableDeclarationKind::Var {
            for declarator in &declaration.declarations {
                self.visit_binding_pattern(&declarator.id);
            }
        }
    }

    /// Var declarations cross statement scopes, but never function or class boundaries.
    pub(super) fn function_vars<'a>(roots: Roots<'n>, statements: &[Statement<'a>]) -> Self {
        let mut vars = FunctionVars(Self::new(roots));
        for statement in statements {
            vars.visit_statement(statement);
        }
        vars.0
    }
}

impl<'a> Visit<'a> for Bindings<'_> {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        self.arguments |= it.name == "arguments";
        self.rest |= self.roots.rest.is_some_and(|name| it.name == name);
    }

    fn visit_expression(&mut self, _: &Expression<'a>) {}
}

struct FunctionVars<'n>(Bindings<'n>);

impl<'a> Visit<'a> for FunctionVars<'_> {
    fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Var {
            for declarator in &it.declarations {
                self.0.visit_binding_pattern(&declarator.id);
            }
        }
    }

    fn visit_expression(&mut self, _: &Expression<'a>) {}
    fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}
    fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}
    fn visit_class(&mut self, _: &Class<'a>) {}
}
