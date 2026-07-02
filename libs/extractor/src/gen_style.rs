use crate::ExtractStyleProp;
use crate::extract_style::style_property::StyleProperty;
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{
    ComputedMemberExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str,
    StringLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use std::collections::BTreeMap;
pub fn gen_styles<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &[ExtractStyleProp<'a>],
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    if style_props.is_empty() {
        return None;
    }
    let properties: Vec<_> = style_props
        .iter()
        .flat_map(|style| gen_style(ast_builder, style, filename))
        .rev()
        .collect();
    if properties.is_empty() {
        return None;
    }
    Some(Expression::new_object_expression(
        SPAN,
        oxc_allocator::Vec::from_iter_in(properties, ast_builder),
        ast_builder,
    ))
}
fn gen_style<'a>(
    ast_builder: &AstBuilder<'a>,
    style: &ExtractStyleProp<'a>,
    filename: Option<&str>,
) -> Vec<ObjectPropertyKind<'a>> {
    let mut properties = vec![];
    if let ExtractStyleProp::Static(st) = style {
        if let Some(StyleProperty::Variable {
            variable_name,
            identifier,
            ..
        }) = st.extract(filename)
        {
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    SPAN,
                    Str::from_in(&variable_name, ast_builder.allocator()),
                    None,
                    ast_builder,
                )),
                Expression::new_identifier(
                    SPAN,
                    Str::from_in(&identifier, ast_builder.allocator()),
                    ast_builder,
                ),
                false,
                false,
                false,
                ast_builder,
            ));
        }
    } else if let ExtractStyleProp::StaticArray(res) = style {
        properties.append(
            &mut res
                .iter()
                .flat_map(|r| gen_style(ast_builder, r, filename))
                .rev()
                .collect(),
        );
    } else if let ExtractStyleProp::Conditional {
        condition,
        consequent,
        alternate,
    } = style
    {
        let r = (consequent, alternate);
        if let (None, Some(c)) = r {
            gen_style(ast_builder, c, filename)
                .into_iter()
                .for_each(|p| {
                    if let ObjectPropertyKind::ObjectProperty(p) = p {
                        properties.push(ObjectPropertyKind::new_object_property(
                            SPAN,
                            PropertyKind::Init,
                            p.key.clone_in(ast_builder.allocator()),
                            Expression::new_conditional_expression(
                                SPAN,
                                condition.clone_in(ast_builder.allocator()),
                                Expression::new_identifier(SPAN, "undefined", ast_builder),
                                p.value.clone_in(ast_builder.allocator()),
                                ast_builder,
                            ),
                            false,
                            false,
                            false,
                            ast_builder,
                        ));
                    }
                });
        } else if let (Some(c), None) = r {
            gen_style(ast_builder, c, filename)
                .into_iter()
                .for_each(|p| {
                    if let ObjectPropertyKind::ObjectProperty(p) = p {
                        properties.push(ObjectPropertyKind::new_object_property(
                            SPAN,
                            PropertyKind::Init,
                            p.key.clone_in(ast_builder.allocator()),
                            Expression::new_conditional_expression(
                                SPAN,
                                condition.clone_in(ast_builder.allocator()),
                                p.value.clone_in(ast_builder.allocator()),
                                Expression::new_identifier(SPAN, "undefined", ast_builder),
                                ast_builder,
                            ),
                            false,
                            false,
                            false,
                            ast_builder,
                        ));
                    }
                });
        } else if let (Some(c), Some(a)) = r {
            let collect_c = gen_style(ast_builder, c, filename);
            let collect_a = gen_style(ast_builder, a, filename);
            if collect_c.is_empty() && collect_a.is_empty() {
                return vec![];
            }
            for p in &collect_c {
                let found = collect_a.iter().any(|q| {
                    let r = matches!((p, q), (ObjectPropertyKind::ObjectProperty(p), ObjectPropertyKind::ObjectProperty(q)) if p.key.name() == q.key.name());
                    if let ObjectPropertyKind::ObjectProperty(p) = p
                        && let ObjectPropertyKind::ObjectProperty(q) = q
                        && r
                    {
                        properties.push(ObjectPropertyKind::new_object_property(SPAN, PropertyKind::Init, p.key.clone_in(ast_builder.allocator()), Expression::new_conditional_expression(SPAN, condition.clone_in(ast_builder.allocator()), p.value.clone_in(ast_builder.allocator()), q.value.clone_in(ast_builder.allocator()), ast_builder), false, false, false, ast_builder));
                    }
                    r
                });
                if !found && let ObjectPropertyKind::ObjectProperty(p) = p {
                    properties.push(ObjectPropertyKind::new_object_property(
                        SPAN,
                        PropertyKind::Init,
                        p.key.clone_in(ast_builder.allocator()),
                        p.value.clone_in(ast_builder.allocator()),
                        false,
                        false,
                        false,
                        ast_builder,
                    ));
                }
            }

            for q in &collect_a {
                let found = collect_c.iter().any(|p| matches!((p, q), (ObjectPropertyKind::ObjectProperty(p), ObjectPropertyKind::ObjectProperty(q)) if p.key.name() == q.key.name()));
                if !found && let ObjectPropertyKind::ObjectProperty(q) = q {
                    properties.push(ObjectPropertyKind::new_object_property(
                        SPAN,
                        PropertyKind::Init,
                        q.key.clone_in(ast_builder.allocator()),
                        q.value.clone_in(ast_builder.allocator()),
                        false,
                        false,
                        false,
                        ast_builder,
                    ));
                }
            }
        }
    } else if let ExtractStyleProp::MemberExpression { map, expression } = style {
        let mut tmp_map = BTreeMap::<String, Vec<(String, String)>>::new();
        for (key, value) in map {
            for style in value.extract() {
                if let Some(StyleProperty::Variable {
                    variable_name,
                    identifier,
                    ..
                }) = style.extract(filename)
                {
                    tmp_map
                        .entry(variable_name)
                        .or_default()
                        .push((key.clone(), identifier));
                }
            }
        }

        for (key, value) in tmp_map {
            let v = if value.len() == 1 {
                // do not create object expression when property is single
                Expression::new_identifier(
                    SPAN,
                    Str::from_in(&value[0].1, ast_builder.allocator()),
                    ast_builder,
                )
            } else {
                Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
                    SPAN,
                    Expression::new_object_expression(
                        SPAN,
                        oxc_allocator::Vec::from_iter_in(
                            value.into_iter().map(|(k, v)| {
                                ObjectPropertyKind::new_object_property(
                                    SPAN,
                                    PropertyKind::Init,
                                    PropertyKey::new_static_identifier(
                                        SPAN,
                                        Str::from_in(&k, ast_builder.allocator()),
                                        ast_builder,
                                    ),
                                    Expression::new_identifier(
                                        SPAN,
                                        Str::from_in(&v, ast_builder.allocator()),
                                        ast_builder,
                                    ),
                                    false,
                                    false,
                                    false,
                                    ast_builder,
                                )
                            }),
                            ast_builder,
                        ),
                        ast_builder,
                    ),
                    expression.clone_in(ast_builder.allocator()),
                    false,
                    ast_builder,
                ))
            };
            properties.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StringLiteral(StringLiteral::boxed(
                    SPAN,
                    Str::from_in(&key, ast_builder.allocator()),
                    None,
                    ast_builder,
                )),
                v,
                false,
                false,
                false,
                ast_builder,
            ));
        }
    }
    properties.sort_by_key(|p| {
        if let ObjectPropertyKind::ObjectProperty(p) = p {
            p.key.name()
        } else {
            None
        }
    });
    properties.reverse();
    properties
}
