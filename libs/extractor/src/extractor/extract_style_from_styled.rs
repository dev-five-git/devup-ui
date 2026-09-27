use rustc_hash::FxHashMap;

use crate::{
    ExtractStyleProp,
    component::ExportVariableKind,
    css_utils::{TemplateStyles, css_to_style_template},
    extract_style::extract_style_value::ExtractStyleValue,
    extractor::{
        ExtractResult,
        extract_style_from_expression::{LiteralHandling, extract_style_from_expression},
    },
    gen_class_name::{gen_class_names, merge_expression_for_class_name},
    gen_style::gen_styles,
    utils::{
        StyleArguments, merge_object_expressions, style_arguments, uncomposable_error,
        unplaced_error, unwrap_syntax_only, unwrap_syntax_only_mut, wrap_array_filter,
        wrap_direct_call,
    },
};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        Argument, BindingPattern, BindingProperty, BindingRestElement, CallExpression, Expression,
        FormalParameter, FormalParameterKind, FormalParameters, JSXAttributeItem, JSXAttributeName,
        JSXAttributeValue, JSXElementName, JSXOpeningElement, ObjectPropertyKind, PropertyKey, Str,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

fn extract_base_tag_and_class_name(
    input: &Expression<'_>,
    imports: &FxHashMap<String, ExportVariableKind>,
) -> (Option<String>, Option<Vec<ExtractStyleValue>>) {
    let input = unwrap_syntax_only(input);
    if let Expression::StaticMemberExpression(member) = input {
        (Some(member.property.name.to_string()), None)
    } else if let Expression::CallExpression(call) = input
        && call.arguments.len() == 1
        && let Some((tag_name, default_class_name)) = tag_from_argument(&call.arguments[0], imports)
    {
        // styled("div") or styled(Component)
        (Some(tag_name), default_class_name)
    } else {
        (None, None)
    }
}

/// Read the base tag out of a `styled(...)` argument, resolving a devup-ui component
/// reference to both its HTML tag and the styles that component contributes by default.
fn tag_from_argument(
    argument: &Argument<'_>,
    imports: &FxHashMap<String, ExportVariableKind>,
) -> Option<(String, Option<Vec<ExtractStyleValue>>)> {
    match argument {
        Argument::StringLiteral(lit) => Some((lit.value.to_string(), None)),
        Argument::Identifier(ident) => Some(match imports.get(ident.name.as_str()) {
            Some(export_variable_kind) => (
                export_variable_kind.to_tag().to_string(),
                Some(export_variable_kind.extract()),
            ),
            None => (ident.name.to_string(), None),
        }),
        _ => None,
    }
}

