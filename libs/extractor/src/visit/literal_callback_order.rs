use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{
    ArrowFunctionBody, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, ReturnStatement,
    Statement, StringLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::{GetSpan, SPAN, Span};

pub(super) struct WrappedCallback<'a> {
    pub(super) original: Expression<'a>,
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

pub(super) fn wrapped<'a>(
    ast: &AstBuilder<'a>,
    function: &mut Expression<'a>,
) -> Option<WrappedCallback<'a>> {
    let original = function.clone_in_with_semantic_ids(ast.allocator());
    wrap(ast, function).then_some(WrappedCallback { original })
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

pub(super) fn wrap<'a>(ast: &AstBuilder<'a>, function: &mut Expression<'a>) -> bool {
    let body = match function {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => match &mut arrow.body {
            ArrowFunctionBody::FunctionBody(body) => body,
            body => {
                let value = body.to_expression_mut();
                *value = order_object(ast, value.clone_in_with_semantic_ids(ast.allocator()))
                    .expression(ast);
                return true;
            }
        },
        Expression::FunctionExpression(function) => match &mut **function {
            oxc_ast::ast::Function {
                body: Some(body),
                r#async: false,
                generator: false,
                ..
            } => body,
            _ => return false,
        },
        _ => return false,
    };
    if !body.statements.last().is_some_and(terminal) {
        return false;
    }
    walk_mut::walk_function_body(&mut Returns { ast }, body);
    true
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
