use super::branch_capture::BranchReads;
use super::capture::Captured;
use super::{DevupVisitor, Text};
use crate::composition::{KnownPart, KnownStyles};
use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, Str},
    builder::AstBuilder,
};
use oxc_ast_visit::VisitMut;
use oxc_span::SPAN;
use rustc_hash::FxHashMap;

impl<'a> DevupVisitor<'a> {
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
    use oxc_ast::ast::{ArrowFunctionBody, Statement};
    let statements = match expression {
        Expression::ArrowFunctionExpression(arrow) if !arrow.r#async => match &mut arrow.body {
            ArrowFunctionBody::FunctionBody(body) => &mut body.statements,
            body => return body.as_expression_mut(),
        },
        Expression::FunctionExpression(function) if !function.r#async && !function.generator => {
            &mut function.body.as_mut()?.statements
        }
        _ => return None,
    };
    match statements.last_mut()? {
        Statement::ReturnStatement(statement) => statement.argument.as_mut(),
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
