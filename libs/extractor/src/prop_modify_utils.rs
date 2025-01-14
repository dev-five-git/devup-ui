use crate::gen_class_name::{apply_class_name_attribute, gen_class_names};
use crate::gen_style::{apply_style_attribute, gen_styles};
use crate::ExtractStyleProp;
use oxc_ast::ast::JSXAttributeItem;
use oxc_ast::ast::JSXAttributeItem::Attribute;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::AstBuilder;
use oxc_span::SPAN;

/// modify JSX props
pub fn modify_props<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<JSXAttributeItem<'a>>,
    styles: Vec<ExtractStyleProp<'a>>,
) {
    let mut class_name_prop = None;
    let mut style_prop = None;

    for idx in 0..props.len() {
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
