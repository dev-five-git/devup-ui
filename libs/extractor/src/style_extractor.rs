use crate::utils::{expression_to_code, get_number_by_literal_expression, is_special_property};
use crate::{ExtractStyleProp, utils};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{
    ArrayExpressionElement, ComputedMemberExpression, Expression, JSXAttributeValue,
    ObjectPropertyKind, PropertyKey, TemplateElementValue,
};
use std::collections::BTreeMap;

use crate::extract_style::ExtractStyleValue::{Dynamic, Static, Typography};
use crate::extract_style::{ExtractDynamicStyle, ExtractStaticStyle};
use css::StyleSelector;
use oxc_ast::AstBuilder;
use oxc_span::SPAN;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};

const IGNORED_IDENTIFIERS: [&str; 3] = ["undefined", "NaN", "Infinity"];

/**
 * type
 * 1. jsx -> <Extract a={1} />
 * 2. object -> createElement('div', {a: 1})
 * 3. object with select -> createElement('div', {a: 1})
 */

#[derive(Debug)]
pub enum ExtractResult<'a> {
    // attribute will be maintained
    Maintain,
    Extract {
        styles: Option<Vec<ExtractStyleProp<'a>>>,
        tag: Option<Expression<'a>>,
        style_order: Option<u8>,
        style_vars: Option<Expression<'a>>,
    },
}

