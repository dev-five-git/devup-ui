use super::branch_capture::BranchReads;
use super::capture::Captured;
use super::literal_callback_order::OrderStep;
use super::styled_callbacks::OrderBody;
use super::{DevupVisitor, Text};
use crate::composition::KnownPart;
use oxc_allocator::{GetAllocator, TakeIn};
use oxc_ast::ast::{ArrowFunctionBody, Expression, FunctionBody, ReturnStatement};
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::SPAN;
use rustc_hash::FxHashMap;

struct Returns<'v, 'a> {
    visitor: &'v mut DevupVisitor<'a>,
    render: String,
    parts: Vec<KnownPart<'a>>,
    orders: Option<&'v mut Vec<OrderStep<'a>>>,
    count: usize,
    complete: bool,
}

enum ReturnOrder<'r, 'a> {
    Styles,
    Order {
        body: OrderBody<'a>,
        values: Vec<Captured<'a>>,
        orders: &'r mut Vec<OrderStep<'a>>,
    },
}

impl<'a> VisitMut<'a> for Returns<'_, 'a> {
    fn visit_return_statement(&mut self, statement: &mut ReturnStatement<'a>) {
        let Some(body) = &mut statement.argument else {
            statement.argument = Some(Expression::new_array_expression(
                SPAN,
                oxc_allocator::Vec::new_in(&self.visitor.ast),
                &self.visitor.ast,
            ));
            return;
        };
        let mut prepared = match &mut self.orders {
            Some(orders) => {
                let (body, values) = self
                    .visitor
                    .prepare_order_body(body.take_in(&self.visitor.ast));
                ReturnOrder::Order {
                    body,
                    values,
                    orders,
                }
            }
            None => ReturnOrder::Styles,
        };
        match &prepared {
            ReturnOrder::Order { .. } => {}
            ReturnOrder::Styles => {
                crate::css_utils::literal_tree::lower(
                    &self.visitor.ast,
                    body,
                    crate::css_utils::literal_tree::Scope {
                        source: self.visitor.source,
                        global: false,
                    },
                );
                if self
                    .visitor
                    .known_parts(body, &mut Vec::new(), Text::Classes)
                    .is_none()
                {
                    self.complete = false;
                    return;
                }
                self.visitor.check_style_orders(body, false);
            }
        }
        let values = match &mut prepared {
            ReturnOrder::Order { values, .. } => std::mem::take(values),
            ReturnOrder::Styles => {
                let mut values = Vec::new();
                self.visitor.capture_shape(body, &mut values);
                values
            }
        };
        let mut reads = BranchReads {
            ast: &self.visitor.ast,
            reads: FxHashMap::default(),
        };
        let mut elements = oxc_allocator::Vec::new_in(&self.visitor.ast);
        elements.push(
            Expression::new_string_literal(
                SPAN,
                self.visitor
                    .ast
                    .allocator()
                    .alloc_str(&self.count.to_string()),
                None,
                &self.visitor.ast,
            )
            .into(),
        );
        for (index, (name, value)) in values.into_iter().enumerate() {
            reads
                .reads
                .insert(name, saved_slot(&self.visitor.ast, &self.render, index + 1));
            elements.push(value.into());
        }
        let parts = match &mut prepared {
            ReturnOrder::Order { body, .. } => {
                body.read(&mut reads);
                Vec::new()
            }
            ReturnOrder::Styles => {
                reads.visit_expression(body);
                let mut parts = Vec::new();
                if self
                    .visitor
                    .known_parts(body, &mut parts, Text::Classes)
                    .is_none()
                {
                    self.complete = false;
                    return;
                }
                parts
            }
        };
        let test = Expression::new_binary_expression(
            SPAN,
            saved_slot(&self.visitor.ast, &self.render, 0),
            oxc_ast::ast::BinaryOperator::StrictEquality,
            Expression::new_string_literal(
                SPAN,
                self.visitor
                    .ast
                    .allocator()
                    .alloc_str(&self.count.to_string()),
                None,
                &self.visitor.ast,
            ),
            &self.visitor.ast,
        );
        match prepared {
            ReturnOrder::Order { body, orders, .. } => orders.push(OrderStep::Guarded {
                test,
                yes: body.candidate(self.visitor),
                no: None,
            }),
            ReturnOrder::Styles => self.parts.extend(self.visitor.guarded_parts(&test, parts)),
        }
        *body = Expression::new_array_expression(SPAN, elements, &self.visitor.ast);
        self.count += 1;
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

impl<'a> DevupVisitor<'a> {
    pub(super) fn prepare_block_callback(
        &mut self,
        expression: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
    ) -> Option<(Vec<KnownPart<'a>>, Captured<'a>)> {
        let body: &mut FunctionBody<'a> = match expression {
            Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => match &mut arrow.body {
                ArrowFunctionBody::FunctionBody(body) => body,
                _ => return None,
            },
            Expression::FunctionExpression(function)
                if !function.r#async && !function.generator =>
            {
                function.body.as_mut()?
            }
            _ => return None,
        };
        let render = self.names.fresh("__devupRenderValues");
        let mut returns = Returns {
            visitor: self,
            render: render.clone(),
            parts: vec![],
            orders: None,
            count: 0,
            complete: true,
        };
        walk_mut::walk_function_body(&mut returns, body);
        if !returns.complete {
            return None;
        }
        let parts = returns.parts;
        let invocation = self.capture_prepared_callback(expression, captures);
        Some((parts, (render, invocation)))
    }

    pub(super) fn prepare_order_block_callback(
        &mut self,
        expression: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
    ) -> Option<(Vec<OrderStep<'a>>, Captured<'a>)> {
        let body = match expression {
            Expression::ArrowFunctionExpression(arrow) => match &mut arrow.body {
                ArrowFunctionBody::FunctionBody(body) => body,
                _ => return None,
            },
            Expression::FunctionExpression(function) => function.body.as_mut()?,
            _ => return None,
        };
        let render = self.names.fresh("__devupRenderValues");
        let mut orders = Vec::new();
        let mut returns = Returns {
            visitor: self,
            render: render.clone(),
            parts: vec![],
            orders: Some(&mut orders),
            count: 0,
            complete: true,
        };
        walk_mut::walk_function_body(&mut returns, body);
        let invocation = self.capture_prepared_callback(expression, captures);
        Some((orders, (render, invocation)))
    }
}

fn saved_slot<'a>(
    ast: &oxc_ast::builder::AstBuilder<'a>,
    name: &str,
    index: usize,
) -> Expression<'a> {
    Expression::new_chain_expression(
        SPAN,
        oxc_ast::ast::ChainElement::ComputedMemberExpression(
            oxc_ast::ast::ComputedMemberExpression::boxed(
                SPAN,
                Expression::new_identifier(SPAN, ast.allocator().alloc_str(name), ast),
                Expression::new_string_literal(
                    SPAN,
                    ast.allocator().alloc_str(&index.to_string()),
                    None,
                    ast,
                ),
                true,
                ast,
            ),
        ),
        ast,
    )
}
