use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::style_property::StyleProperty;
use crate::gen_class_name::gen_class_names;
use crate::gen_style::gen_styles;
use crate::tailwind::{has_tailwind_classes, parse_single_class, parse_tailwind_to_styles};
use crate::utils::{get_string_by_property_key, merge_object_expressions};
use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_allocator::CloneIn;
use oxc_ast::AstBuilder;
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Expression, JSXAttributeItem, JSXAttributeValue, LogicalOperator, ObjectPropertyKind,
    PropertyKey, PropertyKind, TemplateElementValue,
};
use oxc_span::SPAN;
use std::collections::HashMap;

/// Combine two optional className expressions into a conditional expression.
/// `condition ? con_expr : alt_expr`, falling back to `""` for the missing branch.
/// Returns `None` only when both branches are `None`.
pub(crate) fn combine_conditional_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    condition: Expression<'a>,
    con_expr: Option<Expression<'a>>,
    alt_expr: Option<Expression<'a>>,
) -> Option<Expression<'a>> {
    match (con_expr, alt_expr) {
        (Some(con), Some(alt)) => {
            Some(ast_builder.expression_conditional(SPAN, condition, con, alt))
        }
        (Some(con), None) => Some(ast_builder.expression_conditional(
            SPAN,
            condition,
            con,
            ast_builder.expression_string_literal(SPAN, "", None),
        )),
        (None, Some(alt)) => Some(ast_builder.expression_conditional(
            SPAN,
            condition,
            ast_builder.expression_string_literal(SPAN, "", None),
            alt,
        )),
        (None, None) => None,
    }
}

