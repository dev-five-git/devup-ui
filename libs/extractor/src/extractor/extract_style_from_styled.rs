use rustc_hash::FxHashMap;

use crate::{
    ExtractStyleProp,
    component::ExportVariableKind,
    css_utils::css_to_style_literal,
    extract_style::extract_style_value::ExtractStyleValue,
    extractor::{
        ExtractResult,
        extract_style_from_expression::{LiteralHandling, extract_style_from_expression},
    },
    gen_class_name::gen_class_names,
    gen_style::gen_styles,
    utils::{merge_object_expressions, wrap_array_filter},
};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        Argument, BindingPattern, BindingProperty, BindingRestElement, Expression, FormalParameter,
        FormalParameterKind, FormalParameters, FunctionBody, JSXAttributeItem, JSXAttributeName,
        JSXAttributeValue, JSXElementName, JSXOpeningElement, PropertyKey, Statement, Str,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;

fn extract_base_tag_and_class_name(
    input: &Expression<'_>,
    imports: &FxHashMap<String, ExportVariableKind>,
) -> (Option<String>, Option<Vec<ExtractStyleValue>>) {
    if let Expression::StaticMemberExpression(member) = input {
        (Some(member.property.name.to_string()), None)
    } else if let Expression::CallExpression(call) = input
        && call.arguments.len() == 1
    {
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
    split_filename: Option<&str>,
    imports: &FxHashMap<String, ExportVariableKind>,
) -> (ExtractResult<'a>, Expression<'a>) {
    let (result, new_expr) = if let Expression::TaggedTemplateExpression(tag) = expression
        && let (Some(tag_name), default_class_name) =
            extract_base_tag_and_class_name(&tag.tag, imports)
    {
        // Case 1: styled.div`css` or styled("div")`css`
        // Check if tag is styled.div or styled(...)
        // Extract CSS from template literal

        let styles = css_to_style_literal(&tag.quasi, 0, &None);
        let mut props_styles: Vec<ExtractStyleProp<'_>> = styles
            .iter()
            .map(|ex| ExtractStyleProp::Static(ex.clone().into()))
            .collect();

        if let Some(default_class_name) = default_class_name {
            props_styles.extend(default_class_name.into_iter().map(ExtractStyleProp::Static));
        }

        let class_name = gen_class_names(ast_builder, &mut props_styles, None, split_filename);
        let styled_component = create_styled_component(
            ast_builder,
            &tag_name,
            &class_name,
            &gen_styles(ast_builder, &props_styles, None),
        );

        let result = ExtractResult {
            styles: props_styles,
            tag: Some(Expression::new_string_literal(
                SPAN,
                Str::from_in(&tag_name, ast_builder.allocator()),
                None,
                ast_builder,
            )),
            style_order: None,
            style_vars: None,
            props: None,
        };

        (Some(result), Some(styled_component))
    } else if let Expression::CallExpression(call) = expression
        && let (Some(tag_name), default_class_name) =
            extract_base_tag_and_class_name(&call.callee, imports)
        && call.arguments.len() == 1
    {
        // Case 2: styled.div({ bg: "red" }) or styled("div")({ bg: "red" })
        // Check if this is a call to styled.div or styled("div")

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
            LiteralHandling::ExpandResponsiveThemeToken,
        );
        if let Some(default_class_name) = default_class_name {
            styles.extend(default_class_name.into_iter().map(ExtractStyleProp::Static));
        }

        let class_name = gen_class_names(ast_builder, &mut styles, style_order, split_filename);
        let styled_component = create_styled_component(
            ast_builder,
            &tag_name,
            &class_name,
            &gen_styles(ast_builder, &styles, None),
        );

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
    };
    (
        result.unwrap_or_else(ExtractResult::default),
        new_expr.unwrap_or_else(|| expression.clone_in(ast_builder.allocator())),
    )
}

