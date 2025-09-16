use crate::ExtractStyleProp;
use crate::gen_class_name::gen_class_names;
use crate::gen_style::gen_styles;
use oxc_allocator::CloneIn;
use oxc_ast::AstBuilder;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Expression, JSXAttributeItem, JSXAttributeValue, LogicalOperator, ObjectPropertyKind,
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
    props_prop: Option<Expression<'a>>,
    filename: Option<&str>,
) {
    let mut class_name_prop = None;
    let mut style_prop = None;
    let mut spread_props = vec![];
    for idx in (0..props.len()).rev() {
        let prop = props.remove(idx);
        match prop {
            ObjectPropertyKind::ObjectProperty(attr) => {
                if let PropertyKey::StaticIdentifier(ident) = &attr.key {
                    let name = ident.name.as_str();
                    if name == "className" {
                        class_name_prop = Some(attr.value.clone_in(ast_builder.allocator));
                    } else if name == "style" {
                        style_prop = Some(attr.value.clone_in(ast_builder.allocator));
                    } else {
                        props.insert(idx, ObjectPropertyKind::ObjectProperty(attr));
                    }
                } else {
                    props.insert(idx, ObjectPropertyKind::ObjectProperty(attr));
                }
            }
            ObjectPropertyKind::SpreadProperty(spread) => {
                spread_props.push(spread.argument.clone_in(ast_builder.allocator));
                props.insert(idx, ObjectPropertyKind::SpreadProperty(spread));
            }
        }
    }

    if let Some(ex) = get_class_name_expression(
        ast_builder,
        &class_name_prop,
        styles,
        style_order,
        &spread_props,
        filename,
    ) {
        props.push(ast_builder.object_property_kind_object_property(
            SPAN,
            PropertyKind::Init,
            ast_builder.property_key_static_identifier(SPAN, "className"),
            ex,
            false,
            false,
            false,
        ));
    }
    if let Some(ex) = get_style_expression(
        ast_builder,
        &style_prop,
        styles,
        &style_vars,
        &spread_props,
        filename,
    ) {
        props.push(ast_builder.object_property_kind_object_property(
            SPAN,
            PropertyKind::Init,
            ast_builder.property_key_static_identifier(SPAN, "style"),
            ex,
            false,
            false,
            false,
        ));
    }
    if let Some(ex) = props_prop {
        props.push(
            ast_builder
                .object_property_kind_spread_property(SPAN, ex.clone_in(ast_builder.allocator)),
        );
    }
}
/// modify JSX props
pub fn modify_props<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<JSXAttributeItem<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    style_vars: Option<Expression<'a>>,
    props_prop: Option<Expression<'a>>,
    filename: Option<&str>,
) {
    let mut class_name_prop = None;
    let mut style_prop = None;
    let mut spread_props = vec![];
    for idx in (0..props.len()).rev() {
        let prop = props.remove(idx);
        match prop {
            JSXAttributeItem::Attribute(attr) => {
                if let Identifier(ident) = &attr.name
                    && let Some(value) = &attr.value
                    && (ident.name == "className" || ident.name == "style")
                {
                    let mut res = None;

                    if let JSXAttributeValue::ExpressionContainer(container) = &value {
                        res = container
                            .expression
                            .as_expression()
                            .map(|expression| expression.clone_in(ast_builder.allocator));
                    } else if let JSXAttributeValue::StringLiteral(literal) = &value {
                        res =
                            Some(ast_builder.expression_string_literal(SPAN, literal.value, None));
                    }
                    let name = ident.name.as_str();
                    if name == "className" {
                        class_name_prop = res;
                    } else if name == "style" {
                        style_prop = res;
                    }
                } else {
                    props.insert(idx, JSXAttributeItem::Attribute(attr));
                }
            }
            JSXAttributeItem::SpreadAttribute(spread) => {
                spread_props.push(spread.argument.clone_in(ast_builder.allocator));
                props.insert(idx, JSXAttributeItem::SpreadAttribute(spread));
            }
        }
    }
    if let Some(ex) = get_class_name_expression(
        ast_builder,
        &class_name_prop,
        styles,
        style_order,
        &spread_props,
        filename,
    ) {
        props.push(ast_builder.jsx_attribute_item_attribute(
            SPAN,
            ast_builder.jsx_attribute_name_identifier(SPAN, "className"),
            Some(if let Expression::StringLiteral(literal) = ex {
                JSXAttributeValue::StringLiteral(literal)
            } else {
                ast_builder.jsx_attribute_value_expression_container(SPAN, ex.into())
            }),
        ));
    }
    if let Some(ex) = get_style_expression(
        ast_builder,
        &style_prop,
        styles,
        &style_vars,
        &spread_props,
        filename,
    ) {
        props.push(ast_builder.jsx_attribute_item_attribute(
            SPAN,
            ast_builder.jsx_attribute_name_identifier(SPAN, "style"),
            Some(ast_builder.jsx_attribute_value_expression_container(SPAN, ex.into())),
        ));
    }
    if let Some(props_prop) = props_prop {
        props.push(
            ast_builder.jsx_attribute_item_spread_attribute(
                SPAN,
                props_prop.clone_in(ast_builder.allocator),
            ),
        );
    }
}

