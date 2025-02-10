use crate::utils::{expression_to_code, is_special_property};
use crate::ExtractStyleProp;
use oxc_allocator::CloneIn;
use oxc_ast::ast::{
    ArrayExpressionElement, ComputedMemberExpression, Expression, JSXAttributeValue,
    ObjectPropertyKind, PropertyKey, TemplateElementValue,
};
use std::collections::BTreeMap;

use crate::extract_style::ExtractStyleValue::{Dynamic, Static, Typography};
use crate::extract_style::{ExtractDynamicStyle, ExtractStaticStyle};
use oxc_ast::AstBuilder;
use oxc_span::SPAN;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

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
    // attribute will be removed
    Remove,
    // attribute will be removed and the value will be extracted
    ExtractStyle(Vec<ExtractStyleProp<'a>>),
    // attribute will be removed and the tag will be changed
    ChangeTag(Expression<'a>),

    ExtractStyleWithChangeTag(Vec<ExtractStyleProp<'a>>, Expression<'a>),
}

pub fn extract_style_from_jsx_attr<'a>(
    ast_builder: &AstBuilder<'a>,
    name: &str,
    value: &mut JSXAttributeValue<'a>,
    selector: Option<&str>,
) -> ExtractResult<'a> {
    match value {
        JSXAttributeValue::ExpressionContainer(ref mut expression) => {
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
    selector: Option<&str>,
) -> ExtractResult<'a> {
    let mut typo = false;

    if name.is_none() && selector.is_none() {
        return match expression {
            Expression::ObjectExpression(ref mut obj) => {
                let mut props_styles = vec![];
                let mut tag = None;
                for idx in (0..obj.properties.len()).rev() {
                    let mut prop = obj.properties.remove(idx);
                    let mut rm = false;
                    match &mut prop {
                        ObjectPropertyKind::ObjectProperty(prop) => {
                            if let PropertyKey::StaticIdentifier(ident) = &prop.key {
                                let name = ident.name.to_string();
                                rm = match extract_style_from_expression(
                                    ast_builder,
                                    Some(&name),
                                    &mut prop.value,
                                    0,
                                    None,
                                ) {
                                    ExtractResult::Maintain => false,
                                    ExtractResult::Remove => true,
                                    ExtractResult::ExtractStyle(mut styles) => {
                                        props_styles.append(&mut styles);
                                        true
                                    }
                                    ExtractResult::ChangeTag(t) => {
                                        tag = Some(t);
                                        true
                                    }
                                    ExtractResult::ExtractStyleWithChangeTag(mut styles, t) => {
                                        props_styles.append(&mut styles);
                                        tag = Some(t);
                                        true
                                    }
                                }
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(prop) => {
                            rm = match extract_style_from_expression(
                                ast_builder,
                                None,
                                &mut prop.argument,
                                0,
                                None,
                            ) {
                                ExtractResult::Maintain => false,
                                ExtractResult::Remove => true,
                                ExtractResult::ExtractStyle(mut styles) => {
                                    props_styles.append(&mut styles);
                                    true
                                }
                                ExtractResult::ChangeTag(t) => {
                                    tag = Some(t);
                                    true
                                }
                                ExtractResult::ExtractStyleWithChangeTag(mut styles, t) => {
                                    props_styles.append(&mut styles);
                                    tag = Some(t);
                                    true
                                }
                            };
                        }
                    }
                    if !rm {
                        obj.properties.insert(idx, prop);
                    }
                }
                if props_styles.is_empty() {
                    ExtractResult::Maintain
                } else if let Some(tag) = tag {
                    ExtractResult::ExtractStyleWithChangeTag(props_styles, tag)
                } else {
                    ExtractResult::ExtractStyle(props_styles)
                }
            }
            Expression::ConditionalExpression(ref mut conditional) => {
                ExtractResult::ExtractStyle(vec![ExtractStyleProp::Conditional {
                    condition: conditional.test.clone_in(ast_builder.allocator),
                    consequent: if let ExtractResult::ExtractStyle(styles) =
                        extract_style_from_expression(
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
                    alternate: if let ExtractResult::ExtractStyle(styles) =
                        extract_style_from_expression(
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
                }])
            }
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
            return ExtractResult::ChangeTag(expression.clone_in(ast_builder.allocator));

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

        if let Some(new_selector) = name.strip_prefix("_") {
            return extract_style_from_expression(
                ast_builder,
                None,
                expression,
                level,
                Some(
                    if let Some(selector) = selector {
                        format!("{}:{}", selector, new_selector)
                    } else {
                        new_selector.to_string()
                    }
                    .as_str(),
                ),
            );
        }
        typo = name == "typography";
    }
    if let Some(value) = get_string_by_literal_expression(expression) {
        name.map(|name| {
            ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(if typo {
                Typography(value.as_str().to_string())
            } else {
                Static(ExtractStaticStyle::new(
                    name,
                    value.as_str(),
                    level,
                    selector.map(|s| s.into()),
                ))
            })])
        })
        .unwrap_or(ExtractResult::Maintain)
    } else {
        match expression {
            Expression::UnaryExpression(_) | Expression::BinaryExpression(_) => {
                ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(Dynamic(
                    ExtractDynamicStyle::new(
                        name.unwrap(),
                        level,
                        expression_to_code(expression).as_str(),
                        selector.map(|s| s.into()),
                    ),
                ))])
            }
            Expression::ComputedMemberExpression(mem) => {
                extract_style_from_member_expression(ast_builder, name, mem, level, selector)
            }
            Expression::TemplateLiteral(tmp) => {
                if let Some(name) = name {
                    if tmp.quasis.len() == 1 {
                        ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(if typo {
                            Typography(tmp.quasis[0].value.raw.as_str().to_string())
                        } else {
                            Static(ExtractStaticStyle::new(
                                name,
                                tmp.quasis[0].value.raw.as_str(),
                                level,
                                selector.map(|s| s.into()),
                            ))
                        })])
                    } else if typo {
                        ExtractResult::ExtractStyle(vec![ExtractStyleProp::Expression {
                            expression: ast_builder.expression_template_literal(
                                SPAN,
                                ast_builder.vec_from_array([
                                    ast_builder.template_element(
                                        SPAN,
                                        false,
                                        TemplateElementValue {
                                            raw: ast_builder.atom("typo-"),
                                            cooked: None,
                                        },
                                    ),
                                    ast_builder.template_element(
                                        SPAN,
                                        true,
                                        TemplateElementValue {
                                            raw: ast_builder.atom(""),
                                            cooked: None,
                                        },
                                    ),
                                ]),
                                ast_builder
                                    .vec_from_array([expression.clone_in(ast_builder.allocator)]),
                            ),
                            styles: vec![],
                        }])
                    } else {
                        ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(Dynamic(
                            ExtractDynamicStyle::new(
                                name,
                                level,
                                expression_to_code(expression).as_str(),
                                selector.map(|s| s.into()),
                            ),
                        ))])
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
                        ExtractResult::ExtractStyle(vec![ExtractStyleProp::Expression {
                            expression: ast_builder.expression_template_literal(
                                SPAN,
                                ast_builder.vec_from_array([
                                    ast_builder.template_element(
                                        SPAN,
                                        false,
                                        TemplateElementValue {
                                            raw: ast_builder.atom("typo-"),
                                            cooked: None,
                                        },
                                    ),
                                    ast_builder.template_element(
                                        SPAN,
                                        true,
                                        TemplateElementValue {
                                            raw: ast_builder.atom(""),
                                            cooked: None,
                                        },
                                    ),
                                ]),
                                ast_builder
                                    .vec_from_array([expression.clone_in(ast_builder.allocator)]),
                            ),
                            styles: vec![],
                        }])
                    } else {
                        ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(Dynamic(
                            ExtractDynamicStyle::new(
                                name,
                                level,
                                identifier.name.as_str(),
                                selector.map(|s| s.into()),
                            ),
                        ))])
                    }
                } else {
                    ExtractResult::Maintain
                }
            }
            Expression::LogicalExpression(logical) => {
                let res = name.and_then(|name| {
                    match extract_style_from_expression(
                        ast_builder,
                        Some(name),
                        &mut logical.right,
                        level,
                        selector,
                    ) {
                        ExtractResult::ExtractStyle(styles) => {
                            Some(Box::new(ExtractStyleProp::StaticArray(styles)))
                        }
                        _ => None,
                    }
                });
                match logical.operator {
                    LogicalOperator::Or => {
                        ExtractResult::ExtractStyle(vec![ExtractStyleProp::Conditional {
                            condition: logical.left.clone_in(ast_builder.allocator),
                            consequent: None,
                            alternate: res,
                        }])
                    }
                    LogicalOperator::And => {
                        ExtractResult::ExtractStyle(vec![ExtractStyleProp::Conditional {
                            condition: logical.left.clone_in(ast_builder.allocator),
                            consequent: res,
                            alternate: None,
                        }])
                    }
                    LogicalOperator::Coalesce => {
                        ExtractResult::ExtractStyle(vec![ExtractStyleProp::Conditional {
                            condition: Expression::LogicalExpression(
                                ast_builder.alloc_logical_expression(
                                    SPAN,
                                    Expression::BinaryExpression(
                                        ast_builder.alloc_binary_expression(
                                            SPAN,
                                            logical.left.clone_in(ast_builder.allocator),
                                            BinaryOperator::StrictInequality,
                                            Expression::NullLiteral(
                                                ast_builder.alloc_null_literal(SPAN),
                                            ),
                                        ),
                                    ),
                                    LogicalOperator::And,
                                    Expression::BinaryExpression(
                                        ast_builder.alloc_binary_expression(
                                            SPAN,
                                            logical.left.clone_in(ast_builder.allocator),
                                            BinaryOperator::StrictInequality,
                                            Expression::Identifier(
                                                ast_builder
                                                    .alloc_identifier_reference(SPAN, "undefined"),
                                            ),
                                        ),
                                    ),
                                ),
                            ),
                            consequent: None,
                            alternate: res,
                        }])
                    }
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
                    if let ExtractResult::ExtractStyle(mut styles) = extract_style_from_expression(
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
                    ExtractResult::ExtractStyle(props)
                }
            }
            Expression::ConditionalExpression(ref mut conditional) => {
                ExtractResult::ExtractStyle(vec![ExtractStyleProp::Conditional {
                    condition: conditional.test.clone_in(ast_builder.allocator),
                    consequent: if let ExtractResult::ExtractStyle(styles) =
                        extract_style_from_expression(
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
                    alternate: if let ExtractResult::ExtractStyle(styles) =
                        extract_style_from_expression(
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
                }])
            }
            Expression::ObjectExpression(obj) => {
                let mut props = vec![];
                for p in obj.properties.iter_mut() {
                    if let ObjectPropertyKind::ObjectProperty(ref mut o) = p {
                        if let ExtractResult::ExtractStyle(ref mut ret) =
                            extract_style_from_expression(
                                ast_builder,
                                Some(&o.key.name().unwrap()),
                                &mut o.value,
                                level,
                                selector,
                            )
                        {
                            props.append(ret);
                        }
                    };
                }
                if props.is_empty() {
                    ExtractResult::Remove
                } else {
                    ExtractResult::ExtractStyle(props)
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
    selector: Option<&str>,
) -> ExtractResult<'a> {
    let mem_expression = &mem.expression.clone_in(ast_builder.allocator);
    let mut ret: Vec<ExtractStyleProp> = vec![];

    match &mut mem.object {
        Expression::ArrayExpression(array) => {
            if array.elements.is_empty() {
                return ExtractResult::Remove;
            }

            if let Some(num) = get_number_by_literal_expression(mem_expression) {
                if num < 0f64 {
                    return ExtractResult::Remove;
                }
                let mut etc = None;
                for (idx, p) in array.elements.iter_mut().enumerate() {
                    if let ArrayExpressionElement::SpreadElement(sp) = p {
                        etc = Some(sp.argument.clone_in(ast_builder.allocator));
                        continue;
                    }
                    if idx as f64 == num {
                        if let ExtractResult::ExtractStyle(styles) = extract_style_from_expression(
                            ast_builder,
                            name,
                            p.to_expression_mut(),
                            level,
                            selector,
                        ) {
                            return ExtractResult::ExtractStyle(styles);
                        }
                    }
                }
                return match etc {
                    None => ExtractResult::Remove,
                    Some(etc) => ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(
                        Dynamic(ExtractDynamicStyle::new(
                            name.unwrap(),
                            level,
                            expression_to_code(&Expression::ComputedMemberExpression(
                                ast_builder.alloc_computed_member_expression(
                                    SPAN,
                                    etc,
                                    mem_expression.clone_in(ast_builder.allocator),
                                    false,
                                ),
                            ))
                            .as_str(),
                            selector.map(|s| s.into()),
                        )),
                    )]),
                };
            }

            let mut map = BTreeMap::new();
            for (idx, p) in array.elements.iter_mut().enumerate() {
                if let ArrayExpressionElement::SpreadElement(ref sp) = p {
                    map.insert(
                        idx.to_string(),
                        Box::new(ExtractStyleProp::Static(Dynamic(ExtractDynamicStyle::new(
                            name.unwrap(),
                            level,
                            expression_to_code(&Expression::ComputedMemberExpression(
                                ast_builder.alloc_computed_member_expression(
                                    SPAN,
                                    sp.argument.clone_in(ast_builder.allocator),
                                    mem_expression.clone_in(ast_builder.allocator),
                                    false,
                                ),
                            ))
                            .as_str(),
                            selector.map(|s| s.into()),
                        )))),
                    );
                } else if let ExtractResult::ExtractStyle(styles) = extract_style_from_expression(
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
                return ExtractResult::Remove;
            }

            let mut map = BTreeMap::new();
            if let Some(k) = get_string_by_literal_expression(mem_expression) {
                let mut etc = None;
                for p in obj.properties.iter_mut() {
                    if let ObjectPropertyKind::ObjectProperty(ref mut o) = p {
                        if let PropertyKey::StaticIdentifier(ref pk) = o.key {
                            if pk.name == k {
                                if let ExtractResult::ExtractStyle(styles) =
                                    extract_style_from_expression(
                                        ast_builder,
                                        name,
                                        &mut o.value,
                                        level,
                                        selector,
                                    )
                                {
                                    return ExtractResult::ExtractStyle(styles);
                                }
                            }
                        }
                    } else if let ObjectPropertyKind::SpreadProperty(ref sp) = p {
                        etc = Some(sp.argument.clone_in(ast_builder.allocator));
                    }
                }
                match etc {
                    None => return ExtractResult::Remove,
                    Some(etc) => {
                        ret.push(ExtractStyleProp::Static(Dynamic(ExtractDynamicStyle::new(
                            name.unwrap(),
                            level,
                            expression_to_code(&Expression::ComputedMemberExpression(
                                ast_builder.alloc_computed_member_expression(
                                    SPAN,
                                    etc,
                                    mem_expression.clone_in(ast_builder.allocator),
                                    false,
                                ),
                            ))
                            .as_str(),
                            selector.map(|s| s.into()),
                        ))))
                    }
                }
            }

            for p in obj.properties.iter_mut() {
                if let ObjectPropertyKind::ObjectProperty(ref mut o) = p {
                    if let PropertyKey::StaticIdentifier(_)
                    | PropertyKey::NumericLiteral(_)
                    | PropertyKey::StringLiteral(_) = o.key
                    {
                        if let ExtractResult::ExtractStyle(styles) = extract_style_from_expression(
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
                    expression_to_code(&Expression::ComputedMemberExpression(
                        ast_builder.alloc_computed_member_expression(
                            SPAN,
                            mem.object.clone_in(ast_builder.allocator),
                            mem_expression.clone_in(ast_builder.allocator),
                            false,
                        ),
                    ))
                    .as_str(),
                    selector.map(|s| s.into()),
                ))))
            }
        }
        _ => {}
    };

    ExtractResult::ExtractStyle(ret)
}

fn get_number_by_literal_expression(expr: &Expression) -> Option<f64> {
    match expr {
        Expression::ParenthesizedExpression(parenthesized) => {
            get_number_by_literal_expression(&parenthesized.expression)
        }
        Expression::NumericLiteral(num) => Some(num.value),
        Expression::UnaryExpression(unary) => get_number_by_literal_expression(&unary.argument)
            .and_then(|num| match unary.operator {
                UnaryOperator::UnaryNegation => Some(-num),
                UnaryOperator::UnaryPlus => Some(num),
                _ => None,
            }),
        _ => None,
    }
}

fn get_string_by_literal_expression(expr: &Expression) -> Option<String> {
    get_number_by_literal_expression(expr)
        .map(|num| num.to_string())
        .or_else(|| match expr {
            Expression::ParenthesizedExpression(parenthesized) => {
                get_string_by_literal_expression(&parenthesized.expression)
            }
            Expression::StringLiteral(str) => Some(str.value.as_str().to_string()),
            Expression::TemplateLiteral(tmp) => {
                let mut collect = vec![];
                for (idx, q) in tmp.quasis.iter().enumerate() {
                    collect.push(q.value.raw.to_string());
                    if idx < tmp.expressions.len() {
                        if let Some(value) = get_string_by_literal_expression(&tmp.expressions[idx])
                        {
                            collect.push(value);
                        } else {
                            return None;
                        }
                    }
                }
                Some(collect.join(""))
            }
            _ => None,
        })
}
