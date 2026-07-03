use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::style_property::StyleProperty;
use crate::gen_class_name::gen_class_names;
use crate::gen_style::gen_styles;
use crate::tailwind::{has_tailwind_classes, parse_single_class, parse_tailwind_to_styles};
use crate::utils::{get_string_by_property_key, merge_object_expressions};
use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Expression, IdentifierName, JSXAttributeItem, JSXAttributeName, JSXAttributeValue,
    LogicalOperator, ObjectPropertyKind, PropertyKey, PropertyKind, StaticMemberExpression, Str,
    StringLiteral, TemplateElement, TemplateElementValue, TemplateLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use rustc_hash::FxHashMap;
use std::borrow::Cow;

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
        (Some(con), Some(alt)) => Some(Expression::new_conditional_expression(
            SPAN,
            condition,
            con,
            alt,
            ast_builder,
        )),
        (Some(con), None) => Some(Expression::new_conditional_expression(
            SPAN,
            condition,
            con,
            Expression::new_string_literal(SPAN, "", None, ast_builder),
            ast_builder,
        )),
        (None, Some(alt)) => Some(Expression::new_conditional_expression(
            SPAN,
            condition,
            Expression::new_string_literal(SPAN, "", None, ast_builder),
            alt,
            ast_builder,
        )),
        (None, None) => None,
    }
}

/// Resolve the final className expression (and extracted Tailwind styles),
/// handling the optional conditional styleOrder branch:
/// `condition ? consequent_class : alternate_class`.
fn resolve_class_name_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name_prop: &Option<Expression<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    spread_props: &[Expression<'a>],
    filename: Option<&str>,
    conditional_branch: Option<(Expression<'a>, &mut [ExtractStyleProp<'a>], Option<u8>)>,
) -> (Option<Expression<'a>>, Vec<ExtractStyleValue>) {
    if let Some((condition, alt_styles, alt_style_order)) = conditional_branch {
        // Conditional styleOrder: generate className for both branches
        let (con_expr, con_tailwind) = get_class_name_expression(
            ast_builder,
            class_name_prop,
            styles,
            style_order,
            spread_props,
            filename,
        );
        let alt_class_name_prop = class_name_prop
            .as_ref()
            .map(|c| c.clone_in(ast_builder.allocator()));
        let (alt_expr, alt_tailwind) = get_class_name_expression(
            ast_builder,
            &alt_class_name_prop,
            alt_styles,
            alt_style_order,
            spread_props,
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
            class_name_prop,
            styles,
            style_order,
            spread_props,
            filename,
        )
    }
}

