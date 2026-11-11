//! A function the build writes only to pass values in, called where a
//! statement evaluates first, gives what declaring those values right before
//! the statement gives: they are evaluated first either way, in the same order.

use oxc_allocator::{CloneIn, GetAllocator, TakeIn, Vec};
use oxc_ast::{
    ast::{
        Argument, ArrowFunctionBody, ArrowFunctionExpression, BindingIdentifier, BindingPattern,
        CallExpression, CatchClause, Expression, FormalParameters, Function, FunctionBody,
        IdentifierReference, Program, Statement, VariableDeclarationKind, VariableDeclarator,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_span::SPAN;
use oxc_syntax::scope::ScopeFlags;
use rustc_hash::{FxHashMap, FxHashSet};

/// Unwrap the generated functions of `program` that only pass values in
pub(crate) fn unwrap<'a>(ast: &AstBuilder<'a>, program: &mut Program<'a>) {
    let mut eval = DirectEval(false);
    eval.visit_program(program);
    // Code given to direct eval could read a name declared here.
    if !eval.0 {
        Unwrap {
            ast,
            parameters: std::vec::Vec::new(),
        }
        .visit_program(program);
    }
}

struct DirectEval(bool);

impl<'a> Visit<'a> for DirectEval {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        self.0 |= matches!(it.callee.get_inner_expression(), Expression::Identifier(id) if id.name == "eval");
        walk::walk_call_expression(self, it);
    }
}

/// How often each name is bound or read
#[derive(Default)]
struct Names(FxHashMap<String, usize>);

impl<'a> Visit<'a> for Names {
    fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
        *self.0.entry(it.name.to_string()).or_default() += 1;
    }
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        *self.0.entry(it.name.to_string()).or_default() += 1;
    }
}

fn names_of(parameters: &FormalParameters<'_>) -> FxHashSet<String> {
    let mut names = Names::default();
    names.visit_formal_parameters(parameters);
    names.0.into_keys().collect()
}

struct Unwrap<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    /// The names a function body cannot declare again: its parameters
    parameters: std::vec::Vec<FxHashSet<String>>,
}

impl<'a> VisitMut<'a> for Unwrap<'_, 'a> {
    fn visit_program(&mut self, it: &mut Program<'a>) {
        walk_mut::walk_program(self, it);
        statements(self.ast, &mut it.body, &FxHashSet::default());
    }

    fn visit_function(&mut self, it: &mut Function<'a>, flags: ScopeFlags) {
        self.parameters.push(names_of(&it.params));
        walk_mut::walk_function(self, it, flags);
        self.parameters.pop();
    }

    fn visit_arrow_function_expression(&mut self, it: &mut ArrowFunctionExpression<'a>) {
        let parameters = names_of(&it.params);
        self.parameters.push(parameters.clone());
        walk_mut::walk_arrow_function_expression(self, it);
        self.parameters.pop();
        let Some(value) = it.body.as_expression_mut() else {
            return;
        };
        if candidate(value, false).is_none() {
            return;
        }
        let value = value.take_in(self.ast);
        let mut body = Vec::from_array_in(
            [Statement::new_return_statement(SPAN, Some(value), self.ast)],
            self.ast,
        );
        statements(self.ast, &mut body, &parameters);
        it.body = ArrowFunctionBody::new_function_body(SPAN, Vec::new_in(self.ast), body, self.ast);
    }

    fn visit_function_body(&mut self, it: &mut FunctionBody<'a>) {
        walk_mut::walk_function_body(self, it);
        let parameters = self.parameters.last().cloned().unwrap_or_default();
        statements(self.ast, &mut it.statements, &parameters);
    }

    fn visit_block_statement(&mut self, it: &mut oxc_ast::ast::BlockStatement<'a>) {
        walk_mut::walk_block_statement(self, it);
        statements(self.ast, &mut it.body, &FxHashSet::default());
    }

    fn visit_catch_clause(&mut self, it: &mut CatchClause<'a>) {
        let mut names = Names::default();
        if let Some(parameter) = &it.param {
            names.visit_catch_parameter(parameter);
        }
        walk_mut::walk_block_statement(self, &mut it.body);
        statements(self.ast, &mut it.body.body, &names.0.into_keys().collect());
    }

    fn visit_static_block(&mut self, it: &mut oxc_ast::ast::StaticBlock<'a>) {
        walk_mut::walk_static_block(self, it);
        statements(self.ast, &mut it.body, &FxHashSet::default());
    }
}

fn parenthesized<'s, 'a>(mut value: &'s Expression<'a>) -> &'s Expression<'a> {
    while let Expression::ParenthesizedExpression(inner) = value {
        value = &inner.expression;
    }
    value
}

fn parenthesized_mut<'s, 'a>(mut value: &'s mut Expression<'a>) -> &'s mut Expression<'a> {
    while let Expression::ParenthesizedExpression(inner) = value {
        value = &mut inner.expression;
    }
    value
}

/// The value a statement evaluates first and whole, and whether it is a
/// statement of its own
fn head<'s, 'a>(statement: &'s mut Statement<'a>) -> Option<(&'s mut Expression<'a>, bool)> {
    match statement {
        Statement::ExpressionStatement(statement) => Some((&mut statement.expression, true)),
        Statement::ReturnStatement(statement) => {
            statement.argument.as_mut().map(|value| (value, false))
        }
        Statement::VariableDeclaration(declaration) if declaration.declarations.len() == 1 => {
            declaration.declarations[0]
                .init
                .as_mut()
                .map(|value| (value, false))
        }
        _ => None,
    }
}