/// modify object props
/// Returns extracted Tailwind styles from static className strings
/// `conditional_branch`: If Some, contains (condition, alternate_styles, alternate_style_order)
///   for generating a conditional className expression: `condition ? consequent_class : alternate_class`
#[allow(clippy::too_many_arguments)]
pub fn modify_prop_object<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<ObjectPropertyKind<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    style_vars: Option<Expression<'a>>,
    props_prop: Option<Expression<'a>>,
    filename: Option<&str>,
    conditional_branch: Option<(Expression<'a>, &mut [ExtractStyleProp<'a>], Option<u8>)>,
) -> Vec<ExtractStyleValue> {
    let mut class_name_prop = None;
    let mut style_prop = None;
    let mut spread_props = vec![];
    for idx in (0..props.len()).rev() {
        let prop = props.remove(idx);
        match prop {
            ObjectPropertyKind::ObjectProperty(attr) => {
                if let Some(name) = get_string_by_property_key(&attr.key) {
                    if name == "className" {
                        class_name_prop = Some(attr.value.clone_in(ast_builder.allocator));
                    } else if name == "style" {
                        style_prop = Some(attr.value.clone_in(ast_builder.allocator));
                    } else {
                        props.insert(idx, ObjectPropertyKind::ObjectProperty(attr));
                    }
                } else {
                    props.insert(idx, ObjectPropertyKind::ObjectProperty(attr));
                }
            }
            ObjectPropertyKind::SpreadProperty(spread) => {
                spread_props.push(spread.argument.clone_in(ast_builder.allocator));
                props.insert(idx, ObjectPropertyKind::SpreadProperty(spread));
            }
        }
    }

    let (class_name_expr, tailwind_styles) =
        if let Some((condition, alt_styles, alt_style_order)) = conditional_branch {
            // Conditional styleOrder: generate className for both branches
            let (con_expr, con_tailwind) = get_class_name_expression(
                ast_builder,
                &class_name_prop,
                styles,
                style_order,
                &spread_props,
                filename,
            );
            let alt_class_name_prop = class_name_prop
                .as_ref()
                .map(|c| c.clone_in(ast_builder.allocator));
            let (alt_expr, alt_tailwind) = get_class_name_expression(
                ast_builder,
                &alt_class_name_prop,
                alt_styles,
                alt_style_order,
                &spread_props,
                filename,
            );

            let combined_expr =
                combine_conditional_class_name(ast_builder, condition, con_expr, alt_expr);

            let mut all_tailwind = con_tailwind;
            all_tailwind.extend(alt_tailwind);
            (combined_expr, all_tailwind)
        } else {
            get_class_name_expression(
                ast_builder,
                &class_name_prop,
                styles,
                style_order,
                &spread_props,
                filename,
            )
        };

    if let Some(ex) = class_name_expr {
        props.push(ast_builder.object_property_kind_object_property(
            SPAN,
            PropertyKind::Init,
            ast_builder.property_key_static_identifier(SPAN, "className"),
            ex,
            false,
            false,
            false,
        ));
    }
    if let Some(ex) = get_style_expression(
        ast_builder,
        &style_prop,
        styles,
        &style_vars,
        &spread_props,
        filename,
    ) {
        props.push(ast_builder.object_property_kind_object_property(
            SPAN,
            PropertyKind::Init,
            ast_builder.property_key_static_identifier(SPAN, "style"),
            ex,
            false,
            false,
            false,
        ));
    }
    if let Some(ex) = props_prop {
        props.push(
            ast_builder
                .object_property_kind_spread_property(SPAN, ex.clone_in(ast_builder.allocator)),
        );
    }
    tailwind_styles
}
/// modify JSX props
/// Returns extracted Tailwind styles from static className strings
/// `conditional_branch`: If Some, contains (condition, alternate_styles, alternate_style_order)
///   for generating a conditional className expression: `condition ? consequent_class : alternate_class`
#[allow(clippy::too_many_arguments)]
pub fn modify_props<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<JSXAttributeItem<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    style_vars: Option<Expression<'a>>,
    props_prop: Option<Expression<'a>>,
    filename: Option<&str>,
    conditional_branch: Option<(Expression<'a>, &mut [ExtractStyleProp<'a>], Option<u8>)>,
) -> Vec<ExtractStyleValue> {
    let mut class_name_prop = None;
    let mut style_prop = None;
    let mut spread_props = vec![];
    for idx in (0..props.len()).rev() {
        let prop = props.remove(idx);
        match prop {
            JSXAttributeItem::Attribute(attr) => {
                if let Identifier(ident) = &attr.name
                    && let Some(value) = &attr.value
                    && (ident.name == "className" || ident.name == "style")
                {
                    let mut res = None;

                    if let JSXAttributeValue::ExpressionContainer(container) = &value {
                        res = container
                            .expression
                            .as_expression()
                            .map(|expression| expression.clone_in(ast_builder.allocator));
                    } else if let JSXAttributeValue::StringLiteral(literal) = &value {
                        res =
                            Some(ast_builder.expression_string_literal(SPAN, literal.value, None));
                    }
                    let name = ident.name.as_str();
                    if name == "className" {
                        class_name_prop = res;
                    } else if name == "style" {
                        style_prop = res;
                    }
                } else {
                    props.insert(idx, JSXAttributeItem::Attribute(attr));
                }
            }
            JSXAttributeItem::SpreadAttribute(spread) => {
                spread_props.push(spread.argument.clone_in(ast_builder.allocator));
                props.insert(idx, JSXAttributeItem::SpreadAttribute(spread));
            }
        }
    }
    let (class_name_expr, tailwind_styles) =
        if let Some((condition, alt_styles, alt_style_order)) = conditional_branch {
            // Conditional styleOrder: generate className for both branches
            let (con_expr, con_tailwind) = get_class_name_expression(
                ast_builder,
                &class_name_prop,
                styles,
                style_order,
                &spread_props,
                filename,
            );
            let alt_class_name_prop = class_name_prop
                .as_ref()
                .map(|c| c.clone_in(ast_builder.allocator));
            let (alt_expr, alt_tailwind) = get_class_name_expression(
                ast_builder,
                &alt_class_name_prop,
                alt_styles,
                alt_style_order,
                &spread_props,
                filename,
            );

            let combined_expr =
                combine_conditional_class_name(ast_builder, condition, con_expr, alt_expr);

            let mut all_tailwind = con_tailwind;
            all_tailwind.extend(alt_tailwind);
            (combined_expr, all_tailwind)
        } else {
            get_class_name_expression(
                ast_builder,
                &class_name_prop,
                styles,
                style_order,
                &spread_props,
                filename,
            )
        };
    if let Some(ex) = class_name_expr {
        props.push(ast_builder.jsx_attribute_item_attribute(
            SPAN,
            ast_builder.jsx_attribute_name_identifier(SPAN, "className"),
            Some(if let Expression::StringLiteral(literal) = ex {
                JSXAttributeValue::StringLiteral(literal)
            } else {
                ast_builder.jsx_attribute_value_expression_container(SPAN, ex.into())
            }),
        ));
    }
    if let Some(ex) = get_style_expression(
        ast_builder,
        &style_prop,
        styles,
        &style_vars,
        &spread_props,
        filename,
    ) {
        props.push(ast_builder.jsx_attribute_item_attribute(
            SPAN,
            ast_builder.jsx_attribute_name_identifier(SPAN, "style"),
            Some(ast_builder.jsx_attribute_value_expression_container(SPAN, ex.into())),
        ));
    }
    if let Some(props_prop) = props_prop {
        props.push(
            ast_builder.jsx_attribute_item_spread_attribute(
                SPAN,
                props_prop.clone_in(ast_builder.allocator),
            ),
        );
    }
    tailwind_styles
}

