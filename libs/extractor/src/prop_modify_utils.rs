use crate::gen_class_name::{
    apply_class_name_attribute, gen_class_names, merge_expression_for_class_name,
};
use crate::gen_style::{apply_style_attribute, gen_styles};
use crate::ExtractStyleProp;
use oxc_allocator::CloneIn;
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{Expression, JSXAttributeItem, ObjectPropertyKind, PropertyKey, PropertyKind};
use oxc_ast::AstBuilder;
use oxc_span::SPAN;

/// modify object props
pub fn modify_prop_object<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<ObjectPropertyKind<'a>>,
    styles: Vec<ExtractStyleProp<'a>>,
) {
    let mut class_name_prop = None;
    let mut style_prop = None;

    for idx in 0..props.len() {
        if let ObjectPropertyKind::ObjectProperty(attr) = &props[idx] {
            if let PropertyKey::StaticIdentifier(ident) = &attr.key {
                if ident.name == "className" {
                    if let ObjectPropertyKind::ObjectProperty(attr) = props.remove(idx) {
                        class_name_prop = Some(attr);
                    }
                } else if ident.name == "style" {
                    if let ObjectPropertyKind::ObjectProperty(attr) = props.remove(idx) {
                        style_prop = Some(attr);
                    }
                }
            }
        }
    }

    // should modify class name prop
    if let Some(ex) = gen_class_names(ast_builder, &styles) {
        if let Some(pr) = if let Some(class_name_prop) = class_name_prop {
            let res = merge_expression_for_class_name(
                ast_builder,
                vec![class_name_prop.value.clone_in(ast_builder.allocator), ex],
            );
            res.map(|res| {
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
    if let Some(mut ex) = gen_styles(ast_builder, &styles) {
        if let Some(style_prop) = style_prop {
            props.push(ObjectPropertyKind::ObjectProperty(
                ast_builder.alloc_object_property(
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
                ),
            ));
        } else {
            props.push(ObjectPropertyKind::ObjectProperty(
                ast_builder.alloc_object_property(
                    SPAN,
                    PropertyKind::Init,
                    PropertyKey::StaticIdentifier(ast_builder.alloc_identifier_name(SPAN, "style")),
                    Expression::ObjectExpression(ast_builder.alloc(ex)),
                    false,
                    false,
                    false,
                ),
            ));
        };
    } else if let Some(style_prop) = style_prop {
        // re add class name prop if not modified
        props.push(ObjectPropertyKind::ObjectProperty(style_prop))
    }
}
/// modify JSX props
pub fn modify_props<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<JSXAttributeItem<'a>>,
    styles: Vec<ExtractStyleProp<'a>>,
) {
    let mut class_name_prop = None;
    let mut style_prop = None;

    for idx in (0..props.len()).rev() {
        if let Attribute(attr) = &props[idx] {
            if let Identifier(ident) = &attr.name {
                if ident.name == "className" {
                    if let Attribute(attr) = props.remove(idx) {
                        class_name_prop = Some(attr);
                    }
                } else if ident.name == "style" {
                    if let Attribute(attr) = props.remove(idx) {
                        style_prop = Some(attr);
                    }
                }
            }
        }
    }

    // should modify class name prop
    if let Some(ex) = gen_class_names(ast_builder, &styles) {
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
    if let Some(ex) = gen_styles(ast_builder, &styles) {
        let mut attr = if let Some(style_prop) = style_prop {
            style_prop
        } else {
            ast_builder.alloc_jsx_attribute(
                SPAN,
                Identifier(ast_builder.alloc_jsx_identifier(SPAN, "style")),
                None,
            )
        };
        apply_style_attribute(ast_builder, &mut attr, ex);
        props.push(Attribute(attr));
    } else if let Some(style_prop) = style_prop {
        // re add class name prop if not modified
        props.push(Attribute(style_prop))
    }
}
