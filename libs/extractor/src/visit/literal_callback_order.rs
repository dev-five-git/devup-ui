use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{
    ArrowFunctionBody, ArrowFunctionExpression, Expression, Function, ObjectPropertyKind,
    PropertyKey, PropertyKind, ReturnStatement, Statement, StringLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::{GetSpan, SPAN, Span};

pub(super) enum WrappedCallback<'a> {
    Arrow(oxc_allocator::Box<'a, ArrowFunctionExpression<'a>>),
    Function(oxc_allocator::Box<'a, Function<'a>>),
}

pub(super) struct OrderEnvelope<'a> {
    span: Span,
    pub(super) candidate: Expression<'a>,
}

impl<'a> OrderEnvelope<'a> {
    pub(super) fn new(candidate: Expression<'a>) -> Self {
        Self {
            span: candidate.span(),
            candidate,
        }
    }

    pub(super) fn expression(&self, ast: &AstBuilder<'a>) -> Expression<'a> {
        Expression::new_object_expression(
            self.span,
            oxc_allocator::Vec::from_array_in(
                [ObjectPropertyKind::new_object_property(
                    self.span,
                    PropertyKind::Init,
                    PropertyKey::StringLiteral(StringLiteral::boxed(
                        self.span,
                        "styleOrder",
                        None,
                        ast,
                    )),
                    self.candidate.clone_in_with_semantic_ids(ast.allocator()),
                    false,
                    false,
                    false,
                    ast,
                )],
                ast,
            ),
            ast,
        )
    }
}

pub(super) enum OrderStep<'a> {
    Value(Option<Expression<'a>>),
    Guarded {
        test: Expression<'a>,
        yes: Option<Expression<'a>>,
        no: Option<Expression<'a>>,
    },
}

impl<'a> WrappedCallback<'a> {
    pub(super) const fn into_expression(self) -> Expression<'a> {
        match self {
            Self::Arrow(arrow) => Expression::ArrowFunctionExpression(arrow),
            Self::Function(function) => Expression::FunctionExpression(function),
        }
    }
}

struct Returns<'s, 'a> {
    ast: &'s AstBuilder<'a>,
}

impl<'a> VisitMut<'a> for Returns<'_, 'a> {
    fn visit_return_statement(&mut self, statement: &mut ReturnStatement<'a>) {
        if let Some(value) = &mut statement.argument {
            *value = order_object(
                self.ast,
                value.clone_in_with_semantic_ids(self.ast.allocator()),
            )
            .expression(self.ast);
        }
    }
    fn visit_function(
        &mut self,
        _: &mut oxc_ast::ast::Function<'a>,
        _: oxc_syntax::scope::ScopeFlags,
    ) {
    }
    fn visit_arrow_function_expression(
        &mut self,
        _: &mut oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
    }
}

pub(super) fn wrapped<'a>(
    ast: &AstBuilder<'a>,
    function: &mut Expression<'a>,
) -> Option<WrappedCallback<'a>> {
    let (original, body) = match function {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            let original =
                WrappedCallback::Arrow(arrow.clone_in_with_semantic_ids(ast.allocator()));
            match &mut arrow.body {
                ArrowFunctionBody::FunctionBody(body) => (original, body),
                body => {
                    let value = body.to_expression_mut();
                    *value = order_object(ast, value.clone_in_with_semantic_ids(ast.allocator()))
                        .expression(ast);
                    return Some(original);
                }
            }
        }
        Expression::FunctionExpression(function) => {
            let original =
                WrappedCallback::Function(function.clone_in_with_semantic_ids(ast.allocator()));
            match &mut **function {
                Function {
                    body: Some(body),
                    r#async: false,
                    generator: false,
                    ..
                } => (original, body),
                _ => return None,
            }
        }
        _ => return None,
    };
    if !body.statements.last().is_some_and(terminal) && !body.statements.iter().all(normal_exit) {
        return None;
    }
    walk_mut::walk_function_body(&mut Returns { ast }, body);
    Some(original)
}

fn terminal(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::ReturnStatement(statement) => statement.argument.is_some(),
        Statement::BlockStatement(block) => block.body.last().is_some_and(terminal),
        Statement::IfStatement(statement) => {
            terminal(&statement.consequent) && statement.alternate.as_ref().is_some_and(terminal)
        }
        _ => false,
    }
}

/// Recognize the supported normal-exit skeleton without inspecting opaque operations.
fn normal_exit(statement: &Statement<'_>) -> bool {
    match statement {
        Statement::EmptyStatement(_)
        | Statement::ExpressionStatement(_)
        | Statement::VariableDeclaration(_)
        | Statement::FunctionDeclaration(_)
        | Statement::ReturnStatement(_) => true,
        Statement::BlockStatement(block) => block.body.iter().all(normal_exit),
        Statement::IfStatement(statement) => {
            normal_exit(&statement.consequent)
                && statement.alternate.as_ref().is_none_or(normal_exit)
        }
        _ => false,
    }
}

fn order_object<'a>(_: &AstBuilder<'a>, candidate: Expression<'a>) -> OrderEnvelope<'a> {
    OrderEnvelope::new(candidate)
}

pub(super) fn value<'a>(ast: &AstBuilder<'a>, parts: &[OrderStep<'a>]) -> Option<Expression<'a>> {
    let mut selected = None;
    for part in parts {
        match part {
            OrderStep::Value(candidate) => {
                selected = candidate
                    .clone_in_with_semantic_ids(ast.allocator())
                    .or(selected);
            }
            OrderStep::Guarded { test, yes, no } => {
                let yes = yes.as_ref()?.clone_in_with_semantic_ids(ast.allocator());
                let no = no
                    .clone_in_with_semantic_ids(ast.allocator())
                    .or_else(|| selected.take());
                let test = test.clone_in_with_semantic_ids(ast.allocator());
                selected = Some(match no {
                    Some(no) => Expression::new_conditional_expression(SPAN, test, yes, no, ast),
                    None => Expression::new_logical_expression(
                        SPAN,
                        test,
                        oxc_ast::ast::LogicalOperator::And,
                        yes,
                        ast,
                    ),
                });
            }
        }
    }
    selected
}