/// modify object props
/// Returns extracted Tailwind styles from static className strings
/// `conditional_branch`: If Some, contains (condition, `alternate_styles`, `alternate_style_order`)
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
                        class_name_prop = Some(attr.value.clone_in(ast_builder.allocator()));
                    } else if name == "style" {
                        style_prop = Some(attr.value.clone_in(ast_builder.allocator()));
                    } else {
                        props.insert(idx, ObjectPropertyKind::ObjectProperty(attr));
                    }
                } else {
                    props.insert(idx, ObjectPropertyKind::ObjectProperty(attr));
                }
            }
            ObjectPropertyKind::SpreadProperty(spread) => {
                spread_props.push(spread.argument.clone_in(ast_builder.allocator()));
                props.insert(idx, ObjectPropertyKind::SpreadProperty(spread));
            }
        }
    }

    let (class_name_expr, tailwind_styles) = resolve_class_name_expression(
        ast_builder,
        &class_name_prop,
        styles,
        style_order,
        &spread_props,
        filename,
        conditional_branch,
    );

    if let Some(ex) = class_name_expr {
        props.push(ObjectPropertyKind::new_object_property(
            SPAN,
            PropertyKind::Init,
            PropertyKey::new_static_identifier(SPAN, "className", ast_builder),
            ex,
            false,
            false,
            false,
            ast_builder,
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
        props.push(ObjectPropertyKind::new_object_property(
            SPAN,
            PropertyKind::Init,
            PropertyKey::new_static_identifier(SPAN, "style", ast_builder),
            ex,
            false,
            false,
            false,
            ast_builder,
        ));
    }
    if let Some(ex) = props_prop {
        props.push(ObjectPropertyKind::new_spread_property(
            SPAN,
            ex.clone_in(ast_builder.allocator()),
            ast_builder,
        ));
    }
    tailwind_styles
}
/// modify JSX props
/// Returns extracted Tailwind styles from static className strings
/// `conditional_branch`: If Some, contains (condition, `alternate_styles`, `alternate_style_order`)
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
                            .map(|expression| expression.clone_in(ast_builder.allocator()));
                    } else if let JSXAttributeValue::StringLiteral(literal) = &value {
                        res = Some(Expression::new_string_literal(
                            SPAN,
                            literal.value,
                            None,
                            ast_builder,
                        ));
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
                spread_props.push(spread.argument.clone_in(ast_builder.allocator()));
                props.insert(idx, JSXAttributeItem::SpreadAttribute(spread));
            }
        }
    }
    let (class_name_expr, tailwind_styles) = resolve_class_name_expression(
        ast_builder,
        &class_name_prop,
        styles,
        style_order,
        &spread_props,
        filename,
        conditional_branch,
    );
    if let Some(ex) = class_name_expr {
        props.push(JSXAttributeItem::new_attribute(
            SPAN,
            JSXAttributeName::new_identifier(SPAN, "className", ast_builder),
            Some(if let Expression::StringLiteral(literal) = ex {
                JSXAttributeValue::StringLiteral(literal)
            } else {
                JSXAttributeValue::new_expression_container(SPAN, ex.into(), ast_builder)
            }),
            ast_builder,
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
        props.push(JSXAttributeItem::new_attribute(
            SPAN,
            JSXAttributeName::new_identifier(SPAN, "style", ast_builder),
            Some(JSXAttributeValue::new_expression_container(
                SPAN,
                ex.into(),
                ast_builder,
            )),
            ast_builder,
        ));
    }
    if let Some(props_prop) = props_prop {
        props.push(JSXAttributeItem::new_spread_attribute(
            SPAN,
            props_prop.clone_in(ast_builder.allocator()),
            ast_builder,
        ));
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
    let mut class_expressions = Vec::with_capacity(2 + spread_props.len());
    if let Some(class_name) = class_name_to_use {
        class_expressions.push(class_name);
    }
    if let Some(class_name) = gen_class_names(ast_builder, styles, style_order, filename) {
        class_expressions.push(class_name);
    }
    if class_name_prop.is_none() {
        class_expressions.extend(spread_props.iter().map(|ex| {
            convert_class_name(
                ast_builder,
                &Expression::StaticMemberExpression(StaticMemberExpression::boxed(
                    SPAN,
                    ex.clone_in(ast_builder.allocator()),
                    IdentifierName::new(SPAN, "className", ast_builder),
                    true,
                    ast_builder,
                )),
            )
        }));
    }
    let expression = merge_string_expressions(ast_builder, &class_expressions);

    (expression, tailwind_styles)
}

/// Apply `style_order` to all `ExtractStyleValue` items
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
            let mut tailwind_styles = parse_tailwind_to_styles(class_str);
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
                let mut tailwind_styles = parse_tailwind_to_styles(&all_classes);
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
) -> FxHashMap<String, String> {
    let mut mapping = FxHashMap::default();

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
    class_mapping: &FxHashMap<String, String>,
) -> Expression<'a> {
    // Sort the mapping ONCE by key length descending (avoids partial replacements,
    // e.g. "text-3xl" before "text-3") and reuse the sorted slice for every quasi
    // and nested expression instead of re-sorting per call.
    let mut sorted_classes: Vec<(&String, &String)> = class_mapping.iter().collect();
    sorted_classes.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
    rebuild_template_literal_with_sorted(ast_builder, template, &sorted_classes)
}

/// Rebuild a template literal using a pre-sorted class mapping slice.
fn rebuild_template_literal_with_sorted<'a>(
    ast_builder: &AstBuilder<'a>,
    template: &oxc_ast::ast::TemplateLiteral<'a>,
    sorted_classes: &[(&String, &String)],
) -> Expression<'a> {
    // Rebuild quasis with replaced class names
    let new_quasis = template.quasis.iter().map(|quasi| {
        let raw = quasi.value.raw.as_str();
        let replaced = replace_classes_in_string(raw, sorted_classes);
        let cooked = quasi.value.cooked.as_ref().map(|c| {
            let replaced_cooked = replace_classes_in_string(c.as_str(), sorted_classes);
            Str::from_in(&replaced_cooked, ast_builder.allocator())
        });
        TemplateElement::new(
            quasi.span,
            TemplateElementValue {
                raw: Str::from_in(&replaced, ast_builder.allocator()),
                cooked,
            },
            quasi.tail,
            ast_builder,
        )
    });

    // Rebuild expressions with replaced class names
    let new_expressions = template
        .expressions
        .iter()
        .map(|expr| rebuild_expression_with_mapping(ast_builder, expr, sorted_classes));

    Expression::new_template_literal(
        template.span,
        oxc_allocator::Vec::from_iter_in(new_quasis, ast_builder),
        oxc_allocator::Vec::from_iter_in(new_expressions, ast_builder),
        ast_builder,
    )
}