/// Resolve a `styled(...)` call to its base tag, default styles, and the index of the
/// argument holding the style object.
///
/// Two spellings build the same component: the curried `styled.div({...})` /
/// `styled("div")({...})`, whose callee already carries the tag, and the two-argument
/// `styled("div", {...})`, whose callee is the bare `styled` identifier.
fn resolve_styled_call_target(
    call: &CallExpression<'_>,
    imports: &FxHashMap<String, ExportVariableKind>,
) -> Option<(String, Option<Vec<ExtractStyleValue>>, usize)> {
    if call.arguments.len() == 1
        && let (Some(tag_name), default_class_name) =
            extract_base_tag_and_class_name(&call.callee, imports)
    {
        return Some((tag_name, default_class_name, 0));
    }
    // The style object must be a literal: it is the only way to tell `styled(tag, styles)`
    // apart from a malformed `styled("div", "span")`, which must be left untouched.
    if call.arguments.len() == 2
        && matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(_))
        && call.arguments[1].as_expression().is_some_and(|styles| {
            matches!(unwrap_syntax_only(styles), Expression::ObjectExpression(_))
        })
        && let Some((tag_name, default_class_name)) = tag_from_argument(&call.arguments[0], imports)
    {
        return Some((tag_name, default_class_name, 1));
    }
    None
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
    attrs: &[Expression<'a>],
) -> (ExtractResult<'a>, Expression<'a>, Option<String>) {
    let mut composed_classes = Vec::new();
    let mut error = None;
    if let Expression::CallExpression(call) = expression
        && extract_base_tag_and_class_name(&call.callee, imports)
            .0
            .is_some()
    {
        match style_arguments(ast_builder, &call.arguments) {
            Some(StyleArguments { classes, rules }) => {
                call.arguments =
                    oxc_allocator::Vec::from_array_in([Argument::from(rules)], ast_builder);
                composed_classes = classes;
            }
            None if call.arguments.len() > 1
                || matches!(call.arguments.first(), Some(Argument::ArrayExpression(_))) =>
            {
                error = Some(uncomposable_error(&call.arguments));
            }
            None => {}
        }
    }
    let (result, new_expr) = if let Expression::TaggedTemplateExpression(tag) = expression
        && let (Some(tag_name), default_class_name) =
            extract_base_tag_and_class_name(&tag.tag, imports)
    {
        // Case 1: styled.div`css` or styled("div")`css`
        // Check if tag is styled.div or styled(...)
        // Extract CSS from template literal

        let TemplateStyles {
            styles,
            statements,
            unplaced,
        } = css_to_style_template(&tag.quasi, 0, &None);
        if let Some(index) = unplaced.first() {
            error = Some(unplaced_error(&tag.quasi.expressions[*index]));
        }
        let mut props_styles: Vec<ExtractStyleProp<'_>> = styles
            .into_iter()
            .map(|ex| ExtractStyleProp::Static(ex.into()))
            .collect();

        if let Some(default_class_name) = default_class_name {
            props_styles.extend(default_class_name.into_iter().map(ExtractStyleProp::Static));
        }

        let mixins = statements.into_iter().map(|index| {
            let mixin = &tag.quasi.expressions[index];
            if matches!(
                unwrap_syntax_only(mixin),
                Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
            ) {
                // A mixin returns a class, or `false` when its condition fails
                Expression::new_logical_expression(
                    SPAN,
                    wrap_direct_call(
                        ast_builder,
                        mixin,
                        &[Expression::new_identifier(SPAN, "rest", ast_builder)],
                    ),
                    LogicalOperator::Or,
                    Expression::new_string_literal(SPAN, "", None, ast_builder),
                    ast_builder,
                )
            } else {
                mixin.clone_in(ast_builder.allocator())
            }
        });
        let class_name = merge_expression_for_class_name(
            ast_builder,
            mixins
                .collect::<Vec<_>>()
                .into_iter()
                .chain(gen_class_names(
                    ast_builder,
                    &mut props_styles,
                    None,
                    split_filename,
                )),
        );
        let styled_component = apply_attrs(
            ast_builder,
            create_styled_component(
                ast_builder,
                &tag_name,
                &class_name,
                &gen_styles(ast_builder, &props_styles, None),
            ),
            attrs,
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
        && let Some((tag_name, default_class_name, style_index)) =
            resolve_styled_call_target(call, imports)
    {
        // Case 2: styled.div({ bg: "red" }), styled("div")({ bg: "red" }),
        // or styled("div", { bg: "red" })

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
            if let Argument::SpreadElement(spread) = &mut call.arguments[style_index] {
                &mut spread.argument
            } else {
                call.arguments[style_index].to_expression_mut()
            },
            0,
            &None,
            LiteralHandling::ExpandResponsiveThemeToken,
        );
        if let Some(default_class_name) = default_class_name {
            styles.extend(default_class_name.into_iter().map(ExtractStyleProp::Static));
        }

        let class_name = merge_expression_for_class_name(
            ast_builder,
            composed_classes.into_iter().chain(gen_class_names(
                ast_builder,
                &mut styles,
                style_order,
                split_filename,
            )),
        );
        let styled_component = apply_attrs(
            ast_builder,
            create_styled_component(
                ast_builder,
                &tag_name,
                &class_name,
                &gen_styles(ast_builder, &styles, None),
            ),
            attrs,
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
        error,
    )
}

/// The name the attrs wrapper binds props to, chosen not to shadow what the
/// attrs expressions read
const ATTRS_PROPS: &str = "__devupProps";

/// Strip styled-components' `.attrs()` / `.withConfig()` off a styled factory
/// such as `styled.div.attrs(a).withConfig(c)`, returning the attrs in the
/// order they apply. `withConfig` only tunes runtime behavior, so it is dropped.
pub fn take_styled_modifiers<'a>(
    ast_builder: &AstBuilder<'a>,
    factory: &mut Expression<'a>,
    is_styled: impl Fn(&str) -> bool,
) -> Vec<Expression<'a>> {
    let mut attrs = Vec::new();
    if !is_modified_styled(factory, is_styled) {
        return attrs;
    }
    while let Expression::CallExpression(call) = unwrap_syntax_only_mut(factory)
        && let CallExpression {
            callee, arguments, ..
        } = &mut **call
        && let Expression::StaticMemberExpression(member) = unwrap_syntax_only_mut(callee)
        && matches!(member.property.name.as_str(), "attrs" | "withConfig")
    {
        let placeholder = || Expression::new_null_literal(SPAN, ast_builder);
        if member.property.name == "attrs"
            && let Some(argument) = arguments[0].as_expression_mut()
        {
            attrs.push(std::mem::replace(argument, placeholder()));
        }
        let object = std::mem::replace(&mut member.object, placeholder());
        *factory = object;
    }
    attrs.reverse();
    attrs
}

fn is_modified_styled(expression: &Expression<'_>, is_styled: impl Fn(&str) -> bool) -> bool {
    let mut expression = unwrap_syntax_only(expression);
    let mut modified = false;
    while let Some(object) = modifier_object(expression) {
        expression = unwrap_syntax_only(object);
        modified = true;
    }
    modified
        && match expression {
            Expression::StaticMemberExpression(member) => {
                matches!(&member.object, Expression::Identifier(ident) if is_styled(&ident.name))
            }
            Expression::CallExpression(call) => {
                matches!(&call.callee, Expression::Identifier(ident) if is_styled(&ident.name))
            }
            _ => false,
        }
}

