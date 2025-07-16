use crate::{
    ExtractStyleProp,
    css_utils::css_to_style,
    extract_style::{
        extract_dynamic_style::ExtractDynamicStyle, extract_static_style::ExtractStaticStyle,
        extract_style_value::ExtractStyleValue,
    },
    extractor::{
        ExtractResult, extract_style_from_member_expression::extract_style_from_member_expression,
    },
    utils::{
        expression_to_code, get_number_by_literal_expression, get_string_by_literal_expression,
        is_same_expression, is_special_property,
    },
};
use css::style_selector::StyleSelector;
use oxc_allocator::CloneIn;
use oxc_ast::{
    AstBuilder,
    ast::{
        BinaryOperator, Expression, LogicalOperator, ObjectPropertyKind, PropertyKey,
        TemplateElementValue,
    },
};
use oxc_span::SPAN;

const IGNORED_IDENTIFIERS: [&str; 3] = ["undefined", "NaN", "Infinity"];

pub fn extract_style_from_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    name: Option<&str>,
    expression: &mut Expression<'a>,
    level: u8,
    selector: Option<&StyleSelector>,
) -> ExtractResult<'a> {
    let mut typo = false;

    if name.is_none() && selector.is_none() {
        let mut style_order = None;
        let mut style_vars = None;
        return match expression {
            Expression::ObjectExpression(obj) => {
                let mut props_styles: Vec<ExtractStyleProp<'_>> = vec![];
                let mut tag = None;
                for idx in (0..obj.properties.len()).rev() {
                    let mut prop = obj.properties.remove(idx);
                    if !match &mut prop {
                        ObjectPropertyKind::ObjectProperty(prop) => {
                            if let PropertyKey::StaticIdentifier(ident) = &prop.key
                                && let name = ident.name.as_str()
                                && !is_special_property(name)
                            {
                                if name == "styleOrder" {
                                    style_order = get_number_by_literal_expression(&prop.value)
                                        .map(|v| v as u8);
                                    continue;
                                }
                                if name == "styleVars" {
                                    style_vars = Some(prop.value.clone_in(ast_builder.allocator));
                                    continue;
                                }

                                let ExtractResult {
                                    styles, tag: _tag, ..
                                } = extract_style_from_expression(
                                    ast_builder,
                                    Some(name),
                                    &mut prop.value,
                                    0,
                                    None,
                                );
                                props_styles.extend(styles);
                                tag = _tag.or(tag);
                                true
                            } else {
                                false
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(prop) => {
                            let ExtractResult {
                                styles, tag: _tag, ..
                            } = extract_style_from_expression(
                                ast_builder,
                                None,
                                &mut prop.argument,
                                0,
                                None,
                            );
                            props_styles.extend(styles);
                            tag = _tag.or(tag);
                            false
                        }
                    } {
                        obj.properties.insert(idx, prop);
                    }
                }
                ExtractResult {
                    styles: props_styles,
                    tag,
                    style_order,
                    style_vars,
                }
            }
            Expression::ConditionalExpression(conditional) => ExtractResult {
                styles: vec![ExtractStyleProp::Conditional {
                    condition: conditional.test.clone_in(ast_builder.allocator),
                    consequent: Some(Box::new(ExtractStyleProp::StaticArray(
                        extract_style_from_expression(
                            ast_builder,
                            None,
                            &mut conditional.consequent,
                            level,
                            None,
                        )
                        .styles,
                    ))),
                    alternate: Some(Box::new(ExtractStyleProp::StaticArray(
                        extract_style_from_expression(
                            ast_builder,
                            None,
                            &mut conditional.alternate,
                            level,
                            selector,
                        )
                        .styles,
                    ))),
                }],
                tag: None,
                style_order,
                style_vars,
            },
            Expression::ParenthesizedExpression(parenthesized) => extract_style_from_expression(
                ast_builder,
                None,
                &mut parenthesized.expression,
                level,
                None,
            ),
            _ => ExtractResult::default(),
        };
    }

    if let Some(name) = name {
        if is_special_property(name) {
            return ExtractResult::default();
        }

        if name == "as" {
            return ExtractResult {
                tag: Some(expression.clone_in(ast_builder.allocator)),
                ..ExtractResult::default()
            };
        }
        if name == "selectors"
            && let Expression::ObjectExpression(obj) = expression
        {
            let mut props = vec![];
            for p in obj.properties.iter_mut() {
                if let ObjectPropertyKind::ObjectProperty(o) = p {
                    let name = o.key.name().unwrap().to_string();
                    props.extend(
                        extract_style_from_expression(
                            ast_builder,
                            None,
                            &mut o.value,
                            level,
                            Some(
                                &if let Some(selector) = selector {
                                    name.replace("&", &selector.to_string())
                                } else {
                                    name
                                }
                                .as_str()
                                .into(),
                            ),
                        )
                        .styles,
                    );
                }
            }
            return ExtractResult {
                styles: props,
                ..ExtractResult::default()
            };
        }

        if let Some(new_selector) = name.strip_prefix("_") {
            return extract_style_from_expression(
                ast_builder,
                None,
                expression,
                level,
                Some(&if let Some(selector) = selector {
                    (selector, new_selector).into()
                } else {
                    new_selector.into()
                }),
            );
        }
        typo = name == "typography";
    }
    if let Some(value) = get_string_by_literal_expression(expression) {
        if let Some(name) = name {
            ExtractResult {
                styles: vec![ExtractStyleProp::Static(if typo {
                    ExtractStyleValue::Typography(value.to_string())
                } else {
                    ExtractStyleValue::Static(ExtractStaticStyle::new(
                        name,
                        &value,
                        level,
                        selector.cloned(),
                    ))
                })],
                ..ExtractResult::default()
            }
        } else {
            ExtractResult {
                styles: css_to_style(&value, level, &selector),
                ..ExtractResult::default()
            }
        }
    } else {
        match expression {
            Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::StaticMemberExpression(_)
            | Expression::CallExpression(_) => ExtractResult {
                styles: vec![ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                    ExtractDynamicStyle::new(
                        name.unwrap(),
                        level,
                        &expression_to_code(expression),
                        selector.cloned(),
                    ),
                ))],
                ..ExtractResult::default()
            },
            Expression::TSAsExpression(exp) => extract_style_from_expression(
                ast_builder,
                name,
                &mut exp.expression,
                level,
                selector,
            ),
            Expression::ComputedMemberExpression(mem) => {
                extract_style_from_member_expression(ast_builder, name, mem, level, selector)
            }
            Expression::TemplateLiteral(tmp) => {
                if let Some(name) = name {
                    if tmp.quasis.len() == 1 {
                        ExtractResult {
                            styles: vec![ExtractStyleProp::Static(if typo {
                                ExtractStyleValue::Typography(tmp.quasis[0].value.raw.to_string())
                            } else {
                                ExtractStyleValue::Static(ExtractStaticStyle::new(
                                    name,
                                    &tmp.quasis[0].value.raw,
                                    level,
                                    selector.cloned(),
                                ))
                            })],
                            ..ExtractResult::default()
                        }
                    } else if typo {
                        ExtractResult {
                            styles: vec![ExtractStyleProp::Expression {
                                expression: ast_builder.expression_template_literal(
                                    SPAN,
                                    ast_builder.vec_from_array([
                                        ast_builder.template_element(
                                            SPAN,
                                            TemplateElementValue {
                                                raw: ast_builder.atom("typo-"),
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
                                    ]),
                                    ast_builder.vec_from_array([
                                        expression.clone_in(ast_builder.allocator)
                                    ]),
                                ),
                                styles: vec![],
                            }],
                            ..ExtractResult::default()
                        }
                    } else {
                        ExtractResult {
                            styles: vec![ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                                ExtractDynamicStyle::new(
                                    name,
                                    level,
                                    &expression_to_code(expression),
                                    selector.cloned(),
                                ),
                            ))],
                            ..ExtractResult::default()
                        }
                    }
                } else {
                    ExtractResult::default()
                }
            }
            Expression::Identifier(identifier) => {
                if IGNORED_IDENTIFIERS.contains(&identifier.name.as_str()) {
                    ExtractResult::default()
                } else if let Some(name) = name {
                    if typo {
                        ExtractResult {
                            styles: vec![ExtractStyleProp::Expression {
                                expression: ast_builder.expression_conditional(
                                    SPAN,
                                    ast_builder
                                        .expression_identifier(SPAN, identifier.name.as_str()),
                                    ast_builder.expression_template_literal(
                                        SPAN,
                                        ast_builder.vec_from_array([
                                            ast_builder.template_element(
                                                SPAN,
                                                TemplateElementValue {
                                                    raw: ast_builder.atom("typo-"),
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
                                        ]),
                                        ast_builder.vec_from_array([
                                            expression.clone_in(ast_builder.allocator)
                                        ]),
                                    ),
                                    ast_builder.expression_string_literal(SPAN, "", None),
                                ),
                                styles: vec![],
                            }],
                            ..ExtractResult::default()
                        }
                    } else {
                        ExtractResult {
                            styles: vec![ExtractStyleProp::Static(ExtractStyleValue::Dynamic(
                                ExtractDynamicStyle::new(
                                    name,
                                    level,
                                    &identifier.name,
                                    selector.cloned(),
                                ),
                            ))],
                            ..ExtractResult::default()
                        }
                    }
                } else {
                    ExtractResult::default()
                }
            }
            Expression::LogicalExpression(logical) => {
                let res = Some(Box::new(ExtractStyleProp::StaticArray(
                    extract_style_from_expression(
                        ast_builder,
                        name,
                        &mut logical.right,
                        level,
                        selector,
                    )
                    .styles,
                )));
                match logical.operator {
                    LogicalOperator::Or => ExtractResult {
                        styles: vec![ExtractStyleProp::Conditional {
                            condition: logical.left.clone_in(ast_builder.allocator),
                            consequent: None,
                            alternate: res,
                        }],
                        ..ExtractResult::default()
                    },
                    LogicalOperator::And => ExtractResult {
                        styles: vec![ExtractStyleProp::Conditional {
                            condition: logical.left.clone_in(ast_builder.allocator),
                            consequent: res,
                            alternate: None,
                        }],
                        ..ExtractResult::default()
                    },
                    LogicalOperator::Coalesce => ExtractResult {
                        styles: vec![ExtractStyleProp::Conditional {
                            condition: ast_builder.expression_logical(
                                SPAN,
                                ast_builder.expression_binary(
                                    SPAN,
                                    logical.left.clone_in(ast_builder.allocator),
                                    BinaryOperator::StrictInequality,
                                    ast_builder.expression_null_literal(SPAN),
                                ),
                                LogicalOperator::And,
                                ast_builder.expression_binary(
                                    SPAN,
                                    logical.left.clone_in(ast_builder.allocator),
                                    BinaryOperator::StrictInequality,
                                    ast_builder.expression_identifier(SPAN, "undefined"),
                                ),
                            ),
                            consequent: Some(Box::new(ExtractStyleProp::StaticArray(
                                extract_style_from_expression(
                                    ast_builder,
                                    name,
                                    &mut logical.left,
                                    level,
                                    selector,
                                )
                                .styles,
                            ))),
                            alternate: res,
                        }],
                        ..ExtractResult::default()
                    },
                }
            }
            Expression::ParenthesizedExpression(parenthesized) => extract_style_from_expression(
                ast_builder,
                name,
                &mut parenthesized.expression,
                level,
                selector,
            ),
            Expression::ArrayExpression(array) => {
                let mut props = vec![];

                for (idx, element) in array.elements.iter_mut().enumerate() {
                    props.extend(
                        extract_style_from_expression(
                            ast_builder,
                            name,
                            element.to_expression_mut(),
                            idx as u8,
                            selector,
                        )
                        .styles,
                    );
                }
                ExtractResult {
                    styles: vec![ExtractStyleProp::StaticArray(props)],
                    tag: None,
                    style_order: None,
                    style_vars: None,
                }
            }
            Expression::ConditionalExpression(conditional) => {
                if is_same_expression(&conditional.consequent, &conditional.alternate) {
                    extract_style_from_expression(
                        ast_builder,
                        name,
                        &mut conditional.consequent,
                        level,
                        selector,
                    )
                } else {
                    ExtractResult {
                        styles: vec![ExtractStyleProp::Conditional {
                            condition: conditional.test.clone_in(ast_builder.allocator),
                            consequent: Some(Box::new(ExtractStyleProp::StaticArray(
                                extract_style_from_expression(
                                    ast_builder,
                                    name,
                                    &mut conditional.consequent,
                                    level,
                                    selector,
                                )
                                .styles,
                            ))),
                            alternate: Some(Box::new(ExtractStyleProp::StaticArray(
                                extract_style_from_expression(
                                    ast_builder,
                                    name,
                                    &mut conditional.alternate,
                                    level,
                                    selector,
                                )
                                .styles,
                            ))),
                        }],
                        ..ExtractResult::default()
                    }
                }
            }
            Expression::ObjectExpression(obj) => {
                let mut props = vec![];
                for p in obj.properties.iter_mut() {
                    if let ObjectPropertyKind::ObjectProperty(o) = p {
                        props.extend(
                            extract_style_from_expression(
                                ast_builder,
                                Some(&o.key.name().unwrap()),
                                &mut o.value,
                                level,
                                selector,
                            )
                            .styles,
                        );
                    }
                }
                ExtractResult {
                    styles: props,
                    ..ExtractResult::default()
                }
            }
            _ => ExtractResult::default(),
        }
    }
}
