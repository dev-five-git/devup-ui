use super::source::PlainClass;
use super::{
    Allocator, AstBuilder, BinaryOperator, CloneIn, ComputedMemberExpression, Expression,
    GetAllocator, SPAN, StaticMemberExpression, UnaryOperator,
};
use crate::extractor::rule_payload::RuleClass;
use crate::gen_class_name::roots::{
    CapturedClassBody, CapturedSourceClass, ClassConditional, ClassPayload, FinishedClass,
    UncapturedSourceClass,
};
use crate::style_values::StyleValues;
use oxc_ast::ast::ConditionalExpression;

/// The finished result a class composes into, also built from emitted rule payloads
pub(in crate::visit) trait LocalOutput<'a>: ClassConditional<'a> {
    fn from_rule(value: RuleClass<'a>) -> Self;
}

/// A class read from the authored call; only the finished result can hold a call
pub(in crate::visit) trait LocalClass<'a>: ClassConditional<'a> {
    type Output: LocalOutput<'a>;
    fn from_plain(plain: PlainClass<'a>) -> Self;
    fn into_output(self) -> Self::Output;
    fn clone_source_expression(&self, allocator: &'a Allocator) -> Expression<'a>;
    fn read_in(&mut self, values: &StyleValues, ast: &AstBuilder<'a>);
    fn string_class(&self, ast: &AstBuilder<'a>) -> Self;
}

macro_rules! local_output {
    ($($name:ident),+) => {
        $(
            impl<'a> LocalOutput<'a> for $name<'a> {
                fn from_rule(value: RuleClass<'a>) -> Self {
                    match value {
                        RuleClass::Template(value) => Self::Template(value),
                        RuleClass::Call(value) => Self::Call(value),
                    }
                }
            }
        )+
    };
}

local_output!(FinishedClass, CapturedClassBody);

macro_rules! local_root {
    ($name:ident => $output:ident, $($variant:ident => $expression:ident),* $(,)?) => {
        impl<'a> LocalClass<'a> for $name<'a> {
            type Output = $output<'a>;

            fn from_plain(plain: PlainClass<'a>) -> Self {
                match plain {
                    PlainClass::String(value) => Self::String(value),
                    PlainClass::Template(value) => Self::Template(value),
                }
            }

            fn into_output(self) -> Self::Output {
                match self {
                    Self::String(value) => $output::String(value),
                    Self::Template(value) => $output::Template(value),
                    Self::Conditional(value) => $output::Conditional(value),
                    $(Self::$variant(value) => $output::$variant(value),)*
                }
            }

            fn clone_source_expression(&self, allocator: &'a Allocator) -> Expression<'a> {
                match self {
                    Self::String(value) => Expression::StringLiteral(value.clone_in_with_semantic_ids(allocator)),
                    Self::Template(value) => Expression::TemplateLiteral(value.clone_in_with_semantic_ids(allocator)),
                    Self::Conditional(value) => Expression::ConditionalExpression(value.clone_in_with_semantic_ids(allocator)),
                    $(Self::$variant(value) => Expression::$expression(value.clone_in_with_semantic_ids(allocator)),)*
                }
            }

            fn read_in(&mut self, values: &StyleValues, ast: &AstBuilder<'a>) {
                match self {
                    Self::String(_) => {},
                    Self::Template(value) => values.read_in_text(ast, value),
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
                    Self::Conditional(_) $(| Self::$variant(_))* => string_guard(ast, self),
                }
            }
        }
    };
}

local_root!(UncapturedSourceClass => FinishedClass,);
local_root!(CapturedSourceClass => CapturedClassBody,
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
