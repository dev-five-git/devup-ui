use crate::ExtractStyleProp;
use crate::extract_style::style_property::StyleProperty;
use crate::prop_modify_utils::convert_class_name;
use crate::utils::is_same_expression;
use oxc_allocator::CloneIn;
use oxc_ast::AstBuilder;
use oxc_ast::ast::{Expression, PropertyKey, PropertyKind, TemplateElementValue};
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
            .rev()
            .collect(),
    )
}

fn gen_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &mut ExtractStyleProp<'a>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    match style_prop {
        ExtractStyleProp::Static(st) => {
            if let Some(style_order) = style_order {
                st.set_style_order(style_order);
            }
            st.extract(filename).map(|style| {
                let v = ast_builder.atom(&match style {
                    StyleProperty::ClassName(cls) => cls,
                    StyleProperty::Variable { class_name, .. } => class_name,
                });
                ast_builder.expression_string_literal(SPAN, v, None)
            })
        }
        ExtractStyleProp::StaticArray(res) => merge_expression_for_class_name(
            ast_builder,
            res.iter_mut()
                .filter_map(|st| gen_class_name(ast_builder, st, style_order, filename))
                .collect(),
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
                .unwrap_or_else(|| ast_builder.expression_string_literal(SPAN, "", None));

            let alternate = alternate
                .as_mut()
                .and_then(|ref mut alt| gen_class_name(ast_builder, alt, style_order, filename))
                .unwrap_or_else(|| ast_builder.expression_string_literal(SPAN, "", None));
            if is_same_expression(&consequent, &alternate) {
                Some(consequent)
            } else {
                Some(ast_builder.expression_conditional(
                    SPAN,
                    condition.clone_in(ast_builder.allocator),
                    consequent,
                    alternate,
                ))
            }
        }
        ExtractStyleProp::Expression { expression, .. } => {
            Some(expression.clone_in(ast_builder.allocator))
        }
        // direct select
        ExtractStyleProp::MemberExpression { map, expression } => {
            let exp =
                Expression::ComputedMemberExpression(ast_builder.alloc_computed_member_expression(
                    SPAN,
                    ast_builder.expression_object(
                        SPAN,
                        ast_builder.vec_from_iter(map.iter_mut().filter_map(|(key, value)| {
                            gen_class_name(ast_builder, value.as_mut(), style_order, filename).map(
                                |expr| {
                                    ast_builder.object_property_kind_object_property(
                                        SPAN,
                                        PropertyKind::Init,
                                        PropertyKey::StringLiteral(
                                            ast_builder.alloc_string_literal(
                                                SPAN,
                                                ast_builder.atom(key),
                                                None,
                                            ),
                                        ),
                                        expr,
                                        false,
                                        false,
                                        false,
                                    )
                                },
                            )
                        })),
                    ),
                    expression.clone_in(ast_builder.allocator),
                    false,
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
    expressions: Vec<Expression<'a>>,
) -> Option<Expression<'a>> {
    let mut class_names = vec![];
    let mut unknown_expr = vec![];
    for expr in expressions {
        if let Expression::StringLiteral(str) = &expr {
            class_names.push(str.value.trim().to_string())
        } else {
            unknown_expr.push(expr);
        }
    }
    if unknown_expr.is_empty() && class_names.is_empty() {
        return None;
    }
    let mut class_name = class_names.join(" ");
    if !unknown_expr.is_empty() {
        if class_name.is_empty() && unknown_expr.len() == 1 {
            Some(unknown_expr.remove(0))
        } else {
            let mut qu = oxc_allocator::Vec::new_in(ast_builder.allocator);
            for idx in 0..unknown_expr.len() + 1 {
                let tail = idx == unknown_expr.len();
                let t = TemplateElementValue {
                    raw: ast_builder.atom(if idx == 0 {
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
                    }),
                    cooked: None,
                };
                qu.push(ast_builder.template_element(SPAN, t, tail));
            }

            Some(ast_builder.expression_template_literal(
                SPAN,
                qu,
                oxc_allocator::Vec::from_iter_in(unknown_expr, ast_builder.allocator),
            ))
        }
    } else if class_name.is_empty() {
        None
    } else {
        Some(ast_builder.expression_string_literal(SPAN, ast_builder.atom(&class_name), None))
    }
}
