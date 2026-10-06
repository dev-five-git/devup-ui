use super::branch_capture::BranchReads;
use super::capture::Captured;
use super::{DevupVisitor, Text};
use crate::composition::KnownPart;
use oxc_allocator::GetAllocator;
use oxc_ast::ast::{ArrowFunctionBody, Expression, FunctionBody, ReturnStatement};
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::SPAN;
use rustc_hash::FxHashMap;

struct Returns<'v, 'a> {
    visitor: &'v mut DevupVisitor<'a>,
    render: String,
    parts: Vec<KnownPart<'a>>,
    count: usize,
    complete: bool,
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
        let mut values = Vec::new();
        self.visitor.capture_shape(body, &mut values);
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
        self.parts.extend(self.visitor.guarded_parts(&test, parts));
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
            count: 0,
            complete: true,
        };
        walk_mut::walk_function_body(&mut returns, body);
        if !returns.complete {
            return None;
        }
        let parts = returns.parts;
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
        Some((parts, (render, invocation)))
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
