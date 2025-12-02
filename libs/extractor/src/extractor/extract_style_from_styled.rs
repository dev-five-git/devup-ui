use std::collections::HashMap;

use crate::{
    ExtractStyleProp,
    component::ExportVariableKind,
    css_utils::css_to_style,
    extract_style::extract_style_value::ExtractStyleValue,
    extractor::{ExtractResult, extract_style_from_expression::extract_style_from_expression},
    gen_class_name::gen_class_names,
    utils::{merge_object_expressions, wrap_array_filter},
};
use oxc_allocator::CloneIn;
use oxc_ast::{
    AstBuilder,
    ast::{Argument, Expression, FormalParameterKind},
};
use oxc_span::SPAN;

fn extract_base_tag_and_class_name<'a>(
    input: &Expression<'a>,
    styled_name: &str,
    imports: &HashMap<String, ExportVariableKind>,
) -> (Option<String>, Option<Vec<ExtractStyleValue>>) {
    match input {
        Expression::StaticMemberExpression(member) => {
            if let Expression::Identifier(ident) = &member.object {
                if ident.name.as_str() == styled_name {
                    (Some(member.property.name.to_string()), None)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        }
        Expression::CallExpression(call) => {
            if let Expression::Identifier(ident) = &call.callee {
                if ident.name.as_str() == styled_name && call.arguments.len() == 1 {
                    // styled("div") or styled(Component)
                    if let Argument::StringLiteral(lit) = &call.arguments[0] {
                        (Some(lit.value.to_string()), None)
                    } else if let Argument::Identifier(ident) = &call.arguments[0] {
                        if let Some(export_variable_kind) = imports.get(ident.name.as_str()) {
                            (
                                Some(export_variable_kind.to_tag().to_string()),
                                Some(export_variable_kind.extract()),
                            )
                        } else {
                            (Some(ident.name.to_string()), None)
                        }
                    } else {
                        // Component reference - we'll handle this later
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
}

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
    imports: &HashMap<String, ExportVariableKind>,
) -> (ExtractResult<'a>, Expression<'a>) {
    println!("-----------");
    let (result, new_expr) = match expression {
        // Case 1: styled.div`css` or styled("div")`css`
        Expression::TaggedTemplateExpression(tag) => {
            // Check if tag is styled.div or styled(...)
            let (tag_name, default_class_name) =
                extract_base_tag_and_class_name(&tag.tag, styled_name, imports);

            if let Some(tag_name) = tag_name {
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

                if let Some(default_class_name) = default_class_name {
                    props_styles.extend(
                        default_class_name
                            .into_iter()
                            .map(ExtractStyleProp::Static),
                    );
                }

                let class_name =
                    gen_class_names(ast_builder, &mut props_styles, None, split_filename);

                let new_expr = create_styled_component(ast_builder, &tag_name, &class_name, &None);
                let result = ExtractResult {
                    styles: props_styles,
                    tag: Some(ast_builder.expression_string_literal(
                        SPAN,
                        ast_builder.atom(&tag_name),
                        None,
                    )),
                    style_order: None,
                    style_vars: None,
                    props: None,
                };

                (Some(result), Some(new_expr))
            } else {
                (None, None)
            }
        }
        // Case 2: styled.div({ bg: "red" }) or styled("div")({ bg: "red" })
        Expression::CallExpression(call) => {
            // Check if this is a call to styled.div or styled("div")
            let (tag_name, default_class_name) =
                extract_base_tag_and_class_name(&call.callee, styled_name, imports);

            println!(
                "tag_name: {:?}, default_class_name: {:?}",
                tag_name, default_class_name
            );

            if let Some(tag_name) = tag_name
                && call.arguments.len() == 1
            {
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
                if let Some(default_class_name) = default_class_name {
                    styles.extend(
                        default_class_name
                            .into_iter()
                            .map(ExtractStyleProp::Static),
                    );
                }

                let class_name =
                    gen_class_names(ast_builder, &mut styles, style_order, split_filename);
                let styled_component =
                    create_styled_component(ast_builder, &tag_name, &class_name, &style_vars);

                let result = ExtractResult {
                    styles,
                    tag: None,
                    style_order,
                    style_vars,
                    props,
                };

                (Some(result), Some(styled_component))
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    };
    (
        result.unwrap_or(ExtractResult::default()),
        new_expr.unwrap_or(expression.clone_in(ast_builder.allocator)),
    )
}

fn create_styled_component<'a>(
    ast_builder: &AstBuilder<'a>,
    tag_name: &str,
    class_name: &Option<Expression<'a>>,
    style_vars: &Option<Expression<'a>>,
) -> Expression<'a> {
    let params = ast_builder.formal_parameters(
        SPAN,
        FormalParameterKind::ArrowFormalParameters,
        oxc_allocator::Vec::from_iter_in(
            vec![ast_builder.formal_parameter(
                SPAN,
                oxc_allocator::Vec::from_iter_in(vec![], ast_builder.allocator),
                ast_builder.binding_pattern(
                    ast_builder.binding_pattern_kind_object_pattern(
                        SPAN,
                        oxc_allocator::Vec::from_iter_in(
                            vec![
                                    ast_builder.binding_property(
                                        SPAN,
                                        ast_builder.property_key_static_identifier(SPAN, "style"),
                                        ast_builder.binding_pattern(
                                            ast_builder.binding_pattern_kind_binding_identifier(
                                                SPAN, "style",
                                            ),
                                            None::<
                                                oxc_allocator::Box<
                                                    oxc_ast::ast::TSTypeAnnotation<'a>,
                                                >,
                                            >,
                                            false,
                                        ),
                                        true,
                                        false,
                                    ),
                                    ast_builder.binding_property(
                                        SPAN,
                                        ast_builder
                                            .property_key_static_identifier(SPAN, "className"),
                                        ast_builder.binding_pattern(
                                            ast_builder.binding_pattern_kind_binding_identifier(
                                                SPAN,
                                                "className",
                                            ),
                                            None::<
                                                oxc_allocator::Box<
                                                    oxc_ast::ast::TSTypeAnnotation<'a>,
                                                >,
                                            >,
                                            false,
                                        ),
                                        true,
                                        false,
                                    ),
                                ],
                            ast_builder.allocator,
                        ),
                        Some(ast_builder.binding_rest_element(
                            SPAN,
                            ast_builder.binding_pattern(
                                ast_builder.binding_pattern_kind_binding_identifier(
                                    SPAN,
                                    ast_builder.atom("rest"),
                                ),
                                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
                                false,
                            ),
                        )),
                    ),
                    None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
                    false,
                ),
                None,
                false,
                false,
            )],
            ast_builder.allocator,
        ),
        None::<oxc_allocator::Box<oxc_ast::ast::BindingRestElement<'a>>>,
    );
    let body = ast_builder.alloc_function_body(
        SPAN,
        oxc_allocator::Vec::from_iter_in(vec![], ast_builder.allocator),
        oxc_allocator::Vec::from_iter_in(
            vec![ast_builder.statement_expression(
                SPAN,
                ast_builder.expression_jsx_element(
                    SPAN,
                    ast_builder.alloc_jsx_opening_element(
                        SPAN,
                        ast_builder.jsx_element_name_identifier(SPAN, ast_builder.atom(tag_name)),
                        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
                        oxc_allocator::Vec::from_iter_in(
                            vec![
                                    ast_builder.jsx_attribute_item_spread_attribute(
                                        SPAN,
                                        ast_builder
                                            .expression_identifier(SPAN, ast_builder.atom("rest")),
                                    ),
                                    ast_builder.jsx_attribute_item_attribute(
                                        SPAN,
                                        ast_builder.jsx_attribute_name_identifier(
                                            SPAN,
                                            ast_builder.atom("className"),
                                        ),
                                        Some(
                                            ast_builder.jsx_attribute_value_expression_container(
                                                SPAN,
                                                class_name
                                                    .as_ref()
                                                    .map(|name| {
                                                        wrap_array_filter(
                                                            ast_builder,
                                                            &[
                                                                name.clone_in(
                                                                    ast_builder.allocator,
                                                                ),
                                                                ast_builder.expression_identifier(
                                                                    SPAN,
                                                                    ast_builder.atom("className"),
                                                                ),
                                                            ],
                                                        )
                                                        .unwrap()
                                                    })
                                                    .unwrap_or_else(|| {
                                                        ast_builder.expression_identifier(
                                                            SPAN,
                                                            ast_builder.atom("className"),
                                                        )
                                                    })
                                                    .into(),
                                            ),
                                        ),
                                    ),
                                    ast_builder.jsx_attribute_item_attribute(
                                        SPAN,
                                        ast_builder.jsx_attribute_name_identifier(
                                            SPAN,
                                            ast_builder.atom("style"),
                                        ),
                                        Some(
                                            ast_builder.jsx_attribute_value_expression_container(
                                                SPAN,
                                                style_vars
                                                    .as_ref()
                                                    .map(|style_vars| {
                                                        merge_object_expressions(
                                                            ast_builder,
                                                            &[
                                                                style_vars.clone_in(
                                                                    ast_builder.allocator,
                                                                ),
                                                                ast_builder.expression_identifier(
                                                                    SPAN,
                                                                    ast_builder.atom("style"),
                                                                ),
                                                            ],
                                                        )
                                                        .unwrap()
                                                    })
                                                    .unwrap_or_else(|| {
                                                        ast_builder.expression_identifier(
                                                            SPAN,
                                                            ast_builder.atom("style"),
                                                        )
                                                    })
                                                    .into(),
                                            ),
                                        ),
                                    ),
                                ],
                            ast_builder.allocator,
                        ),
                    ),
                    oxc_allocator::Vec::from_iter_in(vec![], ast_builder.allocator),
                    None::<oxc_allocator::Box<oxc_ast::ast::JSXClosingElement<'a>>>,
                ),
            )],
            ast_builder.allocator,
        ),
    );
    ast_builder.expression_arrow_function(
        SPAN,
        true,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        params,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body,
    )
}
