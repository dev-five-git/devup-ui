use super::{ClassConditional, ClassMergeRoot, ClassPayload};
use oxc_allocator::{Allocator, Box, CloneIn};
use oxc_ast::ast::{
    ConditionalExpression, Expression, LogicalExpression, StringLiteral, TemplateLiteral,
};

/// Constructor products of recursive emission; lookups remain descendant expressions.
pub(crate) enum Generated<'a, E> {
    String(Box<'a, StringLiteral<'a>>),
    Template(Box<'a, TemplateLiteral<'a>>),
    Conditional(Box<'a, ConditionalExpression<'a>>),
    Lookup(Box<'a, LogicalExpression<'a>>),
    Supplied(E),
}

/// An equality-preserved operand or an actually constructed conditional root.
pub(crate) enum ConditionalEmission<'a, E> {
    String(Box<'a, StringLiteral<'a>>),
    Template(Box<'a, TemplateLiteral<'a>>),
    Supplied(E),
    Conditional(Box<'a, ConditionalExpression<'a>>),
}

impl<'a, E: ClassPayload<'a>> Generated<'a, E> {
    pub(crate) fn clone_expression(&self, alloc: &'a Allocator) -> Expression<'a> {
        match self {
            Self::String(value) => Expression::StringLiteral(value.clone_in(alloc)),
            Self::Template(value) => Expression::TemplateLiteral(value.clone_in(alloc)),
            Self::Conditional(value) => Expression::ConditionalExpression(value.clone_in(alloc)),
            Self::Lookup(value) => Expression::LogicalExpression(value.clone_in(alloc)),
            Self::Supplied(value) => value.clone_expression(alloc),
        }
    }

    pub(crate) fn into_expression(self) -> Expression<'a> {
        match self {
            Self::String(value) => Expression::StringLiteral(value),
            Self::Template(value) => Expression::TemplateLiteral(value),
            Self::Conditional(value) => Expression::ConditionalExpression(value),
            Self::Lookup(value) => Expression::LogicalExpression(value),
            Self::Supplied(value) => value.into_expression(),
        }
    }
}

impl<'a, E> ConditionalEmission<'a, E> {
    pub(crate) fn into_generated(self) -> Generated<'a, E> {
        match self {
            Self::String(value) => Generated::String(value),
            Self::Template(value) => Generated::Template(value),
            Self::Supplied(value) => Generated::Supplied(value),
            Self::Conditional(value) => Generated::Conditional(value),
        }
    }
}

impl<'a, E: ClassConditional<'a>> ConditionalEmission<'a, E> {
    pub(crate) fn into_payload(self) -> E {
        match self {
            Self::String(value) => E::from_string(value),
            Self::Template(value) => E::from_template(value),
            Self::Supplied(value) => value,
            Self::Conditional(value) => E::from_conditional(value),
        }
    }
}

impl<'a, E: ClassPayload<'a>> ClassMergeRoot<'a> for Generated<'a, E> {
    fn merge_string_value(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value.value.as_str()),
            Self::Supplied(value) => value.string_value(),
            Self::Template(_) | Self::Conditional(_) | Self::Lookup(_) => None,
        }
    }

    fn into_merge_expression(self) -> Expression<'a> {
        self.into_expression()
    }

    fn merge_string(value: Box<'a, StringLiteral<'a>>) -> Self {
        Self::String(value)
    }

    fn merge_template(value: Box<'a, TemplateLiteral<'a>>) -> Self {
        Self::Template(value)
    }
}
