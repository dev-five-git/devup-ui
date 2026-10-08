use oxc_allocator::{Allocator, Box, CloneIn};
use oxc_ast::ast::{CallExpression, Expression, TemplateLiteral};

/// Class payloads retained at the two typography constructors.
#[derive(Debug)]
pub(crate) enum RuleClass<'a> {
    Template(Box<'a, TemplateLiteral<'a>>),
    Call(Box<'a, CallExpression<'a>>),
}

impl<'a> RuleClass<'a> {
    pub(crate) const fn into_expression(self) -> Expression<'a> {
        match self {
            Self::Template(value) => Expression::TemplateLiteral(value),
            Self::Call(value) => Expression::CallExpression(value),
        }
    }

    pub(crate) fn clone_payload(&self, allocator: &'a Allocator) -> Self {
        match self {
            Self::Template(value) => Self::Template(value.clone_in(allocator)),
            Self::Call(value) => Self::Call(value.clone_in(allocator)),
        }
    }
}
