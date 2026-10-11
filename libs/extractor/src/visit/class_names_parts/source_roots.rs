use super::{
    Allocator, Argument, AstBuilder, BinaryOperator, CallExpression, CloneIn,
    ComputedMemberExpression, Expression, GetAllocator, SPAN, StaticMemberExpression,
    UnaryOperator,
};
use crate::extractor::rule_payload::RuleClass;
use crate::gen_class_name::roots::{
    CapturedClassBody, ClassConditional, ClassPayload, FinishedClass,
};
use crate::style_values::StyleValues;
use oxc_ast::ast::ConditionalExpression;

pub(in crate::visit) trait LocalClass<'a>: ClassConditional<'a> {
    fn from_rule(value: RuleClass<'a>) -> Self;
    fn clone_source_expression(&self, allocator: &'a Allocator) -> Expression<'a>;
    fn read_in(&mut self, values: &StyleValues, ast: &AstBuilder<'a>);
    fn string_class(&self, ast: &AstBuilder<'a>) -> Self;
}

macro_rules! local_root {
    ($name:ident, $($variant:ident => $expression:ident),* $(,)?) => {
        impl<'a> LocalClass<'a> for $name<'a> {
            fn from_rule(value: RuleClass<'a>) -> Self {
                match value {
                    RuleClass::Template(value) => Self::Template(value),
                    RuleClass::Call(value) => Self::Call(value),
                }
            }

            fn clone_source_expression(&self, allocator: &'a Allocator) -> Expression<'a> {
                match self {
                    Self::String(value) => Expression::StringLiteral(value.clone_in_with_semantic_ids(allocator)),
                    Self::Template(value) => Expression::TemplateLiteral(value.clone_in_with_semantic_ids(allocator)),
                    Self::Call(value) => Expression::CallExpression(value.clone_in_with_semantic_ids(allocator)),
                    Self::Conditional(value) => Expression::ConditionalExpression(value.clone_in_with_semantic_ids(allocator)),
                    $(Self::$variant(value) => Expression::$expression(value.clone_in_with_semantic_ids(allocator)),)*
                }
            }

            fn read_in(&mut self, values: &StyleValues, ast: &AstBuilder<'a>) {
                match self {
                    Self::String(_) => {},
                    Self::Template(value) => values.read_in_text(ast, value),
                    Self::Call(value) => read_call(values, ast, value),
                    Self::Conditional(value) => {
                        values.read_in(ast, &mut value.test);
                        values.read_in(ast, &mut value.consequent);
                        values.read_in(ast, &mut value.alternate);
                    }
                    $(Self::$variant(value) => read_leaf(values, ast, value),)*
                }
            }

            fn string_class(&self, ast: &AstBuilder<'a>) -> Self {
                match self {
                    Self::String(_) | Self::Template(_) => self.clone_payload(ast.allocator()),
                    Self::Call(_) | Self::Conditional(_) $(| Self::$variant(_))* => string_guard(ast, self),
                }
            }
        }
    };
}

local_root!(FinishedClass,);
local_root!(CapturedClassBody,
    Identifier => Identifier,
    StaticMember => StaticMemberExpression,
    ComputedMember => ComputedMemberExpression,
);

fn string_guard<'a, C: ClassConditional<'a>>(ast: &AstBuilder<'a>, value: &C) -> C {
    let is_string = Expression::new_binary_expression(
        SPAN,
        Expression::new_unary_expression(
            SPAN,
            UnaryOperator::Typeof,
            value.clone_expression(ast.allocator()),
            ast,
        ),
        BinaryOperator::StrictEquality,
        Expression::new_string_literal(SPAN, "string", None, ast),
        ast,
    );
    C::from_conditional(ConditionalExpression::boxed(
        SPAN,
        is_string,
        value.clone_expression(ast.allocator()),
        Expression::new_string_literal(SPAN, "", None, ast),
        ast,
    ))
}

fn read_call<'a>(values: &StyleValues, ast: &AstBuilder<'a>, value: &mut CallExpression<'a>) {
    values.read_in(ast, &mut value.callee);
    for argument in &mut value.arguments {
        match argument {
            Argument::SpreadElement(spread) => values.read_in(ast, &mut spread.argument),
            argument => values.read_in(ast, argument.to_expression_mut()),
        }
    }
}

trait ClassLeaf<'a> {
    fn read_leaf(&mut self, values: &StyleValues, ast: &AstBuilder<'a>);
}

fn read_leaf<'a, L: ClassLeaf<'a>>(values: &StyleValues, ast: &AstBuilder<'a>, value: &mut L) {
    value.read_leaf(values, ast);
}

impl<'a> ClassLeaf<'a> for oxc_allocator::Box<'a, oxc_ast::ast::IdentifierReference<'a>> {
    fn read_leaf(&mut self, _: &StyleValues, _: &AstBuilder<'a>) {}
}

impl<'a> ClassLeaf<'a> for oxc_allocator::Box<'a, StaticMemberExpression<'a>> {
    fn read_leaf(&mut self, values: &StyleValues, ast: &AstBuilder<'a>) {
        values.read_in(ast, &mut self.object);
    }
}

impl<'a> ClassLeaf<'a> for oxc_allocator::Box<'a, ComputedMemberExpression<'a>> {
    fn read_leaf(&mut self, values: &StyleValues, ast: &AstBuilder<'a>) {
        values.read_in(ast, &mut self.object);
        values.read_in(ast, &mut self.expression);
    }
}
