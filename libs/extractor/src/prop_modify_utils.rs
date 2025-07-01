use crate::ExtractStyleProp;
use crate::gen_class_name::gen_class_names;
use crate::gen_style::gen_styles;
use oxc_allocator::CloneIn;
use oxc_ast::AstBuilder;
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Expression, JSXAttributeItem, JSXAttributeValue, JSXExpression, ObjectPropertyKind,
    PropertyKey, PropertyKind, TemplateElementValue,
};
use oxc_span::SPAN;

/// modify object props
pub fn modify_prop_object<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<ObjectPropertyKind<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    style_vars: Option<Expression<'a>>,
) {
    let mut class_name_prop = None;
    let mut style_prop = None;
    for idx in (0..props.len()).rev() {
        let prop = props.remove(idx);
        match prop {
            ObjectPropertyKind::ObjectProperty(attr) => {
                if let PropertyKey::StaticIdentifier(ident) = &attr.key {
                    match ident.name.as_str() {
                        "className" => {
                            class_name_prop = Some(attr.value.clone_in(ast_builder.allocator));
                            continue;
                        }
                        "style" => {
                            style_prop = Some(attr.value.clone_in(ast_builder.allocator));
                            continue;
                        }
                        _ => {}
                    }
                }
                props.insert(idx, ObjectPropertyKind::ObjectProperty(attr));
            }
            _ => {
                props.insert(idx, prop);
            }
        }
    }

    if let Some(ex) = get_class_name_expression(ast_builder, &class_name_prop, styles, style_order)
    {
        props.push(ObjectPropertyKind::ObjectProperty(
            ast_builder.alloc_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StaticIdentifier(ast_builder.alloc_identifier_name(SPAN, "className")),
                ex,
                false,
                false,
                false,
            ),
        ));
    }
    if let Some(ex) = get_style_expression(ast_builder, &style_prop, styles, &style_vars) {
        props.push(ObjectPropertyKind::ObjectProperty(
            ast_builder.alloc_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StaticIdentifier(ast_builder.alloc_identifier_name(SPAN, "style")),
                ex,
                false,
                false,
                false,
            ),
        ));
    }
}
/// modify JSX props
pub fn modify_props<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<JSXAttributeItem<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    style_vars: Option<Expression<'a>>,
) {
    let mut class_name_prop = None;
    let mut style_prop = None;
    for idx in (0..props.len()).rev() {
        let prop = props.remove(idx);
        match prop {
            Attribute(attr) => {
                if let Identifier(ident) = &attr.name
                    && let Some(value) = &attr.value
                    && (ident.name == "className" || ident.name == "style")
                {
                    let value = match &value {
                        JSXAttributeValue::ExpressionContainer(container) => {
                            if matches!(container.expression, JSXExpression::EmptyExpression(_)) {
                                None
                            } else {
                                Some(
                                    container
                                        .expression
                                        .to_expression()
                                        .clone_in(ast_builder.allocator),
                                )
                            }
                        }
                        JSXAttributeValue::StringLiteral(literal) => {
                            Some(Expression::StringLiteral(ast_builder.alloc_string_literal(
                                SPAN,
                                literal.value,
                                None,
                            )))
                        }
                        _ => None,
                    };

                    match ident.name.as_str() {
                        "className" => class_name_prop = value,
                        "style" => style_prop = value,
                        _ => unreachable!(),
                    }

                    continue;
                }
                props.insert(idx, Attribute(attr));
            }
            _ => {
                props.insert(idx, prop);
            }
        }
    }
    if let Some(ex) = get_class_name_expression(ast_builder, &class_name_prop, styles, style_order)
    {
        props.push(Attribute(ast_builder.alloc_jsx_attribute(
            SPAN,
            Identifier(ast_builder.alloc_jsx_identifier(SPAN, "className")),
            Some(if let Expression::StringLiteral(literal) = ex {
                JSXAttributeValue::StringLiteral(literal)
            } else {
                JSXAttributeValue::ExpressionContainer(
                    ast_builder.alloc_jsx_expression_container(SPAN, ex.into()),
                )
            }),
        )));
    }
    if let Some(ex) = get_style_expression(ast_builder, &style_prop, styles, &style_vars) {
        props.push(Attribute(ast_builder.alloc_jsx_attribute(
            SPAN,
            Identifier(ast_builder.alloc_jsx_identifier(SPAN, "style")),
            Some(JSXAttributeValue::ExpressionContainer(
                ast_builder.alloc_jsx_expression_container(SPAN, ex.into()),
            )),
        )));
    }
}

