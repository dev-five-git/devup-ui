use crate::utils::{expression_to_code, is_special_property};
use crate::{ExtractStyleProp, StyleProperty};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{Expression, JSXAttributeValue, ObjectPropertyKind, PropertyKey};

use crate::extract_style::ExtractStyleValue::{Dynamic, Static, Typography};
use crate::extract_style::{
    ExtractDynamicStyle, ExtractStaticStyle, ExtractStyleProperty, ExtractStyleValue,
};
use crate::prop_modify_utils::modify_prop_object;
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
    // attribute will be removed
    Remove,
    // attribute will be removed and the value will be extracted
    ExtractStyle(Vec<ExtractStyleProp<'a>>),
    // attribute will be removed and the tag will be changed
    ChangeTag(String),

    ExtractStyleWithChangeTag(Vec<ExtractStyleProp<'a>>, String),
}

impl<'a> From<ExtractResult<'a>> for Vec<ExtractResult<'a>> {
    fn from(v: ExtractResult<'a>) -> Self {
        vec![v]
    }
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
                    if let ObjectPropertyKind::ObjectProperty(prop) = &mut prop {
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
                                    styles.reverse();
                                    props_styles.append(&mut styles);
                                    true
                                }
                                ExtractResult::ChangeTag(t) => {
                                    tag = Some(t);
                                    true
                                }
                                ExtractResult::ExtractStyleWithChangeTag(mut styles, t) => {
                                    styles.reverse();
                                    props_styles.append(&mut styles);
                                    tag = Some(t);
                                    true
                                }
                            }
                        }
                    }
                    if !rm {
                        obj.properties.insert(idx, prop);
                    }
                }
                if props_styles.is_empty() {
                    ExtractResult::Maintain
                } else {
                    modify_prop_object(ast_builder, &mut obj.properties, &props_styles);
                    let ret = if let Some(tag) = tag {
                        ExtractResult::ExtractStyleWithChangeTag(props_styles, tag)
                    } else {
                        ExtractResult::ExtractStyle(props_styles)
                    };
                    ret
                }
            }
            _ => ExtractResult::Maintain,
        };
    }

    if let Some(name) = name {
        if is_special_property(name) {
            return ExtractResult::Maintain;
        }

        if name == "as" {
            if let Expression::StringLiteral(ident) = &expression {
                return ExtractResult::ChangeTag(ident.value.to_string());
            }
            return ExtractResult::Remove;
        }

        if let Some(selector) = name.strip_prefix("_") {
            return extract_style_from_expression(
                ast_builder,
                None,
                expression,
                level,
                Some(selector),
            );
        }
        if name == "typography" {
            typo = true;
        }
    }
    match expression {
        Expression::ComputedMemberExpression(mem) => {
            let mem_expression = &mem.expression.clone_in(ast_builder.allocator);
            match &mut mem.object {
                Expression::ArrayExpression(array) => {
                    for element in array.elements.iter_mut() {
                        if let Expression::StringLiteral(str) = element.to_expression_mut() {
                            if let Some(rest) = str.value.strip_prefix("$") {
                                str.value = ast_builder.atom(&format!("var(--{})", rest));
                            }
                        } else if let Expression::TemplateLiteral(tmp) = element.to_expression_mut()
                        {
                            if tmp.quasis.len() == 1 {
                                if let Some(rest) = tmp.quasis[0].value.raw.strip_prefix("$") {
                                    tmp.quasis[0].value.raw =
                                        ast_builder.atom(&format!("var(--{})", rest));
                                }
                            }
                        }
                    }

                    match mem_expression {
                        Expression::NumericLiteral(v) => {
                            if array.elements.len() < v.value as usize {
                                // wrong indexing case
                                ExtractResult::Remove
                            } else {
                                extract_style_from_expression(
                                    ast_builder,
                                    name,
                                    array.elements[v.value as usize]
                                        .clone_in(ast_builder.allocator)
                                        .to_expression_mut(),
                                    level,
                                    selector,
                                )
                            }
                        }
                        // wrong indexing case
                        Expression::UnaryExpression(unary) => {
                            if let Expression::NumericLiteral(_) = &unary.argument {
                                ExtractResult::Remove
                            } else {
                                ExtractResult::Maintain
                            }
                        }
                        _ => {
                            if let Some(name) = name {
                                return ExtractResult::ExtractStyle(vec![
                                    ExtractStyleProp::Static(Dynamic(ExtractDynamicStyle::new(
                                        name,
                                        level,
                                        expression_to_code(expression).as_str(),
                                        selector.map(|s| s.into()),
                                    ))),
                                ]);
                            }
                            ExtractResult::Maintain
                        }
                    }
                }
                Expression::ObjectExpression(obj) => match mem_expression {
                    Expression::StringLiteral(str) => {
                        let key = str.value.as_str();
                        for p in obj.properties.iter() {
                            match p {
                                ObjectPropertyKind::ObjectProperty(o) => {
                                    if let PropertyKey::StaticIdentifier(ident) = &o.key {
                                        if ident.name == key {
                                            return extract_style_from_expression(
                                                ast_builder,
                                                name,
                                                &mut o.value.clone_in(ast_builder.allocator),
                                                level,
                                                selector,
                                            );
                                        }
                                    }
                                }
                                ObjectPropertyKind::SpreadProperty(_) => {
                                    if let Some(name) = name {
                                        return ExtractResult::ExtractStyle(vec![
                                            ExtractStyleProp::Static(Dynamic(
                                                ExtractDynamicStyle::new(
                                                    name,
                                                    level,
                                                    expression_to_code(expression).as_str(),
                                                    selector.map(|s| s.into()),
                                                ),
                                            )),
                                        ]);
                                    }
                                }
                            }
                        }
                        ExtractResult::Remove
                    }
                    Expression::Identifier(_) => {
                        if let Some(name) = name {
                            return ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(
                                Dynamic(ExtractDynamicStyle::new(
                                    name,
                                    level,
                                    expression_to_code(expression).as_str(),
                                    selector.map(|s| s.into()),
                                )),
                            )]);
                        }
                        ExtractResult::Maintain
                    }
                    _ => ExtractResult::Maintain,
                },
                _ => ExtractResult::Maintain,
            }
        }
        Expression::NumericLiteral(v) => {
            if let Some(name) = name {
                ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(Static(
                    ExtractStaticStyle::new(
                        name,
                        &v.value.to_string(),
                        level,
                        selector.map(|s| s.into()),
                    ),
                ))])
            } else {
                ExtractResult::Maintain
            }
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
        Expression::StringLiteral(v) => {
            if let Some(name) = name {
                ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(if typo {
                    Typography(v.value.as_str().to_string())
                } else {
                    Static(ExtractStaticStyle::new(
                        name,
                        v.value.as_str(),
                        level,
                        selector.map(|s| s.into()),
                    ))
                })])
            } else {
                ExtractResult::Maintain
            }
        }
        Expression::Identifier(identifier) => {
            if IGNORED_IDENTIFIERS.contains(&identifier.name.as_str()) {
                ExtractResult::Maintain
            } else if let Some(name) = name {
                ExtractResult::ExtractStyle(vec![ExtractStyleProp::Static(Dynamic(
                    ExtractDynamicStyle::new(
                        name,
                        level,
                        identifier.name.as_str(),
                        selector.map(|s| s.into()),
                    ),
                ))])
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
                    ExtractResult::ExtractStyle(mut styles) => Some(Box::new(styles.remove(0))),
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
                                Expression::BinaryExpression(ast_builder.alloc_binary_expression(
                                    SPAN,
                                    logical.left.clone_in(ast_builder.allocator),
                                    BinaryOperator::StrictInequality,
                                    Expression::NullLiteral(ast_builder.alloc_null_literal(SPAN)),
                                )),
                                LogicalOperator::And,
                                Expression::BinaryExpression(ast_builder.alloc_binary_expression(
                                    SPAN,
                                    logical.left.clone_in(ast_builder.allocator),
                                    BinaryOperator::StrictInequality,
                                    Expression::Identifier(
                                        ast_builder.alloc_identifier_reference(SPAN, "undefined"),
                                    ),
                                )),
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
                let a = extract_style_from_expression(
                    ast_builder,
                    name,
                    element.to_expression_mut(),
                    idx as u8,
                    selector,
                );

                if let ExtractResult::ExtractStyle(mut styles) = a {
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
                    Some(Box::new(ExtractStyleProp::Responsive(styles)))
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
                    Some(Box::new(ExtractStyleProp::Responsive(styles)))
                } else {
                    None
                },
            }])
        }
        Expression::ObjectExpression(obj) => {
            let mut props = vec![];
            for p in obj.properties.iter_mut() {
                if let ObjectPropertyKind::ObjectProperty(ref mut o) = p {
                    let name = o.key.name().unwrap();
                    if let ExtractResult::ExtractStyle(ref mut ret) = extract_style_from_expression(
                        ast_builder,
                        Some(&name),
                        &mut o.value,
                        level,
                        selector,
                    ) {
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
        _ => ExtractResult::Maintain,
    }
}
