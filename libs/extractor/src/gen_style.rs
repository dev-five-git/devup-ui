use crate::{ExtractStyleProp, StyleProperty};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{
    Expression, JSXAttribute, JSXAttributeValue, JSXExpression, ObjectExpression,
    ObjectPropertyKind, PropertyKey, PropertyKind,
};
use oxc_ast::AstBuilder;
use oxc_span::SPAN;

pub fn gen_styles<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &[ExtractStyleProp<'a>],
) -> Option<ObjectExpression<'a>> {
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
    Some(ast_builder.object_expression(
        SPAN,
        oxc_allocator::Vec::from_iter_in(properties, ast_builder.allocator),
        None,
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
                properties.push(ObjectPropertyKind::ObjectProperty(
                    ast_builder.alloc_object_property(
                        SPAN,
                        PropertyKind::Init,
                        PropertyKey::StringLiteral(ast_builder.alloc_string_literal(
                            SPAN,
                            variable_name,
                            None,
                        )),
                        Expression::Identifier(
                            ast_builder.alloc_identifier_reference(SPAN, identifier),
                        ),
                        false,
                        false,
                        false,
                    ),
                ));
            }
        },
        ExtractStyleProp::Responsive(res) => {
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
            (Some(c), None) | (None, Some(c)) => {
                gen_style(ast_builder, c)
                    .into_iter()
                    .for_each(|p| properties.push(p));
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
                                properties.push(ObjectPropertyKind::ObjectProperty(
                                    ast_builder.alloc_object_property(
                                        SPAN,
                                        PropertyKind::Init,
                                        p.key.clone_in(ast_builder.allocator),
                                        Expression::ConditionalExpression(
                                            ast_builder.alloc_conditional_expression(
                                                SPAN,
                                                condition.clone_in(ast_builder.allocator),
                                                p.value.clone_in(ast_builder.allocator),
                                                q.value.clone_in(ast_builder.allocator),
                                            ),
                                        ),
                                        false,
                                        false,
                                        false,
                                    ),
                                ));
                                break;
                            }
                        }
                    }
                    if !found {
                        if let ObjectPropertyKind::ObjectProperty(p) = p {
                            properties.push(ObjectPropertyKind::ObjectProperty(
                                ast_builder.alloc_object_property(
                                    SPAN,
                                    PropertyKind::Init,
                                    p.key.clone_in(ast_builder.allocator),
                                    p.value.clone_in(ast_builder.allocator),
                                    false,
                                    false,
                                    false,
                                ),
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
                        {
                            if p.key.name() == q.key.name() {
                                found = true;
                                break;
                            }
                        }
                    }
                    if !found {
                        if let ObjectPropertyKind::ObjectProperty(q) = q {
                            properties.push(ObjectPropertyKind::ObjectProperty(
                                ast_builder.alloc_object_property(
                                    SPAN,
                                    PropertyKind::Init,
                                    q.key.clone_in(ast_builder.allocator),
                                    q.value.clone_in(ast_builder.allocator),
                                    false,
                                    false,
                                    false,
                                ),
                            ));
                        }
                    }
                }
            }
        },
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

pub fn apply_style_attribute<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &mut JSXAttribute<'a>,
    // must be an object expression
    mut expression: ObjectExpression<'a>,
) {
    if let Some(ref mut value) = style_prop.value {
        if let JSXAttributeValue::ExpressionContainer(container) = value {
            match container.expression {
                JSXExpression::ObjectExpression(ref mut obj) => {
                    expression.properties.insert(
                        0,
                        ObjectPropertyKind::SpreadProperty(ast_builder.alloc_spread_element(
                            SPAN,
                            Expression::ObjectExpression(obj.clone_in(ast_builder.allocator)),
                        )),
                    );
                }
                JSXExpression::Identifier(ref ident) => {
                    expression.properties.insert(
                        0,
                        ObjectPropertyKind::SpreadProperty(ast_builder.alloc_spread_element(
                            SPAN,
                            Expression::Identifier(ident.clone_in(ast_builder.allocator)),
                        )),
                    );
                }
                _ => {}
            };
            container.expression = JSXExpression::ObjectExpression(ast_builder.alloc(expression));
        };
    } else {
        style_prop.value = Some(JSXAttributeValue::ExpressionContainer(
            ast_builder.alloc_jsx_expression_container(
                SPAN,
                JSXExpression::ObjectExpression(ast_builder.alloc(expression)),
            ),
        ));
    };
}
