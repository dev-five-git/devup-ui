//! A dynamic `as` value belongs to props evaluation, not the first call argument.

use super::DevupVisitor;
use super::capture::Captured;
use crate::utils::call_with_values;
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, CallExpression, Expression, FormalParameterKind,
    FormalParameters,
};
use oxc_span::{GetSpan, SPAN};

impl<'a> DevupVisitor<'a> {
    fn bound_factory(&mut self, callee: Expression<'a>) -> Expression<'a> {
        let Expression::StaticMemberExpression(mut member) = callee else {
            return callee;
        };
        let receiver_name = self.names.fresh("__devupReceiver");
        let receiver = self.capture_as(receiver_name, &mut member.object);
        let read_receiver = member.object.clone_in(self.ast.allocator());
        let intrinsic = Expression::new_arrow_function_expression(
            SPAN,
            false,
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
            FormalParameters::boxed(
                SPAN,
                FormalParameterKind::ArrowFormalParameters,
                oxc_allocator::Vec::new_in(&self.ast),
                None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
                &self.ast,
            ),
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
            Expression::new_null_literal(SPAN, &self.ast).into(),
            &self.ast,
        );
        let bind = Expression::new_static_member_expression(
            SPAN,
            Expression::new_parenthesized_expression(SPAN, intrinsic, &self.ast),
            oxc_ast::ast::IdentifierName::new(SPAN, "bind", &self.ast),
            false,
            &self.ast,
        );
        let call = Expression::new_static_member_expression(
            SPAN,
            bind,
            oxc_ast::ast::IdentifierName::new(SPAN, "call", &self.ast),
            false,
            &self.ast,
        );
        let args = oxc_allocator::Vec::from_array_in(
            [
                Argument::from(Expression::StaticMemberExpression(member)),
                Argument::from(read_receiver),
            ],
            &self.ast,
        );
        let bound = Expression::new_call_expression(
            SPAN,
            call,
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
            args,
            false,
            &self.ast,
        );
        call_with_values(&self.ast, vec![receiver], bound)
    }

    /// Keep factory lookup before props, and trailing arguments after them.
    pub(super) fn finish_element_call(
        &mut self,
        call: &mut CallExpression<'a>,
        mut values: Vec<Captured<'a>>,
        default_tag: &str,
    ) {
        if values.is_empty() {
            return;
        }
        if matches!(
            call.arguments[0].to_expression(),
            Expression::StringLiteral(_)
        ) {
            let props = call.arguments[1].to_expression_mut().take_in(&self.ast);
            call.arguments[1] = Argument::from(call_with_values(&self.ast, values, props));
            return;
        }
        let target = call.arguments[0].to_expression_mut().take_in(&self.ast);
        call.arguments[0] = Argument::from(Expression::new_logical_expression(
            SPAN,
            target,
            oxc_syntax::operator::LogicalOperator::Or,
            Expression::new_string_literal(
                SPAN,
                oxc_ast::ast::Str::from_in(default_tag, self.ast.allocator()),
                None,
                &self.ast,
            ),
            &self.ast,
        ));
        let factory_name = self.names.fresh("__devupFactory");
        let (name, factory) = self.capture_as(factory_name, &mut call.callee);
        values.insert(0, (name, self.bound_factory(factory)));
        for argument in call.arguments.iter_mut().skip(2) {
            let span = argument.span();
            let original = argument.take_in(&self.ast);
            let (mut value, spread) = match original {
                Argument::SpreadElement(spread) => (
                    Expression::new_array_expression(
                        span,
                        oxc_allocator::Vec::from_array_in(
                            [ArrayExpressionElement::SpreadElement(spread)],
                            &self.ast,
                        ),
                        &self.ast,
                    ),
                    true,
                ),
                argument => (argument.into_expression(), false),
            };
            let name = self.names.fresh("__devupValue");
            values.push(self.capture_as(name, &mut value));
            *argument = if spread {
                Argument::new_spread_element(span, value, &self.ast)
            } else {
                value.into()
            };
        }
        let body = Expression::CallExpression(oxc_allocator::Box::new_in(
            call.clone_in_with_semantic_ids(self.ast.allocator()),
            &self.ast,
        ));
        if let Expression::CallExpression(wrapper) = call_with_values(&self.ast, values, body) {
            *call = wrapper.unbox();
        }
    }
}
