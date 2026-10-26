//! Typed callback parts shared by theme-read analysis.

use oxc_ast::ast::{ArrowFunctionBody, Expression, FormalParameters, FunctionBody};

pub(super) enum Body<'r, 'a> {
    Arrow(&'r ArrowFunctionBody<'a>),
    Function(Option<&'r FunctionBody<'a>>),
}

pub(super) struct Callback<'r, 'a> {
    pub params: &'r FormalParameters<'a>,
    pub body: Body<'r, 'a>,
}

impl<'r, 'a> Callback<'r, 'a> {
    pub fn read(expression: &'r Expression<'a>) -> Option<Self> {
        match expression {
            Expression::ArrowFunctionExpression(arrow) => Some(Self {
                params: &arrow.params,
                body: Body::Arrow(&arrow.body),
            }),
            Expression::FunctionExpression(function) => Some(Self {
                params: &function.params,
                body: Body::Function(function.body.as_deref()),
            }),
            _ => None,
        }
    }
}
