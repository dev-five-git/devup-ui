use oxc_ast::AstKind;
use oxc_ast::ast::{CallExpression, Expression, IdentifierReference, TemplateLiteral};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Semantic;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::super::{StyleSymbols, binding_of, reads_top_level};

mod callable;

pub(in crate::imported_constants) fn closed(
    expression: &Expression<'_>,
    style: &StyleSymbols<'_>,
    known: &dyn Fn(&IdentifierReference<'_>) -> bool,
) -> bool {
    if !matches!(
        expression,
        Expression::Identifier(_)
            | Expression::StaticMemberExpression(_)
            | Expression::ComputedMemberExpression(_)
            | Expression::CallExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::TemplateLiteral(_)
            | Expression::ConditionalExpression(_)
            | Expression::LogicalExpression(_)
    ) {
        return false;
    }
    let mut proof = Closed {
        style,
        known,
        exact: true,
        input: false,
    };
    proof.visit_expression(expression);
    proof.exact && proof.input
}

pub(in crate::imported_constants) fn member(
    input: (&Expression<'_>, Option<&Expression<'_>>),
    style: &StyleSymbols<'_>,
    known: &dyn Fn(&IdentifierReference<'_>) -> bool,
) -> bool {
    let mut proof = Closed {
        style,
        known,
        exact: true,
        input: false,
    };
    proof.visit_expression(input.0);
    if let Some(key) = input.1 {
        proof.visit_expression(key);
    }
    proof.exact && proof.input
}

pub(in crate::imported_constants) fn template(
    template: &TemplateLiteral<'_>,
    style: &StyleSymbols<'_>,
    known: &dyn Fn(&IdentifierReference<'_>) -> bool,
) -> bool {
    let mut proof = Closed {
        style,
        known,
        exact: true,
        input: false,
    };
    proof.visit_template_literal(template);
    proof.exact && proof.input
}

pub(in crate::imported_constants) fn call(
    call: &CallExpression<'_>,
    style: &StyleSymbols<'_>,
    known: &dyn Fn(&IdentifierReference<'_>) -> bool,
) -> bool {
    let mut proof = Closed {
        style,
        known,
        exact: true,
        input: false,
    };
    proof.visit_call_expression(call);
    proof.exact && proof.input
}

pub(super) struct Closed<'s> {
    pub style: &'s StyleSymbols<'s>,
    pub known: &'s dyn Fn(&IdentifierReference<'_>) -> bool,
    pub exact: bool,
    pub input: bool,
}

impl<'a> Visit<'a> for Closed<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        match binding_of(self.style.scoping, identifier) {
            Some(_) => {
                self.exact &= reads_top_level(self.style.scoping, identifier)
                    && !self.style.has(identifier)
                    && (self.known)(identifier);
                self.input = true;
            }
            None => {
                self.exact &= matches!(
                    identifier.name.as_str(),
                    "undefined"
                        | "NaN"
                        | "Infinity"
                        | "Math"
                        | "String"
                        | "Number"
                        | "Boolean"
                        | "Array"
                        | "Object"
                        | "JSON"
                        | "parseInt"
                        | "parseFloat"
                        | "isNaN"
                        | "isFinite"
                        | "encodeURIComponent"
                        | "decodeURIComponent"
                        | "encodeURI"
                        | "decodeURI"
                );
            }
        }
    }

    fn visit_expression(&mut self, expression: &Expression<'a>) {
        if matches!(
            expression,
            Expression::ArrowFunctionExpression(_)
                | Expression::FunctionExpression(_)
                | Expression::JSXElement(_)
                | Expression::JSXFragment(_)
                | Expression::AssignmentExpression(_)
                | Expression::UpdateExpression(_)
                | Expression::AwaitExpression(_)
                | Expression::YieldExpression(_)
        ) {
            self.exact = false;
            return;
        }
        walk::walk_expression(self, expression);
    }
}

pub(super) fn input(
    identifier: &IdentifierReference<'_>,
    semantic: &Semantic<'_>,
    style: &StyleSymbols<'_>,
) -> bool {
    binding_of(semantic.scoping(), identifier)
        .is_some_and(|symbol| binding(symbol, (semantic, style), &FxHashSet::default()))
}

fn binding(
    symbol: SymbolId,
    context: (&Semantic<'_>, &StyleSymbols<'_>),
    seen: &FxHashSet<SymbolId>,
) -> bool {
    let (semantic, style) = context;
    let scoping = semantic.scoping();
    if scoping.symbol_scope_id(symbol) != scoping.root_scope_id() || style.roots.contains(&symbol) {
        return false;
    }
    if scoping.symbol_flags(symbol).is_import() {
        return true;
    }
    let declaration = semantic.nodes().kind(scoping.symbol_declaration(symbol));
    if seen.contains(&symbol) {
        return matches!(declaration, AstKind::Function(_))
            || matches!(declaration, AstKind::VariableDeclarator(declarator)
                if matches!(declarator.init.as_ref(), Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_))));
    }
    let mut seen = seen.clone();
    seen.insert(symbol);
    let known = |identifier: &IdentifierReference<'_>| {
        binding_of(scoping, identifier).is_some_and(|symbol| binding(symbol, context, &seen))
    };
    let mut proof = Closed {
        style,
        known: &known,
        exact: true,
        input: false,
    };
    match declaration {
        AstKind::Function(_) => callable::closed(declaration, &mut proof, semantic),
        AstKind::VariableDeclarator(declarator) => match declarator.init.as_ref() {
            Some(Expression::ArrowFunctionExpression(function)) => callable::closed(
                AstKind::ArrowFunctionExpression(function),
                &mut proof,
                semantic,
            ),
            Some(Expression::FunctionExpression(function)) => {
                callable::closed(AstKind::Function(function), &mut proof, semantic)
            }
            Some(initializer) => {
                proof.visit_expression(initializer);
                proof.exact
            }
            None => false,
        },
        _ => false,
    }
}