pub fn extract_style_from_jsx_attr<'a>(
    ast_builder: &AstBuilder<'a>,
    name: &str,
    value: &mut JSXAttributeValue<'a>,
    selector: Option<&StyleSelector>,
) -> ExtractResult<'a> {
    match value {
        JSXAttributeValue::ExpressionContainer(expression) => {
            if expression.expression.is_expression() {
                extract_style_from_expression(
                    ast_builder,
                    Some(name),
                    expression.expression.to_expression_mut(),
                    0,
                    selector,
                )
            } else {
                ExtractResult::Maintain
            }
        }
        JSXAttributeValue::StringLiteral(literal) => extract_style_from_expression(
            ast_builder,
            Some(name),
            &mut Expression::StringLiteral(literal.clone_in(ast_builder.allocator)),
            0,
            selector,
        ),
        _ => ExtractResult::Maintain,
    }
}

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
                            if let PropertyKey::StaticIdentifier(ident) = &prop.key {
                                let name = ident.name.as_str();

                                if name == "styleOrder" {
                                    style_order = get_number_by_literal_expression(&prop.value)
                                        .map(|v| v as u8);
                                    continue;
                                }
                                if name == "styleVars" {
                                    style_vars = Some(prop.value.clone_in(ast_builder.allocator));
                                    continue;
                                }

                                match extract_style_from_expression(
                                    ast_builder,
                                    Some(name),
                                    &mut prop.value,
                                    0,
                                    None,
                                ) {
                                    ExtractResult::Maintain => false,
                                    ExtractResult::Extract {
                                        styles, tag: _tag, ..
                                    } => {
                                        styles.into_iter().for_each(|mut styles| {
                                            props_styles.append(&mut styles)
                                        });
                                        tag = _tag.or(tag);
                                        true
                                    }
                                }
                            } else {
                                false
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(prop) => {
                            match extract_style_from_expression(
                                ast_builder,
                                None,
                                &mut prop.argument,
                                0,
                                None,
                            ) {
                                ExtractResult::Maintain => false,
                                ExtractResult::Extract {
                                    styles, tag: _tag, ..
                                } => {
                                    styles
                                        .into_iter()
                                        .for_each(|mut styles| props_styles.append(&mut styles));
                                    tag = _tag.or(tag);
                                    true
                                }
                            }
                        }
                    } {
                        obj.properties.insert(idx, prop);
                    }
                }
                if props_styles.is_empty() && style_vars.is_none() {
                    ExtractResult::Maintain
                } else {
                    ExtractResult::Extract {
                        styles: Some(props_styles),
                        tag,
                        style_order,
                        style_vars,
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => ExtractResult::Extract {
                styles: Some(vec![ExtractStyleProp::Conditional {
                    condition: conditional.test.clone_in(ast_builder.allocator),
                    consequent: if let ExtractResult::Extract {
                        styles: Some(styles),
                        ..
                    } = extract_style_from_expression(
                        ast_builder,
                        None,
                        &mut conditional.consequent,
                        level,
                        None,
                    ) {
                        Some(Box::new(ExtractStyleProp::StaticArray(styles)))
                    } else {
                        None
                    },
                    alternate: if let ExtractResult::Extract {
                        styles: Some(styles),
                        ..
                    } = extract_style_from_expression(
                        ast_builder,
                        None,
                        &mut conditional.alternate,
                        level,
                        selector,
                    ) {
                        Some(Box::new(ExtractStyleProp::StaticArray(styles)))
                    } else {
                        None
                    },
                }]),
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
            _ => ExtractResult::Maintain,
        };
    }

    if let Some(name) = name {
        if is_special_property(name) {
            return ExtractResult::Maintain;
        }

        if name == "as" {
            return ExtractResult::Extract {
                styles: None,
                tag: Some(expression.clone_in(ast_builder.allocator)),
                style_order: None,
                style_vars: None,
            };

            // return match expression {
            //     Expression::StringLiteral(ident) => ExtractResult::ChangeTag(
            //         Expression::StringLiteral(ident.clone_in(ast_builder.allocator)),
            //     ),
            //     Expression::TemplateLiteral(tmp) => {
            //         if tmp.quasis.len() == 1 {
            //             ExtractResult::ChangeTag(Expression::TemplateLiteral(
            //                 tmp.clone_in(ast_builder.allocator),
            //             ))
            //         } else {
            //             ExtractResult::Remove
            //         }
            //     }
            //     Expression::ConditionalExpression(ref mut conditional) => {
            //         let mut consequent = None;
            //         let mut alternate = None;
            //         if let ExtractResult::ExtractStyle(mut styles) = extract_style_from_expression(
            //             ast_builder,
            //             None,
            //             &mut conditional.consequent,
            //             level,
            //             None,
            //         ) {
            //             consequent = Some(Box::new(styles.remove(0)));
            //         }
            //         if let ExtractResult::ExtractStyle(mut styles) = extract_style_from_expression(
            //             ast_builder,
            //             None,
            //             &mut conditional.alternate,
            //             level,
            //             selector,
            //         ) {
            //             alternate = Some(Box::new(styles.remove(0)));
            //         }
            //         ExtractResult::ChangeTag(
            //         )
            //     }
            //     _ => ExtractResult::Remove,
            // };
        }
        if name == "selectors"
            && let Expression::ObjectExpression(obj) = expression
        {
            let mut props = vec![];
            for p in obj.properties.iter_mut() {
                if let ObjectPropertyKind::ObjectProperty(o) = p {
                    let name = o.key.name().unwrap().to_string();
                    if let ExtractResult::Extract {
                        styles: Some(mut styles),
                        ..
                    } = extract_style_from_expression(
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
                    ) {
                        props.append(&mut styles);
                    }
                }
            }
            return ExtractResult::Extract {
                styles: Some(props),
                tag: None,
                style_order: None,
                style_vars: None,
            };
        }

        if let Some(new_selector) = name.strip_prefix("_") {
            return extract_style_from_expression(
                ast_builder,
                None,
                expression,
                level,
                Some(&if let Some(selector) = selector {
                    [&selector.to_string(), new_selector].into()
                } else {
                    new_selector.into()
                }),
            );
        }
        typo = name == "typography";
    }
    if let Some(value) = utils::get_string_by_literal_expression(expression) {
        name.map(|name| ExtractResult::Extract {
            style_order: None,
            style_vars: None,
            tag: None,
            styles: Some(vec![ExtractStyleProp::Static(if typo {
                Typography(value.to_string())
            } else {
                Static(ExtractStaticStyle::new(
                    name,
                    &value,
                    level,
                    selector.cloned(),
                ))
            })]),
        })
        .unwrap_or(ExtractResult::Maintain)
    } else {
        match expression {
            Expression::UnaryExpression(_)
            | Expression::BinaryExpression(_)
            | Expression::StaticMemberExpression(_)
            | Expression::CallExpression(_) => ExtractResult::Extract {
                styles: Some(vec![ExtractStyleProp::Static(Dynamic(
                    ExtractDynamicStyle::new(
                        name.unwrap(),
                        level,
                        &expression_to_code(expression),
                        selector.cloned(),
                    ),
                ))]),
                tag: None,
                style_order: None,
                style_vars: None,
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
                        ExtractResult::Extract {
                            styles: Some(vec![ExtractStyleProp::Static(if typo {
                                Typography(tmp.quasis[0].value.raw.to_string())
                            } else {
                                Static(ExtractStaticStyle::new(
                                    name,
                                    &tmp.quasis[0].value.raw,
                                    level,
                                    selector.cloned(),
                                ))
                            })]),
                            tag: None,
                            style_order: None,
                            style_vars: None,
                        }
                    } else if typo {
                        ExtractResult::Extract {
                            styles: Some(vec![ExtractStyleProp::Expression {
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
                            }]),
                            tag: None,
                            style_order: None,
                            style_vars: None,
                        }
                    } else {
                        ExtractResult::Extract {
                            styles: Some(vec![ExtractStyleProp::Static(Dynamic(
                                ExtractDynamicStyle::new(
                                    name,
                                    level,
                                    &expression_to_code(expression),
                                    selector.cloned(),
                                ),
                            ))]),
                            tag: None,
                            style_order: None,
                            style_vars: None,
                        }
                    }
                } else {
                    ExtractResult::Maintain
                }
            }
            Expression::Identifier(identifier) => {
                if IGNORED_IDENTIFIERS.contains(&identifier.name.as_str()) {
                    ExtractResult::Maintain
                } else if let Some(name) = name {
                    if typo {
                        ExtractResult::Extract {
                            styles: Some(vec![ExtractStyleProp::Expression {
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
                            }]),
                            tag: None,
                            style_order: None,
                            style_vars: None,
                        }
                    } else {
                        ExtractResult::Extract {
                            styles: Some(vec![ExtractStyleProp::Static(Dynamic(
                                ExtractDynamicStyle::new(
                                    name,
                                    level,
                                    &identifier.name,
                                    selector.cloned(),
                                ),
                            ))]),
                            tag: None,
                            style_order: None,
                            style_vars: None,
                        }
                    }
                } else {
                    ExtractResult::Maintain
                }
            }
            Expression::LogicalExpression(logical) => {
                let res = match extract_style_from_expression(
                    ast_builder,
                    name,
                    &mut logical.right,
                    level,
                    selector,
                ) {
                    ExtractResult::Extract {
                        styles: Some(styles),
                        ..
                    } => Some(Box::new(ExtractStyleProp::StaticArray(styles))),
                    _ => None,
                };
                match logical.operator {
                    LogicalOperator::Or => ExtractResult::Extract {
                        styles: Some(vec![ExtractStyleProp::Conditional {
                            condition: logical.left.clone_in(ast_builder.allocator),
                            consequent: None,
                            alternate: res,
                        }]),
                        tag: None,
                        style_order: None,
                        style_vars: None,
                    },
                    LogicalOperator::And => ExtractResult::Extract {
                        styles: Some(vec![ExtractStyleProp::Conditional {
                            condition: logical.left.clone_in(ast_builder.allocator),
                            consequent: res,
                            alternate: None,
                        }]),
                        tag: None,
                        style_order: None,
                        style_vars: None,
                    },
                    LogicalOperator::Coalesce => ExtractResult::Extract {
                        styles: Some(vec![ExtractStyleProp::Conditional {
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
                            consequent: None,
                            alternate: res,
                        }]),
                        tag: None,
                        style_order: None,
                        style_vars: None,
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
                    if let ExtractResult::Extract {
                        styles: Some(mut styles),
                        ..
                    } = extract_style_from_expression(
                        ast_builder,
                        name,
                        element.to_expression_mut(),
                        idx as u8,
                        selector,
                    ) {
                        props.append(&mut styles);
                    }
                }

                if props.is_empty() {
                    ExtractResult::Maintain
                } else {
                    ExtractResult::Extract {
                        styles: Some(vec![ExtractStyleProp::StaticArray(props)]),
                        tag: None,
                        style_order: None,
                        style_vars: None,
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => ExtractResult::Extract {
                styles: Some(vec![ExtractStyleProp::Conditional {
                    condition: conditional.test.clone_in(ast_builder.allocator),
                    consequent: if let ExtractResult::Extract {
                        styles: Some(styles),
                        ..
                    } = extract_style_from_expression(
                        ast_builder,
                        name,
                        &mut conditional.consequent,
                        level,
                        selector,
                    ) {
                        Some(Box::new(ExtractStyleProp::StaticArray(styles)))
                    } else {
                        None
                    },
                    alternate: if let ExtractResult::Extract {
                        styles: Some(styles),
                        ..
                    } = extract_style_from_expression(
                        ast_builder,
                        name,
                        &mut conditional.alternate,
                        level,
                        selector,
                    ) {
                        Some(Box::new(ExtractStyleProp::StaticArray(styles)))
                    } else {
                        None
                    },
                }]),
                tag: None,
                style_order: None,
                style_vars: None,
            },
            Expression::ObjectExpression(obj) => {
                let mut props = vec![];
                for p in obj.properties.iter_mut() {
                    if let ObjectPropertyKind::ObjectProperty(o) = p {
                        if let ExtractResult::Extract {
                            styles: Some(mut styles),
                            ..
                        } = extract_style_from_expression(
                            ast_builder,
                            Some(&o.key.name().unwrap()),
                            &mut o.value,
                            level,
                            selector,
                        ) {
                            props.append(&mut styles);
                        }
                    };
                }
                ExtractResult::Extract {
                    styles: Some(props),
                    tag: None,
                    style_order: None,
                    style_vars: None,
                }
            }
            // val if let Some(value) = get_number_by_literal_expression(val) => {}
            _ => ExtractResult::Maintain,
        }
    }
}

fn extract_style_from_member_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    name: Option<&str>,
    mem: &mut ComputedMemberExpression<'a>,
    level: u8,
    selector: Option<&StyleSelector>,
) -> ExtractResult<'a> {
    let mem_expression = &mem.expression.clone_in(ast_builder.allocator);
    let mut ret: Vec<ExtractStyleProp> = vec![];

    match &mut mem.object {
        Expression::ArrayExpression(array) => {
            if array.elements.is_empty() {
                return ExtractResult::Extract {
                    styles: None,
                    tag: None,
                    style_order: None,
                    style_vars: None,
                };
            }

            if let Some(num) = utils::get_number_by_literal_expression(mem_expression) {
                if num < 0f64 {
                    return ExtractResult::Extract {
                        styles: None,
                        tag: None,
                        style_order: None,
                        style_vars: None,
                    };
                }
                let mut etc = None;
                for (idx, p) in array.elements.iter_mut().enumerate() {
                    if let ArrayExpressionElement::SpreadElement(sp) = p {
                        etc = Some(sp.argument.clone_in(ast_builder.allocator));
                        continue;
                    }
                    if idx as f64 == num {
                        if let ExtractResult::Extract {
                            styles: Some(styles),
                            ..
                        } = extract_style_from_expression(
                            ast_builder,
                            name,
                            p.to_expression_mut(),
                            level,
                            selector,
                        ) {
                            return ExtractResult::Extract {
                                styles: Some(styles),
                                tag: None,
                                style_order: None,
                                style_vars: None,
                            };
                        }
                    }
                }
                return ExtractResult::Extract {
                    styles: etc.map(|etc| {
                        vec![ExtractStyleProp::Static(Dynamic(ExtractDynamicStyle::new(
                            name.unwrap(),
                            level,
                            &expression_to_code(&Expression::ComputedMemberExpression(
                                ast_builder.alloc_computed_member_expression(
                                    SPAN,
                                    etc,
                                    mem_expression.clone_in(ast_builder.allocator),
                                    false,
                                ),
                            )),
                            selector.cloned(),
                        )))]
                    }),
                    tag: None,
                    style_order: None,
                    style_vars: None,
                };
            }

            let mut map = BTreeMap::new();
            for (idx, p) in array.elements.iter_mut().enumerate() {
                if let ArrayExpressionElement::SpreadElement(sp) = p {
                    map.insert(
                        idx.to_string(),
                        Box::new(ExtractStyleProp::Static(Dynamic(ExtractDynamicStyle::new(
                            name.unwrap(),
                            level,
                            &expression_to_code(&Expression::ComputedMemberExpression(
                                ast_builder.alloc_computed_member_expression(
                                    SPAN,
                                    sp.argument.clone_in(ast_builder.allocator),
                                    mem_expression.clone_in(ast_builder.allocator),
                                    false,
                                ),
                            )),
                            selector.cloned(),
                        )))),
                    );
                } else if let ExtractResult::Extract {
                    styles: Some(styles),
                    ..
                } = extract_style_from_expression(
                    ast_builder,
                    name,
                    p.to_expression_mut(),
                    level,
                    selector,
                ) {
                    map.insert(
                        idx.to_string(),
                        Box::new(ExtractStyleProp::StaticArray(styles)),
                    );
                }
            }

            ret.push(ExtractStyleProp::MemberExpression {
                expression: mem_expression.clone_in(ast_builder.allocator),
                map,
            });
        }
        Expression::ObjectExpression(obj) => {
            if obj.properties.is_empty() {
                return ExtractResult::Extract {
                    styles: None,
                    tag: None,
                    style_order: None,
                    style_vars: None,
                };
            }

            let mut map = BTreeMap::new();
            if let Some(k) = utils::get_string_by_literal_expression(mem_expression) {
                let mut etc = None;
                for p in obj.properties.iter_mut() {
                    if let ObjectPropertyKind::ObjectProperty(o) = p {
                        if let PropertyKey::StaticIdentifier(ref pk) = o.key {
                            if pk.name == k {
                                if let ExtractResult::Extract {
                                    styles: Some(styles),
                                    ..
                                } = extract_style_from_expression(
                                    ast_builder,
                                    name,
                                    &mut o.value,
                                    level,
                                    selector,
                                ) {
                                    return ExtractResult::Extract {
                                        styles: Some(styles),
                                        tag: None,
                                        style_order: None,
                                        style_vars: None,
                                    };
                                }
                            }
                        }
                    } else if let ObjectPropertyKind::SpreadProperty(sp) = p {
                        etc = Some(sp.argument.clone_in(ast_builder.allocator));
                    }
                }

                match etc {
                    None => {
                        return ExtractResult::Extract {
                            styles: None,
                            tag: None,
                            style_order: None,
                            style_vars: None,
                        };
                    }
                    Some(etc) => {
                        ret.push(ExtractStyleProp::Static(Dynamic(ExtractDynamicStyle::new(
                            name.unwrap(),
                            level,
                            &expression_to_code(&Expression::ComputedMemberExpression(
                                ast_builder.alloc_computed_member_expression(
                                    SPAN,
                                    etc,
                                    mem_expression.clone_in(ast_builder.allocator),
                                    false,
                                ),
                            )),
                            selector.cloned(),
                        ))))
                    }
                }
            }

            for p in obj.properties.iter_mut() {
                if let ObjectPropertyKind::ObjectProperty(o) = p {
                    if let PropertyKey::StaticIdentifier(_)
                    | PropertyKey::NumericLiteral(_)
                    | PropertyKey::StringLiteral(_) = o.key
                    {
                        if let ExtractResult::Extract {
                            styles: Some(styles),
                            ..
                        } = extract_style_from_expression(
                            ast_builder,
                            name,
                            &mut o.value,
                            level,
                            selector,
                        ) {
                            map.insert(
                                o.key.name().unwrap().to_string(),
                                Box::new(ExtractStyleProp::StaticArray(styles)),
                            );
                        }
                    }
                }
            }
            ret.push(ExtractStyleProp::MemberExpression {
                expression: mem_expression.clone_in(ast_builder.allocator),
                map,
            });
        }
        Expression::Identifier(_) => {
            if let Some(name) = name {
                ret.push(ExtractStyleProp::Static(Dynamic(ExtractDynamicStyle::new(
                    name,
                    level,
                    &expression_to_code(&Expression::ComputedMemberExpression(
                        ast_builder.alloc_computed_member_expression(
                            SPAN,
                            mem.object.clone_in(ast_builder.allocator),
                            mem_expression.clone_in(ast_builder.allocator),
                            false,
                        ),
                    )),
                    selector.cloned(),
                ))))
            }
        }
        _ => {}
    };

    ExtractResult::Extract {
        styles: Some(ret),
        tag: None,
        style_order: None,
        style_vars: None,
    }
}
