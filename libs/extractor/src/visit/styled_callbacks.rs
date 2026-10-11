use super::branch_capture::BranchReads;
use super::capture::Captured;
use super::literal_callback_order::{OrderEnvelope, OrderStep, WrappedCallback};
use super::{DevupVisitor, Text};
use crate::composition::{KnownPart, KnownStyles};
use oxc_allocator::{FromIn, GetAllocator, TakeIn};
use oxc_ast::{
    ast::{ArrowFunctionBody, Expression, Str},
    builder::AstBuilder,
};
use oxc_ast_visit::VisitMut;
use oxc_span::SPAN;
use rustc_hash::FxHashMap;

pub(super) enum OrderBody<'a> {
    Candidate(OrderEnvelope<'a>),
    Provenance(Expression<'a>),
}

impl<'a> OrderBody<'a> {
    pub(super) fn read(&mut self, reads: &mut BranchReads<'a, '_>) {
        match self {
            Self::Candidate(envelope) => reads.visit_expression(&mut envelope.candidate),
            Self::Provenance(expression) => reads.visit_expression(expression),
        }
    }

    pub(super) fn candidate(self, visitor: &DevupVisitor<'a>) -> Option<Expression<'a>> {
        match self {
            Self::Candidate(envelope) => visitor
                .order_envelope_is_rules(&envelope.expression(&visitor.ast))
                .then_some(envelope.candidate),
            Self::Provenance(_) => None,
        }
    }
}

impl<'a> DevupVisitor<'a> {
    fn order_envelope_is_rules(&self, expression: &Expression<'a>) -> bool {
        self.style_values.arrays(expression).is_none()
            && self.style_values.styles(expression).is_none()
            && self.style_values.finite(expression).is_none()
    }

    pub(super) fn prepare_order_body(
        &mut self,
        candidate: Expression<'a>,
    ) -> (OrderBody<'a>, Vec<Captured<'a>>) {
        let mut envelope = OrderEnvelope::new(candidate);
        let mut expression = envelope.expression(&self.ast);
        self.check_style_orders(&expression, false);
        let mut values = Vec::new();
        let body = if self.order_envelope_is_rules(&expression) {
            self.capture_order_shape(&mut envelope.candidate, &mut values);
            OrderBody::Candidate(envelope)
        } else {
            self.capture_shape(&mut expression, &mut values);
            OrderBody::Provenance(expression)
        };
        (body, values)
    }

    pub(super) fn prepare_order_callback(
        &mut self,
        expression: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
        wrapped: WrappedCallback<'a>,
    ) -> Option<(Vec<OrderStep<'a>>, Captured<'a>)> {
        if self.finite_callback(expression).is_some() {
            let (_, render) = self.prepare_callback(expression, captures)?;
            return Some((vec![OrderStep::Value(None)], render));
        }
        *expression = wrapped.original;
        let block = match expression {
            Expression::FunctionExpression(function) => {
                Some(self.prepare_order_block_callback(function.body.as_mut()?))
            }
            Expression::ArrowFunctionExpression(arrow) => match &mut arrow.body {
                ArrowFunctionBody::FunctionBody(body) => {
                    Some(self.prepare_order_block_callback(body))
                }
                _ => None,
            },
            _ => None,
        };
        if let Some((orders, render)) = block {
            let invocation = self.capture_prepared_callback(expression, captures);
            return Some((orders, (render, invocation)));
        }
        let candidate = callback_result(expression)?.take_in(&self.ast);
        let (mut body, values) = self.prepare_order_body(candidate);
        let render_name = self.names.fresh("__devupRenderValues");
        let mut reads = BranchReads {
            ast: &self.ast,
            reads: FxHashMap::default(),
        };
        let mut elements = oxc_allocator::Vec::new_in(&self.ast);
        for (index, (name, value)) in values.into_iter().enumerate() {
            reads
                .reads
                .insert(name, slot(&self.ast, &render_name, index));
            elements.push(value.into());
        }
        body.read(&mut reads);
        let parts = vec![OrderStep::Value(body.candidate(self))];
        *callback_result(expression)? = Expression::new_array_expression(SPAN, elements, &self.ast);
        let invocation = self.capture_prepared_callback(expression, captures);
        Some((parts, (render_name, invocation)))
    }

    pub(super) fn capture_prepared_callback(
        &mut self,
        expression: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
    ) -> Expression<'a> {
        let name = self.names.fresh("__devupCallback");
        captures.push(self.capture_as(name, expression));
        crate::utils::wrap_direct_call(
            &self.ast,
            expression,
            &[Expression::new_identifier(
                SPAN,
                "__devupStyleProps",
                &self.ast,
            )],
        )
    }

    pub(super) fn prepare_callback(
        &mut self,
        expression: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
    ) -> Option<(Vec<KnownPart<'a>>, Captured<'a>)> {
        if let Some(finite) = self.finite_callback(expression) {
            let render_name = self.names.fresh("__devupRenderClass");
            let parts = vec![KnownPart::Styles(vec![KnownStyles::Finite(
                finite,
                Expression::new_identifier(
                    SPAN,
                    self.ast.allocator().alloc_str(&render_name),
                    &self.ast,
                ),
            )])];
            let name = self.names.fresh("__devupCallback");
            captures.push(self.capture_as(name, expression));
            let invocation = crate::utils::wrap_direct_call(
                &self.ast,
                expression,
                &[Expression::new_identifier(
                    SPAN,
                    "__devupStyleProps",
                    &self.ast,
                )],
            );
            return Some((parts, (render_name, invocation)));
        }
        if matches!(expression, Expression::FunctionExpression(_))
            || matches!(expression, Expression::ArrowFunctionExpression(arrow) if matches!(arrow.body, oxc_ast::ast::ArrowFunctionBody::FunctionBody(_)))
        {
            return self.prepare_block_callback(expression, captures);
        }
        let body = callback_result(expression)?;
        crate::css_utils::literal_tree::lower(
            &self.ast,
            body,
            crate::css_utils::literal_tree::Scope {
                source: self.source,
                global: false,
            },
        );
        self.known_parts(body, &mut Vec::new(), Text::Classes)?;
        self.check_style_orders(body, false);
        let mut values = Vec::new();
        self.capture_shape(body, &mut values);
        let render_name = self.names.fresh("__devupRenderValues");
        let mut reads = BranchReads {
            ast: &self.ast,
            reads: FxHashMap::default(),
        };
        let mut elements = oxc_allocator::Vec::new_in(&self.ast);
        for (index, (name, value)) in values.into_iter().enumerate() {
            reads
                .reads
                .insert(name, slot(&self.ast, &render_name, index));
            elements.push(value.into());
        }
        reads.visit_expression(body);
        let mut parts = Vec::new();
        self.known_parts(body, &mut parts, Text::Classes)?;
        *callback_result(expression)? = Expression::new_array_expression(SPAN, elements, &self.ast);
        let name = self.names.fresh("__devupCallback");
        captures.push(self.capture_as(name, expression));
        let invocation = crate::utils::wrap_direct_call(
            &self.ast,
            expression,
            &[Expression::new_identifier(
                SPAN,
                "__devupStyleProps",
                &self.ast,
            )],
        );
        Some((parts, (render_name, invocation)))
    }
}

fn callback_result<'r, 'a>(expression: &'r mut Expression<'a>) -> Option<&'r mut Expression<'a>> {
    match expression {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => {
            arrow.body.as_expression_mut()
        }
        _ => None,
    }
}

fn slot<'a>(builder: &AstBuilder<'a>, name: &str, index: usize) -> Expression<'a> {
    Expression::new_computed_member_expression(
        SPAN,
        Expression::new_identifier(SPAN, Str::from_in(name, builder.allocator()), builder),
        Expression::new_string_literal(
            SPAN,
            Str::from_in(index.to_string().as_str(), builder.allocator()),
            None,
            builder,
        ),
        false,
        builder,
    )
}
