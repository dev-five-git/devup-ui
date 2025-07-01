use crate::ExtractStyleProp;
use crate::gen_class_name::{
    apply_class_name_attribute, gen_class_names, merge_expression_for_class_name,
};
use crate::gen_style::{apply_style_attribute, gen_styles};
use oxc_allocator::{CloneIn, Vec};
use oxc_ast::AstBuilder;
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Expression, JSXAttribute, JSXAttributeItem, JSXAttributeValue, JSXExpression,
    ObjectPropertyKind, PropertyKey, PropertyKind, TemplateElementValue,
};
use oxc_span::SPAN;

/// modify object props
pub fn modify_prop_object<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<ObjectPropertyKind<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
) {
    let mut class_name_prop = None;
    let mut style_prop = None;

    for idx in (0..props.len()).rev() {
        let prop = props.remove(idx);
        if let ObjectPropertyKind::ObjectProperty(attr) = prop {
            if let PropertyKey::StaticIdentifier(ident) = &attr.key {
                if ident.name == "className" {
                    class_name_prop = Some(attr);
                    continue;
                } else if ident.name == "style" {
                    style_prop = Some(attr);
                    continue;
                }
            }
            props.insert(idx, ObjectPropertyKind::ObjectProperty(attr));
        } else {
            props.insert(idx, prop);
        }
    }

    // should modify class name prop
    if let Some(ex) = gen_class_names(ast_builder, styles, style_order) {
        if let Some(pr) = if let Some(class_name_prop) = class_name_prop {
            merge_expression_for_class_name(
                ast_builder,
                vec![class_name_prop.value.clone_in(ast_builder.allocator), ex],
            )
            .map(|res| {
                ast_builder.alloc_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StaticIdentifier(
                        ast_builder.alloc_identifier_name(SPAN, "className"),
                    ),
                    res,
                    false,
                    false,
                    false,
                )
            })
        } else {
            Some(ast_builder.alloc_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StaticIdentifier(ast_builder.alloc_identifier_name(SPAN, "className")),
                ex,
                false,
                false,
                false,
            ))
        } {
            props.push(ObjectPropertyKind::ObjectProperty(pr));
        }
    } else if let Some(class_name_prop) = class_name_prop {
        // re add class name prop if not modified
        props.push(ObjectPropertyKind::ObjectProperty(class_name_prop))
    }

    // should modify style prop
    if let Some(mut ex) = gen_styles(ast_builder, styles) {
        props.push(if let Some(style_prop) = style_prop {
            ObjectPropertyKind::ObjectProperty(ast_builder.alloc_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StaticIdentifier(ast_builder.alloc_identifier_name(SPAN, "style")),
                if ex.properties.is_empty() {
                    Expression::ObjectExpression(ast_builder.alloc(ex))
                } else {
                    ex.properties.push(ObjectPropertyKind::SpreadProperty(
                        ast_builder.alloc_spread_element(
                            SPAN,
                            style_prop.value.clone_in(ast_builder.allocator),
                        ),
                    ));
                    Expression::ObjectExpression(ast_builder.alloc(ex))
                },
                false,
                false,
                false,
            ))
        } else {
            ObjectPropertyKind::ObjectProperty(ast_builder.alloc_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::StaticIdentifier(ast_builder.alloc_identifier_name(SPAN, "style")),
                Expression::ObjectExpression(ast_builder.alloc(ex)),
                false,
                false,
                false,
            ))
        });
    } else if let Some(style_prop) = style_prop {
        // re add class name prop if not modified
        props.push(ObjectPropertyKind::ObjectProperty(style_prop))
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
        if let Attribute(attr) = prop {
            if let Identifier(ident) = &attr.name {
                if ident.name == "className" {
                    class_name_prop = Some(attr);
                    continue;
                } else if ident.name == "style" {
                    style_prop = Some(attr);
                    continue;
                }
            }
            props.insert(idx, Attribute(attr));
        } else {
            props.insert(idx, prop);
        }
    }

    // should modify class name prop
    if let Some(ex) = gen_class_names(ast_builder, styles, style_order) {
        let mut attr = if let Some(class_name_prop) = class_name_prop {
            class_name_prop
        } else {
            ast_builder.alloc_jsx_attribute(
                SPAN,
                Identifier(ast_builder.alloc_jsx_identifier(SPAN, "className")),
                None,
            )
        };

        apply_class_name_attribute(ast_builder, &mut attr, ex);
        props.push(Attribute(attr));
    } else if let Some(class_name_prop) = class_name_prop {
        // re add class name prop if not modified
        props.push(Attribute(class_name_prop))
    }

    // should modify style prop
    if let Some(ex) = gen_styles(ast_builder, styles) {
        let mut attr = style_prop.unwrap_or_else(|| {
            ast_builder.alloc_jsx_attribute(
                SPAN,
                Identifier(ast_builder.alloc_jsx_identifier(SPAN, "style")),
                None,
            )
        });
        apply_style_attribute(ast_builder, &mut attr, ex);
        if let Some(mut style_vars) = style_vars {
            merge_style_vars(ast_builder, &mut attr, &mut style_vars);
        }
        props.push(Attribute(attr));
    } else if let Some(mut style_prop) = style_prop {
        // re add class name prop if not modified

        if let Some(mut style_vars) = style_vars {
            merge_style_vars(ast_builder, &mut style_prop, &mut style_vars);
        }
        props.push(Attribute(style_prop))
    } else if let Some(mut style_vars) = style_vars {
        let mut attr = style_prop.unwrap_or_else(|| {
            ast_builder.alloc_jsx_attribute(
                SPAN,
                Identifier(ast_builder.alloc_jsx_identifier(SPAN, "style")),
                None,
            )
        });
        merge_style_vars(ast_builder, &mut attr, &mut style_vars);
        props.push(Attribute(attr));
    }
}

/// Priority: dynamic style, style, styleVars
fn merge_style_vars<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &mut JSXAttribute<'a>,
    style_vars: &mut Expression<'a>,
) {
    convert_style_vars(ast_builder, style_vars);
    if let Some(ref mut value) = style_prop.value {
        if let JSXAttributeValue::ExpressionContainer(container) = value {
            style_prop.value = Some(JSXAttributeValue::ExpressionContainer(
                ast_builder.alloc_jsx_expression_container(
                    SPAN,
                    JSXExpression::ObjectExpression(
                        ast_builder.alloc_object_expression(
                            SPAN,
                            Vec::from_array_in(
                                [
                                    ObjectPropertyKind::SpreadProperty(
                                        ast_builder.alloc_spread_element(
                                            SPAN,
                                            style_vars.clone_in(ast_builder.allocator),
                                        ),
                                    ),
                                    ObjectPropertyKind::SpreadProperty(
                                        ast_builder.alloc_spread_element(
                                            SPAN,
                                            container
                                                .expression
                                                .clone_in(ast_builder.allocator)
                                                .into_expression(),
                                        ),
                                    ),
                                ],
                                ast_builder.allocator,
                            ),
                        ),
                    ),
                ),
            ));
        }
    } else {
        style_prop.value = Some(JSXAttributeValue::ExpressionContainer(
            ast_builder.alloc_jsx_expression_container(
                SPAN,
                style_vars.clone_in(ast_builder.allocator).into(),
            ),
        ));
    }
}

pub fn convert_style_vars<'a>(ast_builder: &AstBuilder<'a>, style_vars: &mut Expression<'a>) {
    if let Expression::ObjectExpression(obj) = style_vars {
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
                                    Vec::from_array_in(
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
                                    Vec::from_array_in(
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
}
