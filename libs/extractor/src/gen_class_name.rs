use crate::{ExtractStyleProp, StyleProperty};
use oxc_allocator::CloneIn;
use oxc_ast::ast::{
    Expression, JSXAttribute, JSXAttributeValue, JSXExpression, TemplateElement,
    TemplateElementValue,
};
use oxc_ast::AstBuilder;
use oxc_span::SPAN;

pub fn gen_class_names<'a>(
    ast_builder: &AstBuilder<'a>,
    style_props: &[ExtractStyleProp<'a>],
) -> Option<Expression<'a>> {
    merge_expression_for_class_name(
        ast_builder,
        style_props
            .iter()
            .filter_map(|st| gen_class_name(ast_builder, st))
            .rev()
            .collect(),
    )
}

fn gen_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &ExtractStyleProp<'a>,
) -> Option<Expression<'a>> {
    match style_prop {
        ExtractStyleProp::Static(st) => Some(Expression::StringLiteral(
            ast_builder.alloc_string_literal(
                SPAN,
                (match st.extract() {
                    StyleProperty::ClassName(cls) => cls,
                    StyleProperty::Variable { class_name, .. } => class_name,
                })
                .as_str(),
                None,
            ),
        )),
        ExtractStyleProp::Responsive(res) => merge_expression_for_class_name(
            ast_builder,
            res.iter()
                .filter_map(|st| gen_class_name(ast_builder, st))
                .collect(),
        ),
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
            ..
        } => {
            let consequent = if let Some(con) = consequent {
                gen_class_name(ast_builder, con).unwrap_or(Expression::StringLiteral(
                    ast_builder.alloc_string_literal(SPAN, "", None),
                ))
            } else {
                Expression::StringLiteral(ast_builder.alloc_string_literal(SPAN, "", None))
            };
            let alternate = if let Some(alt) = alternate {
                gen_class_name(ast_builder, alt).unwrap_or(Expression::StringLiteral(
                    ast_builder.alloc_string_literal(SPAN, "", None),
                ))
            } else {
                Expression::StringLiteral(ast_builder.alloc_string_literal(SPAN, "", None))
            };
            if is_same_expression(&consequent, &alternate) {
                return Some(consequent);
            }
            Some(Expression::ConditionalExpression(
                ast_builder.alloc_conditional_expression(
                    SPAN,
                    condition.clone_in(ast_builder.allocator),
                    consequent,
                    alternate,
                ),
            ))
        }
    }
}
fn is_same_expression<'a>(a: &Expression<'a>, b: &Expression<'a>) -> bool {
    match (a, b) {
        (Expression::StringLiteral(a), Expression::StringLiteral(b)) => a.value == b.value,
        (Expression::TemplateLiteral(a), Expression::TemplateLiteral(b)) => {
            a.quasis.len() == b.quasis.len()
                && a.expressions.len() == b.expressions.len()
                && a.quasis
                    .iter()
                    .zip(b.quasis.iter())
                    .all(|(a, b)| a.value.raw == b.value.raw && a.tail == b.tail)
                && a.expressions
                    .iter()
                    .zip(b.expressions.iter())
                    .all(|(a, b)| is_same_expression(a, b))
        }
        _ => false,
    }
}

pub fn merge_expression_for_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: Vec<Expression<'a>>,
) -> Option<Expression<'a>> {
    let mut class_names = vec![];
    let mut unknown_expr = vec![];
    for expr in expressions {
        match expr {
            Expression::StringLiteral(str) => {
                class_names.push(str.value.to_string().trim().to_string())
            }
            _ => {
                unknown_expr.push(expr);
            }
        }
    }
    if unknown_expr.is_empty() && class_names.is_empty() {
        return None;
    }
    let mut class_name = class_names.join(" ");
    if !unknown_expr.is_empty() {
        if class_name.is_empty() && unknown_expr.len() == 1 {
            Some(unknown_expr.remove(0))
        } else {
            let mut qu = oxc_allocator::Vec::new_in(ast_builder.allocator);
            for idx in 0..unknown_expr.len() + 1 {
                if idx == 0 {
                    qu.push(TemplateElement {
                        span: SPAN,
                        value: TemplateElementValue {
                            raw: ast_builder.atom(if class_name.is_empty() {
                                ""
                            } else {
                                class_name.push(' ');
                                class_name.as_str()
                            }),
                            cooked: None,
                        },
                        tail: false,
                    });
                } else if idx == unknown_expr.len() {
                    qu.push(TemplateElement {
                        span: SPAN,
                        value: TemplateElementValue {
                            raw: ast_builder.atom(""),
                            cooked: None,
                        },
                        tail: true,
                    })
                } else {
                    qu.push(TemplateElement {
                        span: SPAN,
                        value: TemplateElementValue {
                            raw: ast_builder.atom(" "),
                            cooked: None,
                        },
                        tail: false,
                    });
                }
            }

            Some(Expression::TemplateLiteral(
                ast_builder.alloc_template_literal(
                    SPAN,
                    qu,
                    oxc_allocator::Vec::from_iter_in(unknown_expr, ast_builder.allocator),
                ),
            ))
        }
    } else if class_name.is_empty() {
        None
    } else {
        Some(Expression::StringLiteral(ast_builder.alloc_string_literal(
            SPAN,
            class_name.as_str(),
            None,
        )))
    }
}

pub fn apply_class_name_attribute<'a>(
    ast_builder: &AstBuilder<'a>,
    class_prop: &mut JSXAttribute<'a>,
    expression: Expression<'a>,
) {
    if let Some(ref value) = class_prop.value {
        if let Some(ret) = match value {
            JSXAttributeValue::StringLiteral(str) => merge_expression_for_class_name(
                ast_builder,
                vec![
                    Expression::StringLiteral(str.clone_in(ast_builder.allocator)),
                    expression,
                ],
            ),
            JSXAttributeValue::ExpressionContainer(container) => match container.expression {
                JSXExpression::EmptyExpression(_) => Some(expression),
                _ => merge_expression_for_class_name(
                    ast_builder,
                    vec![
                        container
                            .expression
                            .clone_in(ast_builder.allocator)
                            .into_expression(),
                        expression,
                    ],
                ),
            },
            _ => None,
        } {
            class_prop.value = match ret {
                Expression::StringLiteral(literal) => Some(JSXAttributeValue::StringLiteral(
                    literal.clone_in(ast_builder.allocator),
                )),
                _ => Some(JSXAttributeValue::ExpressionContainer(
                    ast_builder.alloc_jsx_expression_container(SPAN, JSXExpression::from(ret)),
                )),
            }
        }
    } else {
        class_prop.value = Some(if let Expression::StringLiteral(literal) = expression {
            JSXAttributeValue::StringLiteral(literal.clone_in(ast_builder.allocator))
        } else {
            JSXAttributeValue::ExpressionContainer(
                ast_builder.alloc_jsx_expression_container(SPAN, JSXExpression::from(expression)),
            )
        });
    };
}
