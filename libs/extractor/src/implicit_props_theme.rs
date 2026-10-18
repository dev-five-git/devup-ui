//! Error detection for alternate access to a callback's actual props argument.

use oxc_ast::ast::{
    ArrowFunctionBody, ArrowFunctionExpression, BindingPattern, BlockStatement, CatchClause, Class,
    ComputedMemberExpression, Expression, ForInStatement, ForOfStatement, ForStatement,
    ForStatementInit, ForStatementLeft, Function, FunctionBody, StaticBlock,
    StaticMemberExpression, SwitchStatement,
};
use oxc_ast_visit::{Visit, walk};
use oxc_syntax::scope::ScopeFlags;

use crate::utils::{get_string_by_literal_expression, unwrap_syntax_only};

#[path = "implicit_props_theme_scope.rs"]
mod scope;
use scope::Bindings;

#[cfg(test)]
#[path = "implicit_props_theme_tests.rs"]
mod tests;

/// These roots deliberately do not participate in static theme-path mapping.
pub(crate) fn reads(expr: &Expression<'_>) -> bool {
    let (params, arguments) = match unwrap_syntax_only(expr) {
        Expression::FunctionExpression(function) => (&function.params, true),
        Expression::ArrowFunctionExpression(arrow) => (&arrow.params, false),
        _ => return false,
    };
    let rest = if params.items.is_empty() {
        params
            .rest
            .as_ref()
            .and_then(|rest| match &rest.rest.argument {
                BindingPattern::BindingIdentifier(identifier) => Some(identifier.name.as_str()),
                _ => None,
            })
    } else {
        None
    };
    let mut reads = ThemeReads {
        roots: Roots { arguments, rest },
        found: false,
    };
    // The callback itself establishes roots; nested functions only inherit rest.
    let mut bindings = Bindings::new(reads.roots);
    bindings.visit_formal_parameters(params);
    reads.roots.arguments &= !bindings.arguments;
    reads.visit_formal_parameters(params);
    match unwrap_syntax_only(expr) {
        Expression::FunctionExpression(function) => {
            if let Some(body) = &function.body {
                reads.visit_root_body(body);
            }
        }
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => reads.visit_root_body(body),
            body => reads.visit_arrow_function_body(body),
        },
        _ => {}
    }
    reads.found
}

#[derive(Clone, Copy)]
struct Roots<'n> {
    arguments: bool,
    rest: Option<&'n str>,
}

impl Roots<'_> {
    fn is_props(self, expr: &Expression<'_>) -> bool {
        let Expression::ComputedMemberExpression(member) = unwrap_syntax_only(expr) else {
            return false;
        };
        if get_string_by_literal_expression(&member.expression).is_none_or(|key| key != "0") {
            return false;
        }
        matches!(unwrap_syntax_only(&member.object), Expression::Identifier(identifier)
            if (self.arguments && identifier.name == "arguments")
                || self.rest.is_some_and(|name| identifier.name == name))
    }

    fn shadowed(self, bindings: &Bindings<'_>) -> Self {
        Self {
            arguments: self.arguments && !bindings.arguments,
            rest: self.rest.filter(|_| !bindings.rest),
        }
    }
}

struct ThemeReads<'n> {
    roots: Roots<'n>,
    found: bool,
}

impl ThemeReads<'_> {
    fn visit_root_body(&mut self, body: &FunctionBody<'_>) {
        let saved = self.roots;
        // Root `var` redeclarations retain the rest parameter / own arguments root.
        // With non-simple parameters the body's var binding copies that value.
        // Function declarations can replace that value, so still mask them here.
        self.roots = saved.shadowed(&Bindings::lexical(saved, &body.statements));
        walk::walk_function_body(self, body);
        self.roots = saved;
    }
}

impl<'a> Visit<'a> for ThemeReads<'_> {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        self.found |= it.property.name == "theme" && self.roots.is_props(&it.object);
        walk::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(&mut self, it: &ComputedMemberExpression<'a>) {
        self.found |= get_string_by_literal_expression(&it.expression)
            .is_some_and(|key| key == "theme")
            && self.roots.is_props(&it.object);
        walk::walk_computed_member_expression(self, it);
    }

    fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
        let saved = self.roots;
        let mut bindings = Bindings::new(saved);
        bindings.visit_formal_parameters(&it.params);
        if let Some(id) = &it.id {
            bindings.visit_binding_identifier(id);
        }
        self.roots = saved.shadowed(&bindings);
        self.roots.arguments = false;
        walk::walk_function(self, it, flags);
        self.roots = saved;
    }

    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        let saved = self.roots;
        let mut bindings = Bindings::new(saved);
        bindings.visit_formal_parameters(&it.params);
        self.roots = saved.shadowed(&bindings);
        walk::walk_arrow_function_expression(self, it);
        self.roots = saved;
    }

    fn visit_function_body(&mut self, it: &FunctionBody<'a>) {
        let saved = self.roots;
        self.roots = saved
            .shadowed(&Bindings::function_vars(saved, &it.statements))
            .shadowed(&Bindings::lexical(saved, &it.statements));
        walk::walk_function_body(self, it);
        self.roots = saved;
    }

    fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
        let saved = self.roots;
        self.roots = saved.shadowed(&Bindings::lexical(saved, &it.body));
        walk::walk_block_statement(self, it);
        self.roots = saved;
    }
    fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
        let saved = self.roots;
        let mut bindings = Bindings::new(saved);
        if let Some(param) = &it.param {
            bindings.visit_binding_pattern(&param.pattern);
        }
        self.roots = saved.shadowed(&bindings);
        walk::walk_catch_clause(self, it);
        self.roots = saved;
    }

    fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
        let saved = self.roots;
        let mut bindings = Bindings::new(saved);
        if let Some(ForStatementInit::VariableDeclaration(declaration)) = &it.init {
            bindings.lexical_declaration(declaration);
        }
        self.roots = saved.shadowed(&bindings);
        walk::walk_for_statement(self, it);
        self.roots = saved;
    }

    fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
        let saved = self.roots;
        let mut bindings = Bindings::new(saved);
        if let ForStatementLeft::VariableDeclaration(declaration) = &it.left {
            bindings.lexical_declaration(declaration);
        }
        self.roots = saved.shadowed(&bindings);
        walk::walk_for_in_statement(self, it);
        self.roots = saved;
    }

    fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
        let saved = self.roots;
        let mut bindings = Bindings::new(saved);
        if let ForStatementLeft::VariableDeclaration(declaration) = &it.left {
            bindings.lexical_declaration(declaration);
        }
        self.roots = saved.shadowed(&bindings);
        walk::walk_for_of_statement(self, it);
        self.roots = saved;
    }

    fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
        self.visit_expression(&it.discriminant);
        let saved = self.roots;
        for case in &it.cases {
            self.roots = self
                .roots
                .shadowed(&Bindings::lexical(saved, &case.consequent));
        }
        for case in &it.cases {
            self.visit_switch_case(case);
        }
        self.roots = saved;
    }

    fn visit_class(&mut self, it: &Class<'a>) {
        let saved = self.roots;
        let mut bindings = Bindings::new(saved);
        if let Some(id) = &it.id {
            bindings.visit_binding_identifier(id);
        }
        self.roots = saved.shadowed(&bindings);
        walk::walk_class(self, it);
        self.roots = saved;
    }

    fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
        let saved = self.roots;
        self.roots = saved
            .shadowed(&Bindings::function_vars(saved, &it.body))
            .shadowed(&Bindings::lexical(saved, &it.body));
        walk::walk_static_block(self, it);
        self.roots = saved;
    }
}
