use crate::extract_style::{ExtractDynamicStyle, ExtractStaticStyle};
use crate::ExtractStyleProp;
use crate::ExtractStyleValue::{Dynamic, Static};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{
    ArrayExpression, ConditionalExpression, Expression, JSXAttributeValue, JSXExpression,
};
use oxc_ast::AstBuilder;
use oxc_span::SPAN;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};

const IGNORED_IDENTIFIERS: [&str; 3] = ["undefined", "NaN", "Infinity"];

/// JSX attribute to style property
pub fn extract_style_prop<'a>(
    ast_builder: &AstBuilder<'a>,
    name: String,
    value: &JSXAttributeValue<'a>,
) -> Option<ExtractStyleProp<'a>> {
    match value {
        JSXAttributeValue::StringLiteral(str) => Some(ExtractStyleProp::Static(Static(
            ExtractStaticStyle::new(name, str.value.to_string(), 0, None),
        ))),
        JSXAttributeValue::ExpressionContainer(expression) => {
            if let JSXExpression::EmptyExpression(_) = expression.expression {
                None
            } else {
                extract_style_prop_from_express(
                    ast_builder,
                    name.as_str(),
                    expression.expression.to_expression(),
                    0,
                    None,
                )
            }
        }
        _ => None,
    }
}
pub fn extract_style_prop_from_express<'a>(
    ast_builder: &AstBuilder<'a>,
    name: &str,
    expression: &Expression<'a>,
    level: u8,
    selector: Option<&str>,
) -> Option<ExtractStyleProp<'a>> {
    match &expression {
        Expression::NumericLiteral(num) => {
            Some(ExtractStyleProp::Static(Static(ExtractStaticStyle::new(
                name.to_string(),
                num.value.to_string(),
                level,
                selector.map(|s| s.to_string()),
            ))))
        }
        Expression::StringLiteral(str) => {
            Some(ExtractStyleProp::Static(Static(ExtractStaticStyle::new(
                name.to_string(),
                str.value.to_string(),
                level,
                selector.map(|s| s.to_string()),
            ))))
        }
        Expression::Identifier(identifier) => {
            if IGNORED_IDENTIFIERS.contains(&identifier.name.as_str()) {
                None
            } else {
                Some(ExtractStyleProp::Static(Dynamic(ExtractDynamicStyle::new(
                    name.to_string(),
                    level,
                    identifier.name.to_string(),
                    selector.map(|s| s.to_string()),
                ))))
            }
        }
        Expression::LogicalExpression(logical) => match logical.operator {
            LogicalOperator::Or => Some(ExtractStyleProp::Conditional {
                condition: logical.left.clone_in(ast_builder.allocator),
                consequent: None,
                alternate: extract_style_prop_from_express(
                    ast_builder,
                    name,
                    &logical.right,
                    level,
                    selector,
                )
                .map(Box::new),
            }),
            LogicalOperator::And => Some(ExtractStyleProp::Conditional {
                condition: logical.left.clone_in(ast_builder.allocator),
                consequent: extract_style_prop_from_express(
                    ast_builder,
                    name,
                    &logical.right,
                    level,
                    selector,
                )
                .map(Box::new),
                alternate: None,
            }),
            LogicalOperator::Coalesce => Some(ExtractStyleProp::Conditional {
                condition: Expression::LogicalExpression(ast_builder.alloc_logical_expression(
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
                )),
                consequent: None,
                alternate: extract_style_prop_from_express(
                    ast_builder,
                    name,
                    &logical.right,
                    level,
                    selector,
                )
                .map(Box::new),
            }),
        },
        Expression::ParenthesizedExpression(parenthesized) => extract_style_prop_from_express(
            ast_builder,
            name,
            &parenthesized.expression,
            level,
            selector,
        ),
        Expression::ArrayExpression(array) => {
            extract_style_prop_from_array_express(ast_builder, name, array, selector)
        }
        Expression::ConditionalExpression(conditional) => {
            extract_style_prop_from_conditional_express(
                ast_builder,
                name,
                conditional,
                level,
                selector,
            )
        }
        _ => None,
    }
}

fn extract_style_prop_from_conditional_express<'a>(
    ast_builder: &AstBuilder<'a>,
    name: &str,
    conditional: &ConditionalExpression<'a>,
    level: u8,
    media: Option<&str>,
) -> Option<ExtractStyleProp<'a>> {
    Some(ExtractStyleProp::Conditional {
        condition: conditional.test.clone_in(ast_builder.allocator),
        consequent: extract_style_prop_from_express(
            ast_builder,
            name,
            &conditional.consequent,
            level,
            media,
        )
        .map(Box::new),
        alternate: extract_style_prop_from_express(
            ast_builder,
            name,
            &conditional.alternate,
            level,
            media,
        )
        .map(Box::new),
    })
}

fn extract_style_prop_from_array_express<'a>(
    ast_builder: &AstBuilder<'a>,
    name: &str,
    array: &ArrayExpression<'a>,
    selector: Option<&str>,
) -> Option<ExtractStyleProp<'a>> {
    let ret = array
        .elements
        .iter()
        .enumerate()
        .filter_map(|(idx, element)| {
            extract_style_prop_from_express(
                ast_builder,
                name,
                element.to_expression(),
                idx as u8,
                selector,
            )
        })
        .collect::<Vec<ExtractStyleProp>>();
    if ret.is_empty() {
        None
    } else {
        Some(ExtractStyleProp::Responsive(ret))
    }
}
