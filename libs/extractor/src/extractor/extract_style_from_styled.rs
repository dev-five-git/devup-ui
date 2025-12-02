use crate::{
    ExtractStyleProp, css_utils::css_to_style,
    extract_style::extract_style_value::ExtractStyleValue, extractor::ExtractResult,
    extractor::extract_style_from_expression::extract_style_from_expression,
    gen_class_name::gen_class_names,
};
use oxc_allocator::CloneIn;
use oxc_ast::{
    AstBuilder,
    ast::{Argument, Expression},
};
use oxc_span::SPAN;

/// Extract styles from styled function calls
/// Handles patterns like:
/// - styled.div`css`
/// - styled("div")`css`
/// - styled("div")({ bg: "red" })
/// - styled.div({ bg: "red" })
/// - styled(Component)({ bg: "red" })
pub fn extract_style_from_styled<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    styled_name: &str,
    split_filename: Option<&str>,
) -> (ExtractResult<'a>, Expression<'a>) {
    match expression {
        // Case 1: styled.div`css` or styled("div")`css`
        Expression::TaggedTemplateExpression(tag) => {
            // Check if tag is styled.div or styled(...)
            let (tag_name, is_member) = match &tag.tag {
                Expression::StaticMemberExpression(member) => {
                    if let Expression::Identifier(ident) = &member.object {
                        if ident.name.as_str() == styled_name {
                            (Some(member.property.name.to_string()), true)
                        } else {
                            (None, false)
                        }
                    } else {
                        (None, false)
                    }
                }
                Expression::CallExpression(call) => {
                    if let Expression::Identifier(ident) = &call.callee {
                        if ident.name.as_str() == styled_name && call.arguments.len() == 1 {
                            // styled("div") or styled(Component)
                            if let Argument::StringLiteral(lit) = &call.arguments[0] {
                                (Some(lit.value.to_string()), false)
                            } else {
                                // Component reference - we'll handle this later
                                (None, false)
                            }
                        } else {
                            (None, false)
                        }
                    } else {
                        (None, false)
                    }
                }
                _ => (None, false),
            };

            if tag_name.is_some() || is_member {
                // Extract CSS from template literal
                let css_str = tag
                    .quasi
                    .quasis
                    .iter()
                    .map(|quasi| quasi.value.raw.to_string())
                    .collect::<String>();

                let styles = css_to_style(&css_str, 0, &None);
                let mut props_styles: Vec<ExtractStyleProp<'_>> = styles
                    .iter()
                    .map(|ex| ExtractStyleProp::Static(ExtractStyleValue::Static(ex.clone())))
                    .collect();

                let class_name =
                    gen_class_names(ast_builder, &mut props_styles, None, split_filename);

                let result = ExtractResult {
                    styles: props_styles,
                    tag: tag_name.map(|name| {
                        ast_builder.expression_string_literal(SPAN, ast_builder.atom(&name), None)
                    }),
                    style_order: None,
                    style_vars: None,
                    props: None,
                };

                let new_expr = if let Some(cls) = class_name {
                    cls
                } else {
                    ast_builder.expression_string_literal(SPAN, ast_builder.atom(""), None)
                };

                return (result, new_expr);
            }
        }
        // Case 2: styled.div({ bg: "red" }) or styled("div")({ bg: "red" })
        Expression::CallExpression(call) => {
            // Check if this is a call to styled.div or styled("div")
            let (tag_name, is_member) = match &call.callee {
                Expression::StaticMemberExpression(member) => {
                    if let Expression::Identifier(ident) = &member.object {
                        if ident.name.as_str() == styled_name {
                            (Some(member.property.name.to_string()), true)
                        } else {
                            (None, false)
                        }
                    } else {
                        (None, false)
                    }
                }
                Expression::CallExpression(inner_call) => {
                    if let Expression::Identifier(ident) = &inner_call.callee {
                        if ident.name.as_str() == styled_name && inner_call.arguments.len() == 1 {
                            // styled("div") or styled(Component)
                            if let Argument::StringLiteral(lit) = &inner_call.arguments[0] {
                                (Some(lit.value.to_string()), false)
                            } else {
                                // Component reference
                                (None, false)
                            }
                        } else {
                            (None, false)
                        }
                    } else {
                        (None, false)
                    }
                }
                _ => (None, false),
            };

            if (tag_name.is_some() || is_member) && call.arguments.len() == 1 {
                // Extract styles from object expression
                let ExtractResult {
                    mut styles,
                    style_order,
                    style_vars,
                    props,
                    ..
                } = extract_style_from_expression(
                    ast_builder,
                    None,
                    if let Argument::SpreadElement(spread) = &mut call.arguments[0] {
                        &mut spread.argument
                    } else {
                        call.arguments[0].to_expression_mut()
                    },
                    0,
                    &None,
                );

                let class_name =
                    gen_class_names(ast_builder, &mut styles, style_order, split_filename);

                let result = ExtractResult {
                    styles,
                    tag: tag_name.map(|name| {
                        ast_builder.expression_string_literal(SPAN, ast_builder.atom(&name), None)
                    }),
                    style_order,
                    style_vars,
                    props,
                };

                let new_expr = if let Some(cls) = class_name {
                    cls
                } else {
                    ast_builder.expression_string_literal(SPAN, ast_builder.atom(""), None)
                };

                return (result, new_expr);
            }
        }
        _ => {}
    }

    // Default: no extraction
    (
        ExtractResult::default(),
        expression.clone_in(ast_builder.allocator),
    )
}