pub fn get_class_name_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name_prop: &Option<Expression<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    spread_props: &[Expression<'a>],
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    // should modify class name prop
    merge_string_expressions(
        ast_builder,
        [
            class_name_prop
                .as_ref()
                .map(|class_name| convert_class_name(ast_builder, class_name)),
            gen_class_names(ast_builder, styles, style_order, filename),
        ]
        .into_iter()
        .flatten()
        .chain(if class_name_prop.is_some() {
            vec![]
        } else {
            spread_props
                .iter()
                .map(|ex| {
                    convert_class_name(
                        ast_builder,
                        &Expression::StaticMemberExpression(
                            ast_builder.alloc_static_member_expression(
                                SPAN,
                                ex.clone_in(ast_builder.allocator),
                                ast_builder.identifier_name(SPAN, ast_builder.atom("className")),
                                true,
                            ),
                        ),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .as_slice(),
    )
}

pub fn get_style_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &Option<Expression<'a>>,
    styles: &[ExtractStyleProp<'a>],
    style_vars: &Option<Expression<'a>>,
    spread_props: &[Expression<'a>],
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    merge_object_expressions(
        ast_builder,
        [
            gen_styles(ast_builder, styles, filename),
            style_vars
                .as_ref()
                .map(|style_vars| convert_style_vars(ast_builder, style_vars)),
            style_prop.clone_in(ast_builder.allocator),
        ]
        .into_iter()
        .flatten()
        .chain(if style_prop.is_some() {
            vec![]
        } else {
            spread_props
                .iter()
                .map(|ex| {
                    Expression::StaticMemberExpression(ast_builder.alloc_static_member_expression(
                        SPAN,
                        ex.clone_in(ast_builder.allocator),
                        ast_builder.identifier_name(SPAN, ast_builder.atom("style")),
                        true,
                    ))
                })
                .collect::<Vec<_>>()
        })
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
    if expressions.len() == 1
        && !matches!(
            expressions.first().unwrap(),
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
        )
    {
        return Some(expressions.first().unwrap().clone_in(ast_builder.allocator));
    }

    let mut string_literals: std::vec::Vec<String> = vec![];
    let mut other_expressions = vec![];
    let mut prev_str = String::new();
    for ex in expressions.iter() {
        if let Expression::StringLiteral(literal) = ex {
            let target_prev = prev_str.trim();
            let target = literal.value.trim();
            prev_str = format!(
                "{}{}{}",
                target_prev,
                if target_prev.is_empty() { "" } else { " " },
                target
            );
        } else if let Expression::TemplateLiteral(template) = ex {
            for (idx, q) in template.quasis.iter().enumerate() {
                let target_prev = prev_str.trim();
                let target = q.value.raw.trim();
                if idx < template.quasis.len() - 1 {
                    string_literals.push(format!(
                        "{}{}{}{}{}",
                        if !other_expressions.is_empty() || idx > 0 {
                            " "
                        } else {
                            ""
                        },
                        target_prev,
                        if !target_prev.is_empty() { " " } else { "" },
                        target,
                        if !target.is_empty() && !target.ends_with("typo-") {
                            " "
                        } else {
                            ""
                        }
                    ));
                } else {
                    prev_str = q.value.raw.trim().to_string();
                }
            }
            other_expressions.extend(template.expressions.clone_in(ast_builder.allocator));
        } else {
            let target_prev = prev_str.trim();
            string_literals.push(format!(
                "{}{}{}",
                if !other_expressions.is_empty() {
                    " "
                } else {
                    ""
                },
                target_prev,
                if !target_prev.is_empty() { " " } else { "" }
            ));
            other_expressions.push(ex.clone_in(ast_builder.allocator));
            prev_str = String::new();
        }
    }
    string_literals.push(format!(
        "{}{}",
        if !prev_str.trim().is_empty() { " " } else { "" },
        prev_str.trim(),
    ));
    if other_expressions.is_empty() {
        return Some(ast_builder.expression_string_literal(
            SPAN,
            ast_builder.atom(string_literals.join("").trim()),
            None,
        ));
    }

    Some(
        ast_builder.expression_template_literal(
            SPAN,
            oxc_allocator::Vec::from_iter_in(
                string_literals.iter().enumerate().map(|(idx, s)| {
                    let tail = idx == string_literals.len() - 1;
                    ast_builder.template_element(
                        SPAN,
                        TemplateElementValue {
                            raw: ast_builder.atom(s),
                            cooked: None,
                        },
                        tail,
                    )
                }),
                ast_builder.allocator,
            ),
            oxc_allocator::Vec::from_iter_in(
                other_expressions
                    .into_iter()
                    .map(|ex| ex.clone_in(ast_builder.allocator)),
                ast_builder.allocator,
            ),
        ),
    )
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
    Some(ast_builder.expression_object(
        SPAN,
        oxc_allocator::Vec::from_iter_in(
            expressions.iter().map(|ex| {
                ast_builder
                    .object_property_kind_spread_property(SPAN, ex.clone_in(ast_builder.allocator))
            }),
            ast_builder.allocator,
        ),
    ))
}

pub fn convert_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name: &Expression<'a>,
) -> Expression<'a> {
    if matches!(
        class_name,
        Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::NumericLiteral(_)
    ) {
        return class_name.clone_in(ast_builder.allocator);
    }

    // wrap ( and ?? ''
    ast_builder.expression_logical(
        SPAN,
        ast_builder.expression_parenthesized(SPAN, class_name.clone_in(ast_builder.allocator)),
        LogicalOperator::Or,
        ast_builder.expression_string_literal(SPAN, "", None),
    )
}

pub fn convert_style_vars<'a>(
    ast_builder: &AstBuilder<'a>,
    style_vars: &Expression<'a>,
) -> Expression<'a> {
    let mut style_vars = style_vars.clone_in(ast_builder.allocator);
    if let Expression::ObjectExpression(obj) = &mut style_vars {
        for idx in (0..obj.properties.len()).rev() {
            let mut prop = obj.properties.remove(idx);

            if let ObjectPropertyKind::ObjectProperty(p) = &mut prop {
                let name = match &p.key {
                    PropertyKey::StaticIdentifier(ident) => Some(ident.name),
                    PropertyKey::StringLiteral(ident) => Some(ident.value),
                    etc => {
                        obj.properties.insert(
                            idx,
                            ast_builder.object_property_kind_object_property(
                                SPAN,
                                PropertyKind::Init,
                                PropertyKey::TemplateLiteral(ast_builder.alloc_template_literal(
                                    SPAN,
                                    oxc_allocator::Vec::from_array_in(
                                        [
                                            ast_builder.template_element(
                                                SPAN,
                                                TemplateElementValue {
                                                    raw: ast_builder.atom("--"),
                                                    cooked: None,
                                                },
                                                false,
                                            ),
                                            ast_builder.template_element(
                                                SPAN,
                                                TemplateElementValue {
                                                    raw: ast_builder.atom(""),
                                                    cooked: None,
                                                },
                                                true,
                                            ),
                                        ],
                                        ast_builder.allocator,
                                    ),
                                    oxc_allocator::Vec::from_array_in(
                                        [etc.to_expression().clone_in(ast_builder.allocator)],
                                        ast_builder.allocator,
                                    ),
                                )),
                                p.value.clone_in(ast_builder.allocator),
                                false,
                                false,
                                true,
                            ),
                        );
                        None
                    }
                };

                if let Some(name) = name {
                    if !name.starts_with("--") {
                        p.key = PropertyKey::StringLiteral(ast_builder.alloc_string_literal(
                            SPAN,
                            ast_builder.atom(&format!("--{name}")),
                            None,
                        ));
                    }
                    obj.properties.insert(idx, prop);
                }
            } else {
                obj.properties.insert(idx, prop);
            }
        }
    }
    style_vars
}
