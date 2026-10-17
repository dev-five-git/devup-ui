//! `createElement(Component, props, ...children)`, which builds what
//! `jsx(Component, props)` builds: the props are written as the object the
//! compilation of a `jsx()` call reads, and the children stay arguments.

use super::DevupVisitor;
use crate::utils::unwrap_syntax_only;
use oxc_allocator::TakeIn;
use oxc_ast::ast::{Argument, CallExpression, Expression, ObjectPropertyKind, UnaryOperator};
use oxc_span::SPAN;

fn holds_nothing(bindings: &crate::scope::Bindings, props: &Expression<'_>) -> bool {
    match unwrap_syntax_only(props) {
        Expression::NullLiteral(_) => true,
        Expression::Identifier(identifier) => bindings.is_global_undefined(identifier),
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::Void
                && super::order::reach(bindings, &unary.argument) == super::order::Reach::Constant
        }
        _ => false,
    }
}

impl<'a> DevupVisitor<'a> {
    /// Write the props of `call`, a `createElement` call, as an object when it
    /// builds a Devup UI component; `false` when it does not, or when its props
    /// are a spread argument
    pub(super) fn lower_create_element(&self, call: &mut CallExpression<'a>) -> bool {
        let renders = call
            .arguments
            .first()
            .and_then(Argument::as_expression)
            .is_some_and(|component| self.bindings.kind(component).is_some());
        if !renders {
            return false;
        }
        let Some(argument) = call.arguments.get_mut(1) else {
            let empty = self.props_object(None);
            call.arguments.push(Argument::from(empty));
            return true;
        };
        let Some(props) = argument.as_expression_mut() else {
            return false;
        };
        if matches!(props, Expression::ObjectExpression(_)) {
            return true;
        }
        let written = if holds_nothing(&self.bindings, props) {
            None
        } else {
            Some(props.take_in(&self.ast))
        };
        *props = self.props_object(written);
        true
    }

    /// An object holding no props, or spreading `spread`
    fn props_object(&self, spread: Option<Expression<'a>>) -> Expression<'a> {
        let properties =
            spread.map(|spread| ObjectPropertyKind::new_spread_property(SPAN, spread, &self.ast));
        Expression::new_object_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(properties, &self.ast),
            &self.ast,
        )
    }
}