/// Replace Tailwind class names in a string with generated class names.
///
/// `sorted_classes` MUST already be sorted by key length descending so longer
/// class names are replaced before their prefixes (e.g. "text-3xl" before "text-3").
fn replace_classes_in_string(s: &str, sorted_classes: &[(&String, &String)]) -> String {
    let mut result = Cow::Borrowed(s);
    for (tailwind_class, generated_class) in sorted_classes {
        if result.contains(tailwind_class.as_str()) {
            result = Cow::Owned(result.replace(tailwind_class.as_str(), generated_class));
        }
    }
    result.into_owned()
}

/// Rebuild an expression, replacing Tailwind classes in string literals
fn rebuild_expression_with_mapping<'a>(
    ast_builder: &AstBuilder<'a>,
    expr: &Expression<'a>,
    sorted_classes: &[(&String, &String)],
) -> Expression<'a> {
    match expr {
        Expression::StringLiteral(lit) => {
            let replaced = replace_classes_in_string(lit.value.as_str(), sorted_classes);
            Expression::new_string_literal(
                SPAN,
                Str::from_in(&replaced, ast_builder.allocator()),
                None,
                ast_builder,
            )
        }
        Expression::ConditionalExpression(cond) => {
            let consequent =
                rebuild_expression_with_mapping(ast_builder, &cond.consequent, sorted_classes);
            let alternate =
                rebuild_expression_with_mapping(ast_builder, &cond.alternate, sorted_classes);
            Expression::new_conditional_expression(
                cond.span,
                cond.test.clone_in(ast_builder.allocator()),
                consequent,
                alternate,
                ast_builder,
            )
        }
        Expression::LogicalExpression(logic) => {
            let left = rebuild_expression_with_mapping(ast_builder, &logic.left, sorted_classes);
            let right = rebuild_expression_with_mapping(ast_builder, &logic.right, sorted_classes);
            Expression::new_logical_expression(logic.span, left, logic.operator, right, ast_builder)
        }
        Expression::ParenthesizedExpression(paren) => {
            let inner =
                rebuild_expression_with_mapping(ast_builder, &paren.expression, sorted_classes);
            Expression::new_parenthesized_expression(paren.span, inner, ast_builder)
        }
        Expression::TemplateLiteral(inner_template) => {
            rebuild_template_literal_with_sorted(ast_builder, inner_template, sorted_classes)
        }
        // For other expressions (variables, etc.), keep as-is
        _ => expr.clone_in(ast_builder.allocator()),
    }
}

/// Extract all class name strings from a template literal, including from conditional expressions
fn extract_all_classes_from_template_literal(template: &oxc_ast::ast::TemplateLiteral) -> String {
    let mut classes = String::new();

    // Extract from quasis (static parts of template literal)
    for quasi in &template.quasis {
        let raw = quasi.value.raw.as_str();
        push_class_segment(&mut classes, raw.trim());
    }

    // Extract from expressions (dynamic parts)
    for expr in &template.expressions {
        extract_classes_from_expression(expr, &mut classes);
    }

    classes
}

fn push_class_segment(classes: &mut String, value: &str) {
    if value.is_empty() {
        return;
    }
    if !classes.is_empty() {
        classes.push(' ');
    }
    classes.push_str(value);
}