fn create_styled_component<'a>(
    ast_builder: &AstBuilder<'a>,
    tag_name: &str,
    class_name: &Option<Expression<'a>>,
    style_vars: &Option<Expression<'a>>,
) -> Expression<'a> {
    let params = FormalParameters::new(
        SPAN,
        FormalParameterKind::ArrowFormalParameters,
        oxc_allocator::Vec::from_iter_in(
            vec![FormalParameter::new(
                SPAN,
                oxc_allocator::Vec::new_in(ast_builder),
                BindingPattern::new_object_pattern(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(
                        vec![
                            BindingProperty::new(
                                SPAN,
                                PropertyKey::new_static_identifier(SPAN, "style", ast_builder),
                                BindingPattern::new_binding_identifier(SPAN, "style", ast_builder),
                                true,
                                false,
                                ast_builder,
                            ),
                            BindingProperty::new(
                                SPAN,
                                PropertyKey::new_static_identifier(SPAN, "className", ast_builder),
                                BindingPattern::new_binding_identifier(
                                    SPAN,
                                    "className",
                                    ast_builder,
                                ),
                                true,
                                false,
                                ast_builder,
                            ),
                        ],
                        ast_builder,
                    ),
                    Some(BindingRestElement::new(
                        SPAN,
                        BindingPattern::new_binding_identifier(SPAN, "rest", ast_builder),
                        ast_builder,
                    )),
                    ast_builder,
                ),
                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
                None::<oxc_allocator::Box<Expression<'a>>>,
                false,
                None,
                false,
                false,
                ast_builder,
            )],
            ast_builder,
        ),
        None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
        ast_builder,
    );
    let body = FunctionBody::boxed(
        SPAN,
        oxc_allocator::Vec::new_in(ast_builder),
        oxc_allocator::Vec::from_iter_in(
            vec![Statement::new_expression_statement(
                SPAN,
                Expression::new_jsx_element(
                    SPAN,
                    JSXOpeningElement::boxed(
                        SPAN,
                        JSXElementName::new_identifier(
                            SPAN,
                            Str::from_in(tag_name, ast_builder.allocator()),
                            ast_builder,
                        ),
                        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
                        oxc_allocator::Vec::from_iter_in(
                            vec![
                                JSXAttributeItem::new_spread_attribute(
                                    SPAN,
                                    Expression::new_identifier(SPAN, "rest", ast_builder),
                                    ast_builder,
                                ),
                                JSXAttributeItem::new_attribute(
                                    SPAN,
                                    JSXAttributeName::new_identifier(
                                        SPAN,
                                        "className",
                                        ast_builder,
                                    ),
                                    Some(JSXAttributeValue::new_expression_container(
                                        SPAN,
                                        class_name
                                            .as_ref()
                                            .map_or_else(
                                                || {
                                                    Expression::new_identifier(
                                                        SPAN,
                                                        "className",
                                                        ast_builder,
                                                    )
                                                },
                                                |name| {
                                                    wrap_array_filter(
                                                        ast_builder,
                                                        &[
                                                            name.clone_in(ast_builder.allocator()),
                                                            Expression::new_identifier(
                                                                SPAN,
                                                                "className",
                                                                ast_builder,
                                                            ),
                                                        ],
                                                    )
                                                    .unwrap_or_else(|| {
                                                        name.clone_in(ast_builder.allocator())
                                                    })
                                                },
                                            )
                                            .into(),
                                        ast_builder,
                                    )),
                                    ast_builder,
                                ),
                                JSXAttributeItem::new_attribute(
                                    SPAN,
                                    JSXAttributeName::new_identifier(SPAN, "style", ast_builder),
                                    Some(JSXAttributeValue::new_expression_container(
                                        SPAN,
                                        style_vars
                                            .as_ref()
                                            .map_or_else(
                                                || {
                                                    Expression::new_identifier(
                                                        SPAN,
                                                        "style",
                                                        ast_builder,
                                                    )
                                                },
                                                |style_vars| {
                                                    merge_object_expressions(
                                                        ast_builder,
                                                        &[
                                                            style_vars
                                                                .clone_in(ast_builder.allocator()),
                                                            Expression::new_identifier(
                                                                SPAN,
                                                                "style",
                                                                ast_builder,
                                                            ),
                                                        ],
                                                    )
                                                    .unwrap_or_else(|| {
                                                        style_vars.clone_in(ast_builder.allocator())
                                                    })
                                                },
                                            )
                                            .into(),
                                        ast_builder,
                                    )),
                                    ast_builder,
                                ),
                            ],
                            ast_builder,
                        ),
                        ast_builder,
                    ),
                    oxc_allocator::Vec::new_in(ast_builder),
                    None::<oxc_allocator::Box<oxc_ast::ast::JSXClosingElement<'a>>>,
                    ast_builder,
                ),
                ast_builder,
            )],
            ast_builder,
        ),
        ast_builder,
    );
    Expression::new_arrow_function_expression(
        SPAN,
        true,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        params,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body,
        ast_builder,
    )
}