/// Returns (className expression, extracted Tailwind styles)
pub fn get_class_name_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name_prop: &Option<Expression<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    spread_props: &[Expression<'a>],
    filename: Option<&str>,
) -> (Option<Expression<'a>>, Vec<ExtractStyleValue>) {
    // Extract Tailwind styles from static className strings and generate class names
    let (tailwind_styles, tailwind_class_expr) =
        extract_tailwind_from_class_name(ast_builder, class_name_prop, style_order, filename);

    // Determine the className expression to use:
    // - If we extracted Tailwind styles, use generated class names (replace original)
    // - Otherwise, preserve the original className
    let class_name_to_use = if tailwind_class_expr.is_some() {
        // Tailwind className → replaced with generated class names
        tailwind_class_expr
    } else {
        // Non-Tailwind className → keep original
        class_name_prop
            .as_ref()
            .map(|class_name| convert_class_name(ast_builder, class_name))
    };

    // Merge class names: [tailwind/original class names] + [devup-ui component styles]
    let expression = merge_string_expressions(
        ast_builder,
        [
            class_name_to_use,
            gen_class_names(ast_builder, styles, style_order, filename),
        ]
        .into_iter()
        .flatten()
        .chain(if class_name_prop.is_some() {
            vec![]
        } else {
            spread_props
                .iter()
                .map(|ex| {
                    convert_class_name(
                        ast_builder,
                        &Expression::StaticMemberExpression(
                            ast_builder.alloc_static_member_expression(
                                SPAN,
                                ex.clone_in(ast_builder.allocator),
                                ast_builder.identifier_name(SPAN, ast_builder.atom("className")),
                                true,
                            ),
                        ),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .as_slice(),
    );

    (expression, tailwind_styles)
}

/// Apply style_order to all ExtractStyleValue items
fn apply_style_order_to_styles(styles: &mut [ExtractStyleValue], style_order: Option<u8>) {
    if let Some(order) = style_order {
        for style in styles.iter_mut() {
            style.set_style_order(order);
        }
    }
}

/// Extract Tailwind CSS styles from a static className string and generate devup-ui class names
/// Returns (extracted styles for CSS generation, generated class names expression)
fn extract_tailwind_from_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name_prop: &Option<Expression<'a>>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> (Vec<ExtractStyleValue>, Option<Expression<'a>>) {
    // Extract from static string literals
    if let Some(Expression::StringLiteral(literal)) = class_name_prop {
        let class_str = literal.value.as_str();
        if has_tailwind_classes(class_str) {
            let mut tailwind_styles = parse_tailwind_to_styles(class_str, filename);
            if !tailwind_styles.is_empty() {
                // Apply style_order to all extracted Tailwind styles
                apply_style_order_to_styles(&mut tailwind_styles, style_order);

                // Convert ExtractStyleValue to ExtractStyleProp::Static for gen_class_names
                let mut tailwind_style_props: Vec<ExtractStyleProp> = tailwind_styles
                    .iter()
                    .cloned()
                    .map(ExtractStyleProp::Static)
                    .collect();

                // Generate devup-ui class names for the Tailwind styles
                let class_names_expr = gen_class_names(
                    ast_builder,
                    &mut tailwind_style_props,
                    style_order,
                    filename,
                );

                return (tailwind_styles, class_names_expr);
            }
        }
    }

    // Extract from template literals (e.g., `${cond ? 'text-red' : 'text-blue'} p-4`)
    if let Some(Expression::TemplateLiteral(template)) = class_name_prop {
        let all_classes = extract_all_classes_from_template_literal(template);
        if has_tailwind_classes(&all_classes) {
            // Build mapping from Tailwind class → generated class name
            let class_mapping = build_tailwind_class_mapping(&all_classes, style_order, filename);

            if !class_mapping.is_empty() {
                // Collect all styles for CSS generation
                let mut tailwind_styles = parse_tailwind_to_styles(&all_classes, filename);
                // Apply style_order to all extracted Tailwind styles
                apply_style_order_to_styles(&mut tailwind_styles, style_order);

                // Build new template literal with replaced class names
                let new_template =
                    rebuild_template_literal_with_mapping(ast_builder, template, &class_mapping);

                return (tailwind_styles, Some(new_template));
            }
        }
    }

    (Vec::new(), None)
}

/// Build a mapping from Tailwind class name to generated devup-ui class name
fn build_tailwind_class_mapping(
    class_str: &str,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> HashMap<String, String> {
    let mut mapping = HashMap::new();

    for class in class_str.split_whitespace() {
        if let Some(parsed) = parse_single_class(class) {
            let mut static_style = parsed.to_static_style();
            if let Some(order) = style_order {
                static_style.style_order = Some(order);
            }
            // Extract to get the generated class name
            if let StyleProperty::ClassName(generated) = static_style.extract(filename) {
                mapping.insert(class.to_string(), generated);
            }
        }
    }

    mapping
}

/// Rebuild a template literal, replacing Tailwind classes with generated class names
fn rebuild_template_literal_with_mapping<'a>(
    ast_builder: &AstBuilder<'a>,
    template: &oxc_ast::ast::TemplateLiteral<'a>,
    class_mapping: &HashMap<String, String>,
) -> Expression<'a> {
    // Rebuild quasis with replaced class names
    let new_quasis = template.quasis.iter().map(|quasi| {
        let raw = quasi.value.raw.as_str();
        let replaced = replace_classes_in_string(raw, class_mapping);
        let cooked = quasi.value.cooked.as_ref().map(|c| {
            let replaced_cooked = replace_classes_in_string(c.as_str(), class_mapping);
            ast_builder.atom(&replaced_cooked)
        });
        ast_builder.template_element(
            quasi.span,
            TemplateElementValue {
                raw: ast_builder.atom(&replaced),
                cooked,
            },
            quasi.tail,
            false, // escape_raw
        )
    });

    // Rebuild expressions with replaced class names
    let new_expressions = template
        .expressions
        .iter()
        .map(|expr| rebuild_expression_with_mapping(ast_builder, expr, class_mapping));

    ast_builder.expression_template_literal(
        template.span,
        oxc_allocator::Vec::from_iter_in(new_quasis, ast_builder.allocator),
        oxc_allocator::Vec::from_iter_in(new_expressions, ast_builder.allocator),
    )
}

/// Replace Tailwind class names in a string with generated class names
fn replace_classes_in_string(s: &str, class_mapping: &HashMap<String, String>) -> String {
    let mut result = s.to_string();
    // Sort by length descending to avoid partial replacements (e.g., "text-3xl" before "text-3")
    let mut sorted_classes: Vec<_> = class_mapping.iter().collect();
    sorted_classes.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    for (tailwind_class, generated_class) in sorted_classes {
        result = result.replace(tailwind_class, generated_class);
    }
    result
}

/// Rebuild an expression, replacing Tailwind classes in string literals
fn rebuild_expression_with_mapping<'a>(
    ast_builder: &AstBuilder<'a>,
    expr: &Expression<'a>,
    class_mapping: &HashMap<String, String>,
) -> Expression<'a> {
    match expr {
        Expression::StringLiteral(lit) => {
            let replaced = replace_classes_in_string(lit.value.as_str(), class_mapping);
            ast_builder.expression_string_literal(SPAN, ast_builder.atom(&replaced), None)
        }
        Expression::ConditionalExpression(cond) => {
            let consequent =
                rebuild_expression_with_mapping(ast_builder, &cond.consequent, class_mapping);
            let alternate =
                rebuild_expression_with_mapping(ast_builder, &cond.alternate, class_mapping);
            ast_builder.expression_conditional(
                cond.span,
                cond.test.clone_in(ast_builder.allocator),
                consequent,
                alternate,
            )
        }
        Expression::LogicalExpression(logic) => {
            let left = rebuild_expression_with_mapping(ast_builder, &logic.left, class_mapping);
            let right = rebuild_expression_with_mapping(ast_builder, &logic.right, class_mapping);
            ast_builder.expression_logical(logic.span, left, logic.operator, right)
        }
        Expression::ParenthesizedExpression(paren) => {
            let inner =
                rebuild_expression_with_mapping(ast_builder, &paren.expression, class_mapping);
            ast_builder.expression_parenthesized(paren.span, inner)
        }
        Expression::TemplateLiteral(inner_template) => {
            rebuild_template_literal_with_mapping(ast_builder, inner_template, class_mapping)
        }
        // For other expressions (variables, etc.), keep as-is
        _ => expr.clone_in(ast_builder.allocator),
    }
}

/// Extract all class name strings from a template literal, including from conditional expressions
fn extract_all_classes_from_template_literal(template: &oxc_ast::ast::TemplateLiteral) -> String {
    let mut classes = Vec::new();

    // Extract from quasis (static parts of template literal)
    for quasi in &template.quasis {
        let raw = quasi.value.raw.as_str();
        if !raw.trim().is_empty() {
            classes.push(raw.trim().to_string());
        }
    }

    // Extract from expressions (dynamic parts)
    for expr in &template.expressions {
        extract_classes_from_expression(expr, &mut classes);
    }

    classes.join(" ")
}

/// Recursively extract class name strings from an expression
fn extract_classes_from_expression(expr: &Expression, classes: &mut Vec<String>) {
    match expr {
        // Direct string literal: 'text-red-500'
        Expression::StringLiteral(lit) => {
            let value = lit.value.as_str().trim();
            if !value.is_empty() {
                classes.push(value.to_string());
            }
        }
        // Ternary/conditional: cond ? 'text-red' : 'text-blue'
        Expression::ConditionalExpression(cond) => {
            extract_classes_from_expression(&cond.consequent, classes);
            extract_classes_from_expression(&cond.alternate, classes);
        }
        // Logical OR: value || 'fallback'
        Expression::LogicalExpression(logic) => {
            extract_classes_from_expression(&logic.left, classes);
            extract_classes_from_expression(&logic.right, classes);
        }
        // Parenthesized expression: (expr)
        Expression::ParenthesizedExpression(paren) => {
            extract_classes_from_expression(&paren.expression, classes);
        }
        // Template literal inside expression
        Expression::TemplateLiteral(inner_template) => {
            let inner_classes = extract_all_classes_from_template_literal(inner_template);
            if !inner_classes.is_empty() {
                classes.push(inner_classes);
            }
        }
        // Other expressions (variables, function calls, etc.) - skip, can't extract statically
        _ => {}
    }
}

pub fn get_style_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &Option<Expression<'a>>,
    styles: &[ExtractStyleProp<'a>],
    style_vars: &Option<Expression<'a>>,
    spread_props: &[Expression<'a>],
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    merge_object_expressions(
        ast_builder,
        [
            gen_styles(ast_builder, styles, filename),
            style_vars
                .as_ref()
                .map(|style_vars| convert_style_vars(ast_builder, style_vars)),
            style_prop.clone_in(ast_builder.allocator),
        ]
        .into_iter()
        .flatten()
        .chain(if style_prop.is_some() {
            vec![]
        } else {
            spread_props
                .iter()
                .map(|ex| {
                    Expression::StaticMemberExpression(ast_builder.alloc_static_member_expression(
                        SPAN,
                        ex.clone_in(ast_builder.allocator),
                        ast_builder.identifier_name(SPAN, ast_builder.atom("style")),
                        true,
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
        .as_slice(),
    )
}

fn merge_string_expressions<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: &[Expression<'a>],
) -> Option<Expression<'a>> {
    if expressions.is_empty() {
        return None;
    }
    if expressions.len() == 1
        && !matches!(
            expressions.first().unwrap(),
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
        )
    {
        return Some(expressions.first().unwrap().clone_in(ast_builder.allocator));
    }

    let mut string_literals: std::vec::Vec<String> = vec![];
    let mut other_expressions = vec![];
    let mut prev_str = String::new();
    for ex in expressions.iter() {
        if let Expression::StringLiteral(literal) = ex {
            let target_prev = prev_str.trim();
            let target = literal.value.trim();
            prev_str = format!(
                "{}{}{}",
                target_prev,
                if target_prev.is_empty() { "" } else { " " },
                target
            );
        } else if let Expression::TemplateLiteral(template) = ex {
            for (idx, q) in template.quasis.iter().enumerate() {
                let target_prev = prev_str.trim();
                let target = q.value.raw.trim();
                if idx < template.quasis.len() - 1 {
                    string_literals.push(format!(
                        "{}{}{}{}{}",
                        if !other_expressions.is_empty() || idx > 0 {
                            " "
                        } else {
                            ""
                        },
                        target_prev,
                        if !target_prev.is_empty() { " " } else { "" },
                        target,
                        if !target.is_empty() && !target.ends_with("typo-") {
                            " "
                        } else {
                            ""
                        }
                    ));
                } else {
                    prev_str = q.value.raw.trim().to_string();
                }
            }
            other_expressions.extend(template.expressions.clone_in(ast_builder.allocator));
        } else {
            let target_prev = prev_str.trim();
            string_literals.push(format!(
                "{}{}{}",
                if !other_expressions.is_empty() {
                    " "
                } else {
                    ""
                },
                target_prev,
                if !target_prev.is_empty() { " " } else { "" }
            ));
            other_expressions.push(ex.clone_in(ast_builder.allocator));
            prev_str = String::new();
        }
    }
    string_literals.push(format!(
        "{}{}",
        if !prev_str.trim().is_empty() { " " } else { "" },
        prev_str.trim(),
    ));
    if other_expressions.is_empty() {
        return Some(ast_builder.expression_string_literal(
            SPAN,
            ast_builder.atom(string_literals.join("").trim()),
            None,
        ));
    }

    let q = oxc_allocator::Vec::from_iter_in(
        string_literals.iter().enumerate().map(|(idx, s)| {
            let tail = idx == string_literals.len() - 1;
            ast_builder.template_element(
                SPAN,
                TemplateElementValue {
                    raw: ast_builder.atom(s),
                    cooked: None,
                },
                tail,
                false,
            )
        }),
        ast_builder.allocator,
    );
    Some(
        ast_builder.expression_template_literal(
            SPAN,
            q,
            oxc_allocator::Vec::from_iter_in(
                other_expressions
                    .into_iter()
                    .map(|ex| ex.clone_in(ast_builder.allocator)),
                ast_builder.allocator,
            ),
        ),
    )
}

pub fn convert_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name: &Expression<'a>,
) -> Expression<'a> {
    if matches!(
        class_name,
        Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::NumericLiteral(_)
    ) {
        return class_name.clone_in(ast_builder.allocator);
    }

    // wrap ( and ?? ''
    ast_builder.expression_logical(
        SPAN,
        ast_builder.expression_parenthesized(SPAN, class_name.clone_in(ast_builder.allocator)),
        LogicalOperator::Or,
        ast_builder.expression_string_literal(SPAN, "", None),
    )
}

pub fn convert_style_vars<'a>(
    ast_builder: &AstBuilder<'a>,
    style_vars: &Expression<'a>,
) -> Expression<'a> {
    let mut style_vars = style_vars.clone_in(ast_builder.allocator);
    if let Expression::ObjectExpression(obj) = &mut style_vars {
        for idx in (0..obj.properties.len()).rev() {
            let mut prop = obj.properties.remove(idx);

            if let ObjectPropertyKind::ObjectProperty(p) = &mut prop {
                let name = if let Some(name) = get_string_by_property_key(&p.key) {
                    Some(name)
                } else {
                    obj.properties.insert(
                        idx,
                        ast_builder.object_property_kind_object_property(
                            SPAN,
                            PropertyKind::Init,
                            PropertyKey::TemplateLiteral(ast_builder.alloc_template_literal(
                                SPAN,
                                oxc_allocator::Vec::from_array_in(
                                    [
                                        ast_builder.template_element(
                                            SPAN,
                                            TemplateElementValue {
                                                raw: ast_builder.atom("--"),
                                                cooked: None,
                                            },
                                            false,
                                            false,
                                        ),
                                        ast_builder.template_element(
                                            SPAN,
                                            TemplateElementValue {
                                                raw: ast_builder.atom(""),
                                                cooked: None,
                                            },
                                            true,
                                            false,
                                        ),
                                    ],
                                    ast_builder.allocator,
                                ),
                                oxc_allocator::Vec::from_array_in(
                                    [p.key.to_expression().clone_in(ast_builder.allocator)],
                                    ast_builder.allocator,
                                ),
                            )),
                            p.value.clone_in(ast_builder.allocator),
                            false,
                            false,
                            true,
                        ),
                    );
                    None
                };

                if let Some(name) = name {
                    if !name.starts_with("--") {
                        p.key = PropertyKey::StringLiteral(ast_builder.alloc_string_literal(
                            SPAN,
                            ast_builder.atom(&format!("--{name}")),
                            None,
                        ));
                    }
                    obj.properties.insert(idx, prop);
                }
            } else {
                obj.properties.insert(idx, prop);
            }
        }
    }
    style_vars
}