pub fn get_class_name_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name_prop: &Option<Expression<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
) -> Option<Expression<'a>> {
    // should modify class name prop
    merge_string_expressions(
        ast_builder,
        [
            class_name_prop.clone_in(ast_builder.allocator),
            gen_class_names(ast_builder, styles, style_order),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .as_slice(),
    )
}

pub fn get_style_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &Option<Expression<'a>>,
    styles: &[ExtractStyleProp<'a>],
    style_vars: &Option<Expression<'a>>,
) -> Option<Expression<'a>> {
    merge_object_expressions(
        ast_builder,
        [
            gen_styles(ast_builder, styles),
            style_vars
                .as_ref()
                .map(|style_vars| convert_style_vars(ast_builder, style_vars)),
            style_prop.clone_in(ast_builder.allocator),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .as_slice(),
    )
}

fn merge_string_expressions<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: &[Expression<'a>],
) -> Option<Expression<'a>> {
    if expressions.is_empty() {
        return None;
    }
    if expressions.len() == 1 {
        return Some(expressions[0].clone_in(ast_builder.allocator));
    }

    let mut string_literals = vec![];
    let mut other_expressions = vec![];
    for ex in expressions {
        if let Expression::StringLiteral(literal) = ex {
            string_literals.push(literal.value.trim());
        } else {
            other_expressions.push(ex);
        }
    }
    if other_expressions.is_empty() {
        return Some(Expression::StringLiteral(ast_builder.alloc_string_literal(
            SPAN,
            ast_builder.atom(&string_literals.join(" ")),
            None,
        )));
    }

    Some(Expression::TemplateLiteral(
        ast_builder.alloc_template_literal(
            SPAN,
            oxc_allocator::Vec::from_iter_in(
                [ast_builder.template_element(
                    SPAN,
                    TemplateElementValue {
                        raw: ast_builder.atom(&format!("{} ", string_literals.join(" "))),
                        cooked: None,
                    },
                    false,
                )],
                ast_builder.allocator,
            ),
            oxc_allocator::Vec::from_iter_in(
                other_expressions
                    .into_iter()
                    .map(|ex| ex.clone_in(ast_builder.allocator)),
                ast_builder.allocator,
            ),
        ),
    ))
}

/// merge expressions to object expression
fn merge_object_expressions<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: &[Expression<'a>],
) -> Option<Expression<'a>> {
    if expressions.is_empty() {
        return None;
    }
    if expressions.len() == 1 {
        return Some(expressions[0].clone_in(ast_builder.allocator));
    }
    Some(Expression::ObjectExpression(
        ast_builder.alloc_object_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(
                expressions.into_iter().map(|ex| {
                    ObjectPropertyKind::SpreadProperty(
                        ast_builder.alloc_spread_element(SPAN, ex.clone_in(ast_builder.allocator)),
                    )
                }),
                ast_builder.allocator,
            ),
        ),
    ))
}

pub fn convert_style_vars<'a>(
    ast_builder: &AstBuilder<'a>,
    style_vars: &Expression<'a>,
) -> Expression<'a> {
    let mut style_vars = style_vars.clone_in(ast_builder.allocator);
    if let Expression::ObjectExpression(obj) = &mut style_vars {
        for idx in (0..obj.properties.len()).rev() {
            let mut prop = obj.properties.remove(idx);

            if let ObjectPropertyKind::ObjectProperty(prop) = &mut prop {
                let name = match &prop.key {
                    PropertyKey::StaticIdentifier(ident) => ident.name,
                    PropertyKey::StringLiteral(ident) => ident.value,
                    etc => {
                        obj.properties.insert(
                            idx,
                            ObjectPropertyKind::ObjectProperty(ast_builder.alloc_object_property(
                                SPAN,
                                PropertyKind::Init,
                                PropertyKey::TemplateLiteral(ast_builder.alloc_template_literal(
                                    SPAN,
                                    oxc_allocator::Vec::from_array_in(
                                        [ast_builder.template_element(
                                            SPAN,
                                            TemplateElementValue {
                                                raw: ast_builder.atom("--"),
                                                cooked: None,
                                            },
                                            false,
                                        )],
                                        ast_builder.allocator,
                                    ),
                                    oxc_allocator::Vec::from_array_in(
                                        [etc.to_expression().clone_in(ast_builder.allocator)],
                                        ast_builder.allocator,
                                    ),
                                )),
                                prop.value.clone_in(ast_builder.allocator),
                                false,
                                false,
                                true,
                            )),
                        );
                        continue;
                    }
                };

                if !name.starts_with("--") {
                    prop.key = PropertyKey::StringLiteral(ast_builder.alloc_string_literal(
                        SPAN,
                        ast_builder.atom(&format!("--{name}")),
                        None,
                    ));
                }
            }
            obj.properties.insert(idx, prop);
        }
    }
    style_vars
}
