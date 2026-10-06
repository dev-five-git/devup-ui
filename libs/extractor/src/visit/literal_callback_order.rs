use crate::composition::{KnownPart, KnownStyles};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{
    ArrowFunctionBody, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, ReturnStatement,
    Statement, StringLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::{GetSpan, SPAN};

struct Returns<'s, 'a> {
    ast: &'s AstBuilder<'a>,
}

impl<'a> VisitMut<'a> for Returns<'_, 'a> {
    fn visit_return_statement(&mut self, statement: &mut ReturnStatement<'a>) {
        if let Some(value) = &mut statement.argument {
            *value = order_object(
                self.ast,
                value.clone_in_with_semantic_ids(self.ast.allocator()),
            );
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
                let Some(value) = body.as_expression_mut() else {
                    return false;
                };
                *value = order_object(ast, value.clone_in_with_semantic_ids(ast.allocator()));
                return true;
            }
        },
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => {
            let Some(body) = &mut function.body else {
                return false;
            };
            body
        }
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

fn order_object<'a>(ast: &AstBuilder<'a>, value: Expression<'a>) -> Expression<'a> {
    Expression::new_object_expression(
        value.span(),
        oxc_allocator::Vec::from_array_in(
            [ObjectPropertyKind::new_object_property(
                value.span(),
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    value.span(),
                    "styleOrder",
                    None,
                    ast,
                )),
                value,
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

fn styles_order<'a>(ast: &AstBuilder<'a>, styles: &[KnownStyles<'a>]) -> Option<Expression<'a>> {
    styles.iter().find_map(|style| {
        let KnownStyles::Rules(Expression::ObjectExpression(object)) = style else {
            return None;
        };
        object.properties.iter().find_map(|property| {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return None;
            };
            property
                .key
                .static_name()
                .is_some_and(|key| crate::style_order::reserved(&key))
                .then(|| property.value.clone_in_with_semantic_ids(ast.allocator()))
        })
    })
}

pub(super) fn value<'a>(ast: &AstBuilder<'a>, parts: &[KnownPart<'a>]) -> Option<Expression<'a>> {
    let mut selected = None;
    for part in parts {
        match part {
            KnownPart::Styles(styles) => selected = styles_order(ast, styles).or(selected),
            KnownPart::Conditional {
                test,
                consequent,
                alternate,
            } => {
                let yes = styles_order(ast, consequent)?;
                let no = styles_order(ast, alternate)
                    .or_else(|| selected.take())
                    .unwrap_or_else(|| yes.clone_in_with_semantic_ids(ast.allocator()));
                selected = Some(Expression::new_conditional_expression(
                    SPAN,
                    test.clone_in_with_semantic_ids(ast.allocator()),
                    yes,
                    no,
                    ast,
                ));
            }
            KnownPart::Class(_) => return None,
        }
    }
    selected
}
