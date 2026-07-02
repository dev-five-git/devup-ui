use crate::ExtractStyleProp;
use crate::extract_style::style_property::StyleProperty;
use crate::prop_modify_utils::convert_class_name;
use crate::utils::is_same_expression;
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{
    ComputedMemberExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str,
    StringLiteral, TemplateElement, TemplateElementValue,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;

pub fn gen_class_names<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    merge_expression_for_class_name(
        ast_builder,
        style_props
            .iter_mut()
            .filter_map(|st| gen_class_name(ast_builder, st, style_order, filename))
            .rev(),
    )
}

fn gen_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &mut ExtractStyleProp<'a>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    match style_prop {
        ExtractStyleProp::Enum { map, condition } => {
            let properties = map.iter_mut().filter_map(|(key, value)| {
                merge_expression_for_class_name(
                    ast_builder,
                    value
                        .iter_mut()
                        .filter_map(|v| gen_class_name(ast_builder, v, style_order, filename)),
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
            Some(convert_class_name(
                ast_builder,
                &Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
                    SPAN,
                    obj,
                    condition.clone_in(ast_builder.allocator()),
                    false,
                    ast_builder,
                )),
            ))
        }
        ExtractStyleProp::Static(st) => {
            if let Some(style_order) = style_order {
                st.set_style_order(style_order);
            }
            st.extract(filename).map(|style| {
                let v = Str::from_in(
                    &match style {
                        StyleProperty::ClassName(cls) => cls,
                        StyleProperty::Variable { class_name, .. } => class_name,
                    },
                    ast_builder.allocator(),
                );
                Expression::new_string_literal(SPAN, v, None, ast_builder)
            })
        }
        ExtractStyleProp::StaticArray(res) => merge_expression_for_class_name(
            ast_builder,
            res.iter_mut()
                .filter_map(|st| gen_class_name(ast_builder, st, style_order, filename)),
        ),
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
            ..
        } => {
            let consequent = consequent
                .as_mut()
                .and_then(|ref mut con| {
                    gen_class_name(ast_builder, con.as_mut(), style_order, filename)
                })
                .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast_builder));

            let alternate = alternate
                .as_mut()
                .and_then(|ref mut alt| gen_class_name(ast_builder, alt, style_order, filename))
                .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast_builder));
            if is_same_expression(&consequent, &alternate) {
                Some(consequent)
            } else {
                Some(Expression::new_conditional_expression(
                    SPAN,
                    condition.clone_in(ast_builder.allocator()),
                    consequent,
                    alternate,
                    ast_builder,
                ))
            }
        }
        ExtractStyleProp::Expression { expression, .. } => {
            Some(expression.clone_in(ast_builder.allocator()))
        }
        // direct select
        ExtractStyleProp::MemberExpression { map, expression } => {
            let exp = Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
                SPAN,
                Expression::new_object_expression(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(
                        map.iter_mut().filter_map(|(key, value)| {
                            gen_class_name(ast_builder, value.as_mut(), style_order, filename).map(
                                |expr| {
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
                                },
                            )
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
                Some(convert_class_name(ast_builder, &exp))
            } else {
                Some(exp)
            }
        }
    }
}

pub fn merge_expression_for_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: impl IntoIterator<Item = Expression<'a>>,
) -> Option<Expression<'a>> {
    let mut unknown_expr = vec![];
    let mut class_name = String::new();
    for expr in expressions {
        if let Expression::StringLiteral(str) = &expr {
            let value = str.value.trim();
            if !value.is_empty() {
                if !class_name.is_empty() {
                    class_name.push(' ');
                }
                class_name.push_str(value);
            }
        } else {
            unknown_expr.push(expr);
        }
    }
    if unknown_expr.is_empty() {
        if class_name.is_empty() {
            return None;
        }
        return Some(Expression::new_string_literal(
            SPAN,
            Str::from_in(&class_name, ast_builder.allocator()),
            None,
            ast_builder,
        ));
    }
    if class_name.is_empty() && unknown_expr.len() == 1 {
        Some(unknown_expr.remove(0))
    } else {
        let mut qu = oxc_allocator::Vec::new_in(ast_builder);
        for idx in 0..=unknown_expr.len() {
            let tail = idx == unknown_expr.len();
            let t = TemplateElementValue {
                raw: Str::from_in(
                    if idx == 0 {
                        if class_name.is_empty() {
                            ""
                        } else {
                            class_name.push(' ');
                            class_name.as_str()
                        }
                    } else if tail {
                        ""
                    } else {
                        " "
                    },
                    ast_builder.allocator(),
                ),
                cooked: None,
            };
            qu.push(TemplateElement::new(SPAN, t, tail, ast_builder));
        }

        Some(Expression::new_template_literal(
            SPAN,
            qu,
            oxc_allocator::Vec::from_iter_in(unknown_expr, ast_builder),
            ast_builder,
        ))
    }
}