/// Recursively extract class name strings from an expression
fn extract_classes_from_expression(expr: &Expression, classes: &mut String) {
    match expr {
        // Direct string literal: 'text-red-500'
        Expression::StringLiteral(lit) => {
            let value = lit.value.as_str().trim();
            push_class_segment(classes, value);
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
            push_class_segment(classes, &inner_classes);
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
    let mut style_expressions = Vec::with_capacity(3 + spread_props.len());
    if let Some(style) = gen_styles(ast_builder, styles, filename) {
        style_expressions.push(style);
    }
    if let Some(style_vars) = style_vars
        .as_ref()
        .map(|style_vars| convert_style_vars(ast_builder, style_vars))
    {
        style_expressions.push(style_vars);
    }
    if let Some(style_prop) = style_prop.clone_in(ast_builder.allocator()) {
        style_expressions.push(style_prop);
    }
    if style_prop.is_none() {
        style_expressions.extend(spread_props.iter().map(|ex| {
            Expression::StaticMemberExpression(StaticMemberExpression::boxed(
                SPAN,
                ex.clone_in(ast_builder.allocator()),
                IdentifierName::new(SPAN, "style", ast_builder),
                true,
                ast_builder,
            ))
        }));
    }

    merge_object_expressions(ast_builder, &style_expressions)
}

fn merge_string_expressions<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: &[Expression<'a>],
) -> Option<Expression<'a>> {
    if expressions.is_empty() {
        return None;
    }
    if let [expression] = expressions
        && !matches!(
            expression,
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
        )
    {
        return Some(expression.clone_in(ast_builder.allocator()));
    }

    let mut string_literals: std::vec::Vec<String> = vec![];
    let mut other_expressions = vec![];
    let mut prev_str = String::new();
    for ex in expressions {
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
                        if target_prev.is_empty() { "" } else { " " },
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
            other_expressions.extend(template.expressions.clone_in(ast_builder.allocator()));
        } else {
            let target_prev = prev_str.trim();
            string_literals.push(format!(
                "{}{}{}",
                if other_expressions.is_empty() {
                    ""
                } else {
                    " "
                },
                target_prev,
                if target_prev.is_empty() { "" } else { " " }
            ));
            other_expressions.push(ex.clone_in(ast_builder.allocator()));
            prev_str = String::new();
        }
    }
    string_literals.push(format!(
        "{}{}",
        if prev_str.trim().is_empty() { "" } else { " " },
        prev_str.trim(),
    ));
    if other_expressions.is_empty() {
        return Some(Expression::new_string_literal(
            SPAN,
            Str::from_in(string_literals.join("").trim(), ast_builder.allocator()),
            None,
            ast_builder,
        ));
    }

    let q = oxc_allocator::Vec::from_iter_in(
        string_literals.iter().enumerate().map(|(idx, s)| {
            let tail = idx == string_literals.len() - 1;
            TemplateElement::new(
                SPAN,
                TemplateElementValue {
                    raw: Str::from_in(s, ast_builder.allocator()),
                    cooked: None,
                },
                tail,
                ast_builder,
            )
        }),
        ast_builder,
    );
    Some(Expression::new_template_literal(
        SPAN,
        q,
        oxc_allocator::Vec::from_iter_in(
            other_expressions
                .into_iter()
                .map(|ex| ex.clone_in(ast_builder.allocator())),
            ast_builder,
        ),
        ast_builder,
    ))
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
        return class_name.clone_in(ast_builder.allocator());
    }

    // wrap ( and ?? ''
    Expression::new_logical_expression(
        SPAN,
        Expression::new_parenthesized_expression(
            SPAN,
            class_name.clone_in(ast_builder.allocator()),
            ast_builder,
        ),
        LogicalOperator::Or,
        Expression::new_string_literal(SPAN, "", None, ast_builder),
        ast_builder,
    )
}

pub fn convert_style_vars<'a>(
    ast_builder: &AstBuilder<'a>,
    style_vars: &Expression<'a>,
) -> Expression<'a> {
    let mut style_vars = style_vars.clone_in(ast_builder.allocator());
    if let Expression::ObjectExpression(obj) = &mut style_vars {
        for idx in (0..obj.properties.len()).rev() {
            let mut prop = obj.properties.remove(idx);

            if let ObjectPropertyKind::ObjectProperty(p) = &mut prop {
                let name = if let Some(name) = get_string_by_property_key(&p.key) {
                    Some(name)
                } else {
                    obj.properties.insert(
                        idx,
                        ObjectPropertyKind::new_object_property(
                            SPAN,
                            PropertyKind::Init,
                            PropertyKey::TemplateLiteral(TemplateLiteral::boxed(
                                SPAN,
                                oxc_allocator::Vec::from_array_in(
                                    [
                                        TemplateElement::new(
                                            SPAN,
                                            TemplateElementValue {
                                                raw: Str::from("--"),
                                                cooked: None,
                                            },
                                            false,
                                            ast_builder,
                                        ),
                                        TemplateElement::new(
                                            SPAN,
                                            TemplateElementValue {
                                                raw: Str::from(""),
                                                cooked: None,
                                            },
                                            true,
                                            ast_builder,
                                        ),
                                    ],
                                    ast_builder,
                                ),
                                oxc_allocator::Vec::from_array_in(
                                    [p.key.to_expression().clone_in(ast_builder.allocator())],
                                    ast_builder,
                                ),
                                ast_builder,
                            )),
                            p.value.clone_in(ast_builder.allocator()),
                            false,
                            false,
                            true,
                            ast_builder,
                        ),
                    );
                    None
                };

                if let Some(name) = name {
                    if !name.starts_with("--") {
                        p.key = PropertyKey::StringLiteral(StringLiteral::boxed(
                            SPAN,
                            Str::from_in(&format!("--{name}"), ast_builder.allocator()),
                            None,
                            ast_builder,
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