/// The names a generated call declares once unwrapped, when it can be
fn candidate(value: &Expression<'_>, alone: bool) -> Option<std::vec::Vec<String>> {
    let Expression::CallExpression(call) = parenthesized(value) else {
        return None;
    };
    let Expression::ArrowFunctionExpression(arrow) = parenthesized(&call.callee) else {
        return None;
    };
    if call.optional
        || arrow.span != SPAN
        || arrow.r#async
        || arrow.params.rest.is_some()
        || arrow.params.items.len() != call.arguments.len()
        || call.arguments.iter().any(Argument::is_spread)
    {
        return None;
    }
    let mut names = std::vec::Vec::new();
    for parameter in &arrow.params.items {
        let BindingPattern::BindingIdentifier(id) = &parameter.pattern else {
            return None;
        };
        if parameter.initializer.is_some() {
            return None;
        }
        names.push(id.name.to_string());
    }
    let returned = match &arrow.body {
        ArrowFunctionBody::FunctionBody(body) => {
            let (Statement::ReturnStatement(returned), declarations) =
                body.statements.split_last()?
            else {
                return None;
            };
            if !body.directives.is_empty() {
                return None;
            }
            for statement in declarations {
                let Statement::VariableDeclaration(declaration) = statement else {
                    return None;
                };
                if declaration.kind != VariableDeclarationKind::Const {
                    return None;
                }
                for declarator in &declaration.declarations {
                    let BindingPattern::BindingIdentifier(id) = &declarator.id else {
                        return None;
                    };
                    names.push(id.name.to_string());
                }
            }
            returned.argument.as_ref()?
        }
        body => body.as_expression()?,
    };
    // A string alone in a statement could read as a directive.
    if alone && matches!(parenthesized(returned), Expression::StringLiteral(_)) {
        return None;
    }
    Some(names)
}

fn statements<'a>(
    ast: &AstBuilder<'a>,
    statements: &mut Vec<'a, Statement<'a>>,
    parameters: &FxHashSet<String>,
) {
    let mut index = 0;
    while index < statements.len() {
        let Some((names, inside)) = head(&mut statements[index]).and_then(|(value, alone)| {
            let names = candidate(value, alone)?;
            let mut inside = Names::default();
            if let Expression::CallExpression(call) = parenthesized(value) {
                inside.visit_expression(&call.callee);
            }
            Some((names, inside))
        }) else {
            index += 1;
            continue;
        };
        let mut everywhere = Names::default();
        for statement in statements.iter() {
            everywhere.visit_statement(statement);
        }
        // Each name declared must be read nowhere else here, nor be a parameter.
        let free = names
            .iter()
            .all(|name| !parameters.contains(name) && everywhere.0.get(name) == inside.0.get(name));
        let declaration = matches!(&statements[index], Statement::VariableDeclaration(_));
        let Some(inserted) = head(&mut statements[index])
            .filter(|_| free)
            .and_then(|(value, _)| unwrap_call(ast, value, declaration))
        else {
            index += 1;
            continue;
        };
        let count = inserted.len();
        for (offset, statement) in inserted.into_iter().enumerate() {
            statements.insert(index + offset, statement);
        }
        // What the call gave may be another call to unwrap.
        index += count;
    }
}

/// Replace the generated call `value` by what it returns, giving the
/// statements that declare what it read before
fn unwrap_call<'a>(
    ast: &AstBuilder<'a>,
    value: &mut Expression<'a>,
    declaration: bool,
) -> Option<Vec<'a, Statement<'a>>> {
    let Expression::CallExpression(call) = parenthesized_mut(value) else {
        return None;
    };
    let arguments = call.arguments.take_in(ast);
    let Expression::ArrowFunctionExpression(arrow) = parenthesized_mut(&mut call.callee) else {
        return None;
    };
    let mut declarators = Vec::new_in(ast);
    for (parameter, argument) in arrow.params.items.iter().zip(arguments) {
        declarators.push(VariableDeclarator::new(
            SPAN,
            parameter.pattern.clone_in(ast.allocator()),
            None,
            Some(unbound(ast, argument.into_expression())),
            false,
            ast,
        ));
    }
    let (mut body, returned) = match arrow.body.take_in(ast) {
        ArrowFunctionBody::FunctionBody(mut body) => match body.statements.pop() {
            Some(Statement::ReturnStatement(mut returned)) => {
                (body.statements.take_in(ast), returned.argument.take()?)
            }
            _ => return None,
        },
        body => (Vec::new_in(ast), body.into_expression()),
    };
    *value = if declaration {
        unbound(ast, returned)
    } else {
        returned
    };
    let mut statements = Vec::new_in(ast);
    if !declarators.is_empty() {
        statements.push(Statement::new_variable_declaration(
            SPAN,
            VariableDeclarationKind::Const,
            declarators,
            false,
            ast,
        ));
    }
    statements.append(&mut body);
    Some(statements)
}

/// `value`, without taking the name of the binding it initializes
fn unbound<'a>(ast: &AstBuilder<'a>, value: Expression<'a>) -> Expression<'a> {
    if matches!(
        parenthesized(&value),
        Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ClassExpression(_)
    ) {
        crate::utils::unbound_value(ast, value)
    } else {
        value
    }
}

#[cfg(test)]
#[path = "unwrap_iife_tests.rs"]
mod tests;