fn modifier_object<'b, 'a>(expression: &'b Expression<'a>) -> Option<&'b Expression<'a>> {
    if let Expression::CallExpression(call) = expression
        && let [argument] = call.arguments.as_slice()
        && argument.is_expression()
        && let Expression::StaticMemberExpression(member) = unwrap_syntax_only(&call.callee)
        && matches!(member.property.name.as_str(), "attrs" | "withConfig")
    {
        Some(&member.object)
    } else {
        None
    }
}

/// Wrap `component` so the attrs are merged over its props first: an object is
/// spread, anything else is called with the props when it is a function, as
/// styled-components does
fn apply_attrs<'a>(
    ast_builder: &AstBuilder<'a>,
    component: Expression<'a>,
    attrs: &[Expression<'a>],
) -> Expression<'a> {
    if attrs.is_empty() {
        return component;
    }
    let props = || Expression::new_identifier(SPAN, ATTRS_PROPS, ast_builder);
    let mut merged = props();
    for attr in attrs {
        let attr = attr.clone_in(ast_builder.allocator());
        if matches!(unwrap_syntax_only(&attr), Expression::ObjectExpression(_)) {
            merged = spread_objects(ast_builder, merged, attr);
            continue;
        }
        let called = wrap_direct_call(ast_builder, &attr, &[props()]);
        let resolved = if matches!(
            unwrap_syntax_only(&attr),
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
        ) {
            called
        } else {
            let is_function = Expression::new_binary_expression(
                SPAN,
                Expression::new_unary_expression(
                    SPAN,
                    UnaryOperator::Typeof,
                    attr.clone_in(ast_builder.allocator()),
                    ast_builder,
                ),
                BinaryOperator::StrictEquality,
                Expression::new_string_literal(SPAN, "function", None, ast_builder),
                ast_builder,
            );
            Expression::new_conditional_expression(SPAN, is_function, called, attr, ast_builder)
        };
        let step = props_arrow(ast_builder, spread_objects(ast_builder, props(), resolved));
        merged = wrap_direct_call(ast_builder, &step, &[merged]);
    }
    props_arrow(
        ast_builder,
        wrap_direct_call(ast_builder, &component, &[merged]),
    )
}

fn spread_objects<'a>(
    ast_builder: &AstBuilder<'a>,
    first: Expression<'a>,
    second: Expression<'a>,
) -> Expression<'a> {
    let mut properties = oxc_allocator::Vec::with_capacity_in(2, ast_builder);
    properties.push(ObjectPropertyKind::new_spread_property(
        SPAN,
        first,
        ast_builder,
    ));
    properties.push(ObjectPropertyKind::new_spread_property(
        SPAN,
        second,
        ast_builder,
    ));
    Expression::new_object_expression(SPAN, properties, ast_builder)
}

fn props_arrow<'a>(ast_builder: &AstBuilder<'a>, body: Expression<'a>) -> Expression<'a> {
    let parameter = FormalParameter::new(
        SPAN,
        oxc_allocator::Vec::new_in(ast_builder),
        BindingPattern::new_binding_identifier(SPAN, ATTRS_PROPS, ast_builder),
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        None::<oxc_allocator::Box<Expression<'a>>>,
        false,
        None,
        false,
        false,
        ast_builder,
    );
    let params = FormalParameters::boxed(
        SPAN,
        FormalParameterKind::ArrowFormalParameters,
        oxc_allocator::Vec::from_iter_in([parameter], ast_builder),
        None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
        ast_builder,
    );
    Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        params,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body.into(),
        ast_builder,
    )
}

fn create_styled_component<'a>(
    ast_builder: &AstBuilder<'a>,
    tag_name: &str,
    class_name: &Option<Expression<'a>>,
    style_vars: &Option<Expression<'a>>,
) -> Expression<'a> {
    let params = FormalParameters::boxed(
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
                    Some(BindingRestElement::boxed(
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
    let body = Expression::new_jsx_element(
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
                        JSXAttributeName::new_identifier(SPAN, "className", ast_builder),
                        Some(JSXAttributeValue::new_expression_container(
                            SPAN,
                            class_name
                                .as_ref()
                                .map_or_else(
                                    || Expression::new_identifier(SPAN, "className", ast_builder),
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
                                        .unwrap_or_else(|| name.clone_in(ast_builder.allocator()))
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
                                    || Expression::new_identifier(SPAN, "style", ast_builder),
                                    |style_vars| {
                                        merge_object_expressions(
                                            ast_builder,
                                            &[
                                                style_vars.clone_in(ast_builder.allocator()),
                                                Expression::new_identifier(
                                                    SPAN,
                                                    "style",
                                                    ast_builder,
                                                ),
                                            ],
                                        )
                                        .unwrap_or_else(
                                            || style_vars.clone_in(ast_builder.allocator()),
                                        )
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
    );
    Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        params,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body.into(),
        ast_builder,
    )
}
