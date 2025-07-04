use crate::{ExtractStyleProp, StyleProperty};
use oxc_allocator::CloneIn;
use oxc_ast::AstBuilder;
use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKey, PropertyKind};
use oxc_span::SPAN;
use std::collections::BTreeMap;
pub fn gen_styles<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &[ExtractStyleProp<'a>],
) -> Option<Expression<'a>> {
    if style_props.is_empty() {
        return None;
    }
    let properties: Vec<_> = style_props
        .iter()
        .flat_map(|style| gen_style(ast_builder, style))
        .rev()
        .collect();
    if properties.is_empty() {
        return None;
    }
    Some(ast_builder.expression_object(
        SPAN,
        oxc_allocator::Vec::from_iter_in(properties, ast_builder.allocator),
    ))
}
fn gen_style<'a>(
    ast_builder: &AstBuilder<'a>,
    style: &ExtractStyleProp<'a>,
) -> Vec<ObjectPropertyKind<'a>> {
    let mut properties = vec![];
    match style {
        ExtractStyleProp::Static(st) => match st.extract() {
            StyleProperty::ClassName(_) => {}
            StyleProperty::Variable {
                variable_name,
                identifier,
                ..
            } => {
                properties.push(ast_builder.object_property_kind_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StringLiteral(ast_builder.alloc_string_literal(
                        SPAN,
                        ast_builder.atom(&variable_name),
                        None,
                    )),
                    ast_builder.expression_identifier(SPAN, ast_builder.atom(&identifier)),
                    false,
                    false,
                    false,
                ));
            }
        },
        ExtractStyleProp::StaticArray(res) => {
            properties.append(
                &mut res
                    .iter()
                    .flat_map(|r| gen_style(ast_builder, r))
                    .rev()
                    .collect(),
            );
        }
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
        } => match (consequent, alternate) {
            (None, None) => {
                return vec![];
            }
            (None, Some(c)) => {
                gen_style(ast_builder, c).into_iter().for_each(|p| {
                    if let ObjectPropertyKind::ObjectProperty(p) = p {
                        properties.push(ast_builder.object_property_kind_object_property(
                            SPAN,
                            PropertyKind::Init,
                            p.key.clone_in(ast_builder.allocator),
                            ast_builder.expression_conditional(
                                SPAN,
                                condition.clone_in(ast_builder.allocator),
                                ast_builder.expression_identifier(SPAN, "undefined"),
                                p.value.clone_in(ast_builder.allocator),
                            ),
                            false,
                            false,
                            false,
                        ))
                    }
                });
            }
            (Some(c), None) => {
                gen_style(ast_builder, c).into_iter().for_each(|p| {
                    if let ObjectPropertyKind::ObjectProperty(p) = p {
                        properties.push(ast_builder.object_property_kind_object_property(
                            SPAN,
                            PropertyKind::Init,
                            p.key.clone_in(ast_builder.allocator),
                            ast_builder.expression_conditional(
                                SPAN,
                                condition.clone_in(ast_builder.allocator),
                                p.value.clone_in(ast_builder.allocator),
                                ast_builder.expression_identifier(SPAN, "undefined"),
                            ),
                            false,
                            false,
                            false,
                        ))
                    }
                });
            }
            (Some(c), Some(a)) => {
                let collect_c = gen_style(ast_builder, c);
                let collect_a = gen_style(ast_builder, a);
                if collect_c.is_empty() && collect_a.is_empty() {
                    return vec![];
                }
                for p in collect_c.iter() {
                    let mut found = false;
                    for q in collect_a.iter() {
                        if let (
                            ObjectPropertyKind::ObjectProperty(p),
                            ObjectPropertyKind::ObjectProperty(q),
                        ) = (p, q)
                        {
                            if p.key.name() == q.key.name() {
                                found = true;
                                properties.push(ast_builder.object_property_kind_object_property(
                                    SPAN,
                                    PropertyKind::Init,
                                    p.key.clone_in(ast_builder.allocator),
                                    ast_builder.expression_conditional(
                                        SPAN,
                                        condition.clone_in(ast_builder.allocator),
                                        p.value.clone_in(ast_builder.allocator),
                                        q.value.clone_in(ast_builder.allocator),
                                    ),
                                    false,
                                    false,
                                    false,
                                ));
                                break;
                            }
                        }
                    }
                    if !found {
                        if let ObjectPropertyKind::ObjectProperty(p) = p {
                            properties.push(ast_builder.object_property_kind_object_property(
                                SPAN,
                                PropertyKind::Init,
                                p.key.clone_in(ast_builder.allocator),
                                p.value.clone_in(ast_builder.allocator),
                                false,
                                false,
                                false,
                            ));
                        }
                    }
                }

                for q in collect_a.iter() {
                    let mut found = false;
                    for p in collect_c.iter() {
                        if let (
                            ObjectPropertyKind::ObjectProperty(p),
                            ObjectPropertyKind::ObjectProperty(q),
                        ) = (p, q)
                            && p.key.name() == q.key.name()
                        {
                            found = true;
                            break;
                        }
                    }
                    if !found && let ObjectPropertyKind::ObjectProperty(q) = q {
                        properties.push(ast_builder.object_property_kind_object_property(
                            SPAN,
                            PropertyKind::Init,
                            q.key.clone_in(ast_builder.allocator),
                            q.value.clone_in(ast_builder.allocator),
                            false,
                            false,
                            false,
                        ));
                    }
                }
            }
        },
        ExtractStyleProp::Expression { styles, .. } => {
            for style in styles {
                if let StyleProperty::Variable {
                    variable_name,
                    identifier,
                    ..
                } = style.extract()
                {
                    properties.push(ast_builder.object_property_kind_object_property(
                        SPAN,
                        PropertyKind::Init,
                        PropertyKey::StringLiteral(ast_builder.alloc_string_literal(
                            SPAN,
                            ast_builder.atom(&variable_name),
                            None,
                        )),
                        ast_builder.expression_identifier(SPAN, ast_builder.atom(&identifier)),
                        false,
                        false,
                        true,
                    ));
                }
            }
        }
        ExtractStyleProp::MemberExpression { map, expression } => {
            let mut tmp_map = BTreeMap::<String, Vec<(String, String)>>::new();
            for (key, value) in map.iter() {
                for style in value.extract() {
                    if let StyleProperty::Variable {
                        variable_name,
                        identifier,
                        ..
                    } = style.extract()
                    {
                        tmp_map
                            .entry(variable_name)
                            .or_default()
                            .push((key.to_string(), identifier));
                    }
                }
            }

            for (key, value) in tmp_map {
                properties.push(ast_builder.object_property_kind_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StringLiteral(ast_builder.alloc_string_literal(
                        SPAN,
                        ast_builder.atom(&key),
                        None,
                    )),
                    if value.len() == 1 {
                        // do not create object expression when property is single
                        ast_builder.expression_identifier(SPAN, ast_builder.atom(&value[0].1))
                    } else {
                        Expression::ComputedMemberExpression(
                            ast_builder.alloc_computed_member_expression(
                                SPAN,
                                ast_builder.expression_object(
                                    SPAN,
                                    oxc_allocator::Vec::from_iter_in(
                                        value
                                            .into_iter()
                                            .map(|(k, v)| {
                                                ast_builder.object_property_kind_object_property(
                                                    SPAN,
                                                    PropertyKind::Init,
                                                    ast_builder.property_key_static_identifier(
                                                        SPAN,
                                                        ast_builder.atom(&k),
                                                    ),
                                                    ast_builder.expression_identifier(
                                                        SPAN,
                                                        ast_builder.atom(&v),
                                                    ),
                                                    false,
                                                    false,
                                                    false,
                                                )
                                            })
                                            .collect::<Vec<_>>(),
                                        ast_builder.allocator,
                                    ),
                                ),
                                expression.clone_in(ast_builder.allocator),
                                false,
                            ),
                        )
                    },
                    false,
                    false,
                    false,
                ));
            }
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
