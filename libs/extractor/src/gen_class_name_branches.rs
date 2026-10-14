use super::{ExtractStyleProp, gen_class_name, merge_expression_for_class_name};
use crate::{prop_modify_utils::convert_class_name, utils::is_same_expression};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        ComputedMemberExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str,
        StringLiteral,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;
use std::collections::BTreeMap;

pub(super) struct ClassProjection<'b, 'a> {
    pub(super) ast: &'b AstBuilder<'a>,
    pub(super) order: Option<u8>,
    pub(super) filename: Option<&'b str>,
}

impl<'a> ClassProjection<'_, 'a> {
    fn class(&self, style: &mut ExtractStyleProp<'a>) -> Option<Expression<'a>> {
        gen_class_name(self.ast, style, self.order, self.filename)
    }

    pub(super) fn enum_class(
        &self,
        map: &mut BTreeMap<String, Vec<ExtractStyleProp<'a>>>,
        condition: &Expression<'a>,
    ) -> Expression<'a> {
        let ast_builder = self.ast;
        let properties = map.iter_mut().filter_map(|(key, value)| {
            merge_expression_for_class_name(
                ast_builder,
                value.iter_mut().filter_map(|v| self.class(v)),
            )
            .map(|class_name| {
                ObjectPropertyKind::new_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StringLiteral(StringLiteral::boxed(
                        SPAN,
                        Str::from_in(key, ast_builder.allocator()),
                        None,
                        ast_builder,
                    )),
                    class_name,
                    false,
                    false,
                    false,
                    ast_builder,
                )
            })
        });
        let obj = Expression::new_object_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(properties, ast_builder),
            ast_builder,
        );
        convert_class_name(
            ast_builder,
            &Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
                SPAN,
                obj,
                condition.clone_in(ast_builder.allocator()),
                false,
                ast_builder,
            )),
        )
    }

    pub(super) fn conditional_class(
        &self,
        condition: &Expression<'a>,
        branches: (
            &mut Option<Box<ExtractStyleProp<'a>>>,
            &mut Option<Box<ExtractStyleProp<'a>>>,
        ),
    ) -> Expression<'a> {
        let ast_builder = self.ast;
        let (consequent, alternate) = branches;
        let consequent = consequent
            .as_mut()
            .and_then(|ref mut con| self.class(con.as_mut()))
            .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast_builder));
        let alternate = alternate
            .as_mut()
            .and_then(|ref mut alt| self.class(alt))
            .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast_builder));
        if is_same_expression(&consequent, &alternate) {
            consequent
        } else {
            Expression::new_conditional_expression(
                SPAN,
                condition.clone_in(ast_builder.allocator()),
                consequent,
                alternate,
                ast_builder,
            )
        }
    }

    pub(super) fn member_class(
        &self,
        map: &mut BTreeMap<String, Box<ExtractStyleProp<'a>>>,
        expression: &Expression<'a>,
    ) -> Expression<'a> {
        let ast_builder = self.ast;
        let exp = Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
            SPAN,
            Expression::new_object_expression(
                SPAN,
                oxc_allocator::Vec::from_iter_in(
                    map.iter_mut().filter_map(|(key, value)| {
                        self.class(value.as_mut()).map(|expr| {
                            ObjectPropertyKind::new_object_property(
                                SPAN,
                                PropertyKind::Init,
                                PropertyKey::StringLiteral(StringLiteral::boxed(
                                    SPAN,
                                    Str::from_in(key, ast_builder.allocator()),
                                    None,
                                    ast_builder,
                                )),
                                expr,
                                false,
                                false,
                                false,
                                ast_builder,
                            )
                        })
                    }),
                    ast_builder,
                ),
                ast_builder,
            ),
            expression.clone_in(ast_builder.allocator()),
            false,
            ast_builder,
        ));
        if let Expression::Identifier(_) = &expression {
            convert_class_name(ast_builder, &exp)
        } else {
            exp
        }
    }
}
