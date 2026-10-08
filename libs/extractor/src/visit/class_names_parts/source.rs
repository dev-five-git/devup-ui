use super::super::class_names_emission::NonemptyCaptures;
use super::{AstBuilder, CloneIn, Expression, GetAllocator, LocalClass};
use crate::gen_class_name::roots::{CapturedClassBody, ClassConstructors, FinishedClass};

pub(in crate::visit) trait LocalSource<'a> {
    type Class: LocalClass<'a>;
    fn class(&self, ast: &AstBuilder<'a>, expression: &Expression<'a>) -> Option<Self::Class>;
}

pub(in crate::visit) struct UncapturedSource;

/// The factory is a direct borrow of the actual nonempty capture owner.
pub(in crate::visit) type CapturedSource<'s, 'a> = &'s NonemptyCaptures<'a>;

impl<'a> LocalSource<'a> for UncapturedSource {
    type Class = FinishedClass<'a>;

    fn class(&self, ast: &AstBuilder<'a>, expression: &Expression<'a>) -> Option<Self::Class> {
        match expression {
            Expression::StringLiteral(value) => Some(FinishedClass::from_string(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            Expression::TemplateLiteral(value) => Some(FinishedClass::from_template(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            _ => None,
        }
    }
}

impl<'a> LocalSource<'a> for CapturedSource<'_, 'a> {
    type Class = CapturedClassBody<'a>;

    fn class(&self, ast: &AstBuilder<'a>, expression: &Expression<'a>) -> Option<Self::Class> {
        match expression {
            Expression::StringLiteral(value) => Some(CapturedClassBody::String(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            Expression::TemplateLiteral(value) => Some(CapturedClassBody::Template(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            Expression::Identifier(value) => Some(CapturedClassBody::Identifier(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            Expression::StaticMemberExpression(value) => Some(CapturedClassBody::StaticMember(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            Expression::ComputedMemberExpression(value) => Some(CapturedClassBody::ComputedMember(
                value.clone_in_with_semantic_ids(ast.allocator()),
            )),
            _ => None,
        }
    }
}
