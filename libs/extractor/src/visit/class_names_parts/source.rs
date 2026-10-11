use super::super::class_names_emission::NonemptyCaptures;
use super::{AstBuilder, CloneIn, Expression, GetAllocator, LocalClass, LocalOutput};
use crate::gen_class_name::roots::{
    CapturedClassBody, CapturedSourceClass, FinishedClass, UncapturedSourceClass,
};
use oxc_allocator::Box;
use oxc_ast::ast::{StringLiteral, TemplateLiteral};

pub(in crate::visit) trait LocalSource<'a> {
    type Class: LocalOutput<'a>;
    type Source: LocalClass<'a, Output = Self::Class>;
    fn class(&self, ast: &AstBuilder<'a>, expression: &Expression<'a>) -> Option<Self::Source>;
}

pub(in crate::visit) struct UncapturedSource;

/// The factory is a direct borrow of the actual nonempty capture owner.
pub(in crate::visit) type CapturedSource<'s, 'a> = &'s NonemptyCaptures<'a>;

/// A class both factories read the same way
pub(in crate::visit) enum PlainClass<'a> {
    String(Box<'a, StringLiteral<'a>>),
    Template(Box<'a, TemplateLiteral<'a>>),
}

fn plain<'a>(ast: &AstBuilder<'a>, expression: &Expression<'a>) -> Option<PlainClass<'a>> {
    match expression {
        Expression::StringLiteral(value) => Some(PlainClass::String(
            value.clone_in_with_semantic_ids(ast.allocator()),
        )),
        Expression::TemplateLiteral(value) => Some(PlainClass::Template(
            value.clone_in_with_semantic_ids(ast.allocator()),
        )),
        _ => None,
    }
}

impl<'a> LocalSource<'a> for UncapturedSource {
    type Class = FinishedClass<'a>;
    type Source = UncapturedSourceClass<'a>;

    fn class(&self, ast: &AstBuilder<'a>, expression: &Expression<'a>) -> Option<Self::Source> {
        plain(ast, expression).map(UncapturedSourceClass::from_plain)
    }
}

impl<'a> LocalSource<'a> for CapturedSource<'_, 'a> {
    type Class = CapturedClassBody<'a>;
    type Source = CapturedSourceClass<'a>;

    fn class(&self, ast: &AstBuilder<'a>, expression: &Expression<'a>) -> Option<Self::Source> {
        match expression {
            Expression::Identifier(value) => Some(CapturedSourceClass::Identifier(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            Expression::StaticMemberExpression(value) => Some(CapturedSourceClass::StaticMember(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            Expression::ComputedMemberExpression(value) => {
                Some(CapturedSourceClass::ComputedMember(
                    value.clone_in_with_semantic_ids(ast.allocator()),
                ))
            }
            other => plain(ast, other).map(CapturedSourceClass::from_plain),
        }
    }
}
