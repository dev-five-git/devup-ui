use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::style_property::StyleProperty;
use crate::gen_class_name::gen_class_names;
use crate::gen_style::gen_styles;
use crate::tailwind::{
    TailwindClass, has_tailwind_classes, parse_single_class, parse_tailwind_to_styles,
};
use crate::utils::{get_str_by_property_key, merge_object_expressions};
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
    filename: Option<&str>,
    conditional_branch: Option<(Expression<'a>, &mut [ExtractStyleProp<'a>], Option<u8>)>,
) -> (Option<Expression<'a>>, Vec<ExtractStyleValue>) {
    if let Some((condition, alt_styles, alt_style_order)) = conditional_branch {
        // Conditional styleOrder: generate className for both branches
        let (con_expr, con_tailwind) =
            get_class_name_expression(ast_builder, class_name_prop, styles, style_order, filename);
        let alt_class_name_prop = class_name_prop
            .as_ref()
            .map(|c| c.clone_in(ast_builder.allocator()));
        let (alt_expr, alt_tailwind) = get_class_name_expression(
            ast_builder,
            &alt_class_name_prop,
            alt_styles,
            alt_style_order,
            filename,
        );

        let combined_expr =
            combine_conditional_class_name(ast_builder, condition, con_expr, alt_expr);

        let mut all_tailwind = con_tailwind;
        all_tailwind.extend(alt_tailwind);
        (combined_expr, all_tailwind)
    } else {
        get_class_name_expression(ast_builder, class_name_prop, styles, style_order, filename)
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
    let mut class_names = Vec::new();
    let mut style_values = Vec::new();
    let written = std::mem::replace(props, oxc_allocator::Vec::new_in(ast_builder));
    for prop in written {
        match prop {
            ObjectPropertyKind::ObjectProperty(attr)
                if matches!(
                    get_str_by_property_key(&attr.key).as_deref(),
                    Some("className" | "style")
                ) =>
            {
                let value = Written::Prop(attr.value.clone_in(ast_builder.allocator()));
                if get_str_by_property_key(&attr.key).as_deref() == Some("className") {
                    class_names.push(value);
                } else {
                    style_values.push(value);
                }
            }
            ObjectPropertyKind::SpreadProperty(spread) => {
                class_names.push(Written::Spread(
                    spread.argument.clone_in(ast_builder.allocator()),
                ));
                style_values.push(Written::Spread(
                    spread.argument.clone_in(ast_builder.allocator()),
                ));
                props.push(ObjectPropertyKind::SpreadProperty(spread));
            }
            prop @ ObjectPropertyKind::ObjectProperty(_) => props.push(prop),
        }
    }
    let class_name_prop = last_written(ast_builder, &class_names, "className");
    let style_prop = last_written(ast_builder, &style_values, "style");

    let (class_name_expr, tailwind_styles) = resolve_class_name_expression(
        ast_builder,
        &class_name_prop,
        styles,
        style_order,
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
    if let Some(ex) = get_style_expression(ast_builder, &style_prop, styles, &style_vars, filename)
    {
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
    let mut class_names = Vec::new();
    let mut style_values = Vec::new();
    let written = std::mem::replace(props, oxc_allocator::Vec::new_in(ast_builder));
    for prop in written {
        match prop {
            JSXAttributeItem::Attribute(attr)
                if matches!(&attr.name, Identifier(ident)
                    if ident.name == "className" || ident.name == "style")
                    && attr.value.is_some() =>
            {
                let value = match &attr.value {
                    Some(JSXAttributeValue::ExpressionContainer(container)) => container
                        .expression
                        .as_expression()
                        .map(|expression| expression.clone_in(ast_builder.allocator())),
                    Some(JSXAttributeValue::StringLiteral(literal)) => Some(
                        Expression::new_string_literal(SPAN, literal.value, None, ast_builder),
                    ),
                    _ => None,
                };
                let Some(value) = value else {
                    continue;
                };
                if matches!(&attr.name, Identifier(ident) if ident.name == "className") {
                    class_names.push(Written::Prop(value));
                } else {
                    style_values.push(Written::Prop(value));
                }
            }
            JSXAttributeItem::SpreadAttribute(spread) => {
                class_names.push(Written::Spread(
                    spread.argument.clone_in(ast_builder.allocator()),
                ));
                style_values.push(Written::Spread(
                    spread.argument.clone_in(ast_builder.allocator()),
                ));
                props.push(JSXAttributeItem::SpreadAttribute(spread));
            }
            prop @ JSXAttributeItem::Attribute(_) => props.push(prop),
        }
    }
    let class_name_prop = last_written(ast_builder, &class_names, "className");
    let style_prop = last_written(ast_builder, &style_values, "style");
    let (class_name_expr, tailwind_styles) = resolve_class_name_expression(
        ast_builder,
        &class_name_prop,
        styles,
        style_order,
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
    if let Some(ex) = get_style_expression(ast_builder, &style_prop, styles, &style_vars, filename)
    {
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

/// What `value` of a JSX attribute is as an expression
fn attribute_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    value: &JSXAttributeValue<'a>,
) -> Option<Expression<'a>> {
    match value {
        JSXAttributeValue::ExpressionContainer(container) => container
            .expression
            .as_expression()
            .map(|expression| expression.clone_in_with_semantic_ids(ast_builder.allocator())),
        JSXAttributeValue::StringLiteral(literal) => Some(Expression::new_string_literal(
            SPAN,
            literal.value,
            None,
            ast_builder,
        )),
        _ => None,
    }
}

/// The `className` JSX props end up with, as written or spread
pub(crate) fn written_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &[JSXAttributeItem<'a>],
) -> Option<Expression<'a>> {
    let written: Vec<Written<'a>> = props
        .iter()
        .filter_map(|prop| match prop {
            JSXAttributeItem::Attribute(attr)
                if matches!(&attr.name, Identifier(ident) if ident.name == "className") =>
            {
                attr.value
                    .as_ref()
                    .and_then(|value| attribute_expression(ast_builder, value))
                    .map(Written::Prop)
            }
            JSXAttributeItem::SpreadAttribute(spread) => Some(Written::Spread(
                spread.argument.clone_in(ast_builder.allocator()),
            )),
            JSXAttributeItem::Attribute(_) => None,
        })
        .collect();
    last_written(ast_builder, &written, "className")
}

/// The `className` the props of an object end up with, as written or spread
pub(crate) fn written_object_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &[ObjectPropertyKind<'a>],
) -> Option<Expression<'a>> {
    let written: Vec<Written<'a>> = props
        .iter()
        .filter_map(|prop| match prop {
            ObjectPropertyKind::ObjectProperty(attr)
                if get_str_by_property_key(&attr.key).as_deref() == Some("className") =>
            {
                Some(Written::Prop(
                    attr.value
                        .clone_in_with_semantic_ids(ast_builder.allocator()),
                ))
            }
            ObjectPropertyKind::SpreadProperty(spread) => Some(Written::Spread(
                spread.argument.clone_in(ast_builder.allocator()),
            )),
            ObjectPropertyKind::ObjectProperty(_) => None,
        })
        .collect();
    last_written(ast_builder, &written, "className")
}

/// JSX props giving `class_name` beside the `className` they end up with, and
/// `style` under the `style` they end up with, for an element outside Devup UI
pub(crate) fn add_class_and_style<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<'a, JSXAttributeItem<'a>>,
    class_name: Option<Expression<'a>>,
    style: Option<Expression<'a>>,
) {
    let mut class_names = Vec::new();
    let mut style_values = Vec::new();
    let adds = |key: &str| {
        (key == "className" && class_name.is_some()) || (key == "style" && style.is_some())
    };
    let written = std::mem::replace(props, oxc_allocator::Vec::new_in(ast_builder));
    for prop in written {
        match prop {
            JSXAttributeItem::Attribute(attr)
                if attr.value.is_some()
                    && matches!(&attr.name, Identifier(ident) if adds(&ident.name)) =>
            {
                let Some(value) = attr
                    .value
                    .as_ref()
                    .and_then(|value| attribute_expression(ast_builder, value))
                else {
                    continue;
                };
                if matches!(&attr.name, Identifier(ident) if ident.name == "className") {
                    class_names.push(Written::Prop(value));
                } else {
                    style_values.push(Written::Prop(value));
                }
            }
            JSXAttributeItem::SpreadAttribute(spread) => {
                class_names.push(Written::Spread(
                    spread.argument.clone_in(ast_builder.allocator()),
                ));
                style_values.push(Written::Spread(
                    spread.argument.clone_in(ast_builder.allocator()),
                ));
                props.push(JSXAttributeItem::SpreadAttribute(spread));
            }
            prop @ JSXAttributeItem::Attribute(_) => props.push(prop),
        }
    }
    let (class_name, style) =
        written_with(ast_builder, &class_names, &style_values, class_name, style);
    if let Some(ex) = class_name {
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
    if let Some(ex) = style {
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
}

/// The props of an object giving `class_name` beside the `className` they end
/// up with, and `style` under the `style` they end up with, for an element
/// outside Devup UI
pub(crate) fn add_class_and_style_to_object<'a>(
    ast_builder: &AstBuilder<'a>,
    props: &mut oxc_allocator::Vec<'a, ObjectPropertyKind<'a>>,
    class_name: Option<Expression<'a>>,
    style: Option<Expression<'a>>,
) {
    let mut class_names = Vec::new();
    let mut style_values = Vec::new();
    let adds = |key: &str| {
        (key == "className" && class_name.is_some()) || (key == "style" && style.is_some())
    };
    let written = std::mem::replace(props, oxc_allocator::Vec::new_in(ast_builder));
    for prop in written {
        match prop {
            ObjectPropertyKind::ObjectProperty(attr)
                if get_str_by_property_key(&attr.key).is_some_and(|key| adds(&key)) =>
            {
                let value = Written::Prop(attr.value.clone_in(ast_builder.allocator()));
                if get_str_by_property_key(&attr.key).as_deref() == Some("className") {
                    class_names.push(value);
                } else {
                    style_values.push(value);
                }
            }
            ObjectPropertyKind::SpreadProperty(spread) => {
                class_names.push(Written::Spread(
                    spread.argument.clone_in(ast_builder.allocator()),
                ));
                style_values.push(Written::Spread(
                    spread.argument.clone_in(ast_builder.allocator()),
                ));
                props.push(ObjectPropertyKind::SpreadProperty(spread));
            }
            prop @ ObjectPropertyKind::ObjectProperty(_) => props.push(prop),
        }
    }
    let (class_name, style) =
        written_with(ast_builder, &class_names, &style_values, class_name, style);
    for (key, value) in [("className", class_name), ("style", style)] {
        if let Some(value) = value {
            props.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::new_static_identifier(SPAN, key, ast_builder),
                value,
                false,
                false,
                false,
                ast_builder,
            ));
        }
    }
}

/// The `className` and `style` the props written end up with, joined to
/// `class_name` and merged over `style`
fn written_with<'a>(
    ast_builder: &AstBuilder<'a>,
    class_names: &[Written<'a>],
    style_values: &[Written<'a>],
    class_name: Option<Expression<'a>>,
    style: Option<Expression<'a>>,
) -> (Option<Expression<'a>>, Option<Expression<'a>>) {
    let class_name = class_name.and_then(|class_name| {
        let mut expressions = Vec::with_capacity(2);
        if let Some(written) = last_written(ast_builder, class_names, "className") {
            expressions.push(convert_class_name(ast_builder, &written));
        }
        expressions.push(class_name);
        merge_string_expressions(ast_builder, &expressions)
    });
    let style = style.and_then(|style| {
        let mut expressions = vec![style];
        expressions.extend(last_written(ast_builder, style_values, "style"));
        merge_object_expressions(ast_builder, &expressions)
    });
    (class_name, style)
}

/// Returns (className expression, extracted Tailwind styles)
pub fn get_class_name_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name_prop: &Option<Expression<'a>>,
    styles: &mut [ExtractStyleProp<'a>],
    style_order: Option<u8>,
    filename: Option<&str>,
) -> (Option<Expression<'a>>, Vec<ExtractStyleValue>) {
    // Extract Tailwind styles from static className strings and generate class names
    let (tailwind_styles, tailwind_class_expr) =
        extract_tailwind_from_class_name(ast_builder, class_name_prop, style_order, filename);

    // Determine the className expression to use:
    // - If we extracted Tailwind styles, use generated class names (replace original)
    // - Otherwise, preserve the original className
    let class_name_to_use = if let Some(tailwind_class_expr) = tailwind_class_expr {
        // Tailwind className → replaced with generated class names. A rebuilt
        // `cond && "a"` still evaluates to `false`, which React would render as
        // `class="false"`, so it needs the same falsy guard as a passthrough.
        Some(convert_class_name(ast_builder, &tailwind_class_expr))
    } else {
        // Non-Tailwind className → keep original
        class_name_prop
            .as_ref()
            .map(|class_name| convert_class_name(ast_builder, class_name))
    };

    // Merge class names: [tailwind/original class names] + [devup-ui component styles]
    let mut class_expressions = Vec::with_capacity(2);
    if let Some(class_name) = class_name_to_use {
        class_expressions.push(class_name);
    }
    if let Some(class_name) = gen_class_names(ast_builder, styles, style_order, filename) {
        class_expressions.push(class_name);
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

                // Move ExtractStyleValue into ExtractStyleProp::Static for gen_class_names,
                // then recover the same values afterward without deep-cloning each style.
                let mut tailwind_style_props: Vec<ExtractStyleProp> = tailwind_styles
                    .into_iter()
                    .map(ExtractStyleProp::Static)
                    .collect();

                // Generate devup-ui class names for the Tailwind styles
                let class_names_expr = gen_class_names(
                    ast_builder,
                    &mut tailwind_style_props,
                    style_order,
                    filename,
                );

                // Tailwind className styles are always `Static`, for which
                // `into_extract` yields exactly `vec![style]`. Flattening through it
                // keeps that hot path allocation-equivalent while staying total, so
                // there is no unreachable arm to carve out of coverage.
                let tailwind_styles = tailwind_style_props
                    .into_iter()
                    .flat_map(ExtractStyleProp::into_extract)
                    .collect();

                return (tailwind_styles, class_names_expr);
            }
        }
    }

    // Extract from any expression that can still carry static class strings:
    // `` `${cond ? 'text-red' : 'text-blue'} p-4` ``, `cond ? 'p-4' : 'p-8'`, `cond && 'p-4'`.
    if let Some(expression) = class_name_prop {
        let mut all_classes = String::new();
        extract_classes_from_expression(expression, &mut all_classes);
        if has_tailwind_classes(&all_classes) {
            // Single pass over every class: parse ONCE, then build both the
            // `Tailwind class → generated class name` mapping and the styles vec for
            // CSS generation together. The previous code called
            // `build_tailwind_class_mapping` and then `parse_tailwind_to_styles`, which
            // re-parsed (and re-allocated a `TailwindClass` + `ExtractStaticStyle` for)
            // every class a second time. Merging them keeps the mapping, the collected
            // styles, and the ordered `extract()` side effects byte-identical while
            // halving the per-class parse/allocate work.
            let mut class_mapping: FxHashMap<String, String> = FxHashMap::default();
            // Upper bound: at most one style per whitespace-separated class (the same
            // presize `parse_tailwind_to_styles` used), so the vec never grow-reallocs.
            let mut tailwind_styles: Vec<ExtractStyleValue> =
                Vec::with_capacity(all_classes.bytes().filter(u8::is_ascii_whitespace).count() + 1);
            for class in all_classes.split_whitespace() {
                if let Some(mut static_style) = parse_single_class(class)
                    .as_ref()
                    .and_then(TailwindClass::to_static_style)
                {
                    if let Some(order) = style_order {
                        static_style.style_order = Some(order);
                    }
                    // `ExtractStaticStyle::extract` always yields a `ClassName`, so this
                    // records the exact mapping entry the two-pass version produced.
                    if let StyleProperty::ClassName(generated) = static_style.extract(filename) {
                        class_mapping.insert(class.to_string(), generated);
                    }
                    tailwind_styles.push(ExtractStyleValue::Static(static_style));
                }
            }

            if !class_mapping.is_empty() {
                // Build the same expression back with replaced class names
                let new_expression = rebuild_expression_with_mapping_unsorted(
                    ast_builder,
                    expression,
                    &class_mapping,
                );

                return (tailwind_styles, Some(new_expression));
            }
        }
    }

    (Vec::new(), None)
}

/// Rebuild a template literal, replacing Tailwind classes with generated class names
fn rebuild_expression_with_mapping_unsorted<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &Expression<'a>,
    class_mapping: &FxHashMap<String, String>,
) -> Expression<'a> {
    // Sort the mapping ONCE by key length descending (avoids partial replacements,
    // e.g. "text-3xl" before "text-3") and reuse the sorted slice for every quasi
    // and nested expression instead of re-sorting per call.
    let mut sorted_classes: Vec<(&String, &String)> = class_mapping.iter().collect();
    sorted_classes.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));
    rebuild_expression_with_mapping(ast_builder, expression, &sorted_classes)
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
        // `str::replace` already scans the whole string internally and allocates a
        // fresh `String` only on a match, so the previous standalone `contains`
        // pre-scan was a redundant second scan of the same content. Compute the
        // replacement once and only re-own the `Cow` when it actually changed
        // (length differs, or same-length differing contents). Byte-identical.
        let replaced = result.replace(tailwind_class.as_str(), generated_class);
        if replaced.len() != result.len() || replaced != *result {
            result = Cow::Owned(replaced);
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

/// `className` or `style` written as a prop, or spread with other props
pub(crate) enum Written<'a> {
    Prop(Expression<'a>),
    Spread(Expression<'a>),
}

/// The value `key` ends up with as the props are written in order: a later
/// prop or spread holding `key` replaces an earlier one. Whether a spread holds
/// it only the runtime tells, unless it is an object literal.
fn last_written<'a>(
    ast_builder: &AstBuilder<'a>,
    written: &[Written<'a>],
    key: &'static str,
) -> Option<Expression<'a>> {
    let mut value: Option<Expression<'a>> = None;
    for written in written {
        let spread = match written {
            Written::Prop(prop) => {
                value = Some(prop.clone_in_with_semantic_ids(ast_builder.allocator()));
                continue;
            }
            Written::Spread(spread) if may_hold(spread, key) => spread,
            Written::Spread(_) => continue,
        };
        let member = |optional| {
            Expression::StaticMemberExpression(StaticMemberExpression::boxed(
                SPAN,
                spread.clone_in(ast_builder.allocator()),
                IdentifierName::new(SPAN, key, ast_builder),
                optional,
                ast_builder,
            ))
        };
        if let Some(written) = written_value(spread, key) {
            value = Some(written.clone_in_with_semantic_ids(ast_builder.allocator()));
            continue;
        }
        value = Some(match value {
            Some(earlier) if !names(spread, key) => {
                let holds = Expression::new_binary_expression(
                    SPAN,
                    Expression::new_string_literal(SPAN, key, None, ast_builder),
                    oxc_syntax::operator::BinaryOperator::In,
                    crate::utils::wrap_direct_call(
                        ast_builder,
                        &Expression::new_identifier(SPAN, "Object", ast_builder),
                        &[spread.clone_in(ast_builder.allocator())],
                    ),
                    ast_builder,
                );
                Expression::new_conditional_expression(
                    SPAN,
                    holds,
                    member(false),
                    earlier,
                    ast_builder,
                )
            }
            _ => member(!names(spread, key)),
        });
    }
    value
}

/// Whether the object literal `spread` names `key`, so it always holds it
fn names(spread: &Expression<'_>, key: &str) -> bool {
    let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(spread) else {
        return false;
    };
    object.properties.iter().any(|property| {
        matches!(property, ObjectPropertyKind::ObjectProperty(property)
            if !property.computed
                && get_str_by_property_key(&property.key).as_deref() == Some(key))
    })
}

/// What the object literal `spread` sets `key` to, when it writes it by name
/// after anything that may also hold it
fn written_value<'b, 'a>(spread: &'b Expression<'a>, key: &str) -> Option<&'b Expression<'a>> {
    let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(spread) else {
        return None;
    };
    object
        .properties
        .iter()
        .rev()
        .find_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property)
                if !property.computed
                    && get_str_by_property_key(&property.key).as_deref() != Some(key) =>
            {
                None
            }
            ObjectPropertyKind::ObjectProperty(property) if !property.computed => {
                Some(Some(&property.value))
            }
            _ => Some(None),
        })
        .flatten()
}

/// Whether the object `spread` gives may hold `key`: an object literal holds
/// only the keys it writes
fn may_hold(spread: &Expression<'_>, key: &str) -> bool {
    let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(spread) else {
        return true;
    };
    object.properties.iter().any(|property| match property {
        ObjectPropertyKind::ObjectProperty(property) => {
            property.computed || get_str_by_property_key(&property.key).as_deref() == Some(key)
        }
        ObjectPropertyKind::SpreadProperty(_) => true,
    })
}

pub fn get_style_expression<'a>(
    ast_builder: &AstBuilder<'a>,
    style_prop: &Option<Expression<'a>>,
    styles: &[ExtractStyleProp<'a>],
    style_vars: &Option<Expression<'a>>,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    let mut style_expressions = Vec::with_capacity(3);
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
            // Reuse the `prev_str` buffer instead of allocating a fresh String via
            // `format!` each iteration. `prev_str` is only ever built here from trimmed
            // pieces, so it never carries leading whitespace; trim only its trailing end.
            let trimmed_len = prev_str.trim_end().len();
            prev_str.truncate(trimmed_len);
            let target = literal.value.trim();
            if !prev_str.is_empty() {
                prev_str.push(' ');
            }
            prev_str.push_str(target);
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
                    // Reuse the existing heap buffer instead of dropping it and
                    // allocating a fresh String: one fewer allocation per template
                    // quasi. Output stays byte-identical (same trimmed contents).
                    prev_str.clear();
                    prev_str.push_str(q.value.raw.trim());
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
            // Reuse the backing capacity instead of allocating a fresh empty
            // String; `clear()` keeps the buffer, byte-identical behavior.
            prev_str.clear();
        }
    }
    {
        let tail = prev_str.trim();
        if tail.is_empty() {
            string_literals.push(String::new());
        } else {
            let mut buf = String::with_capacity(tail.len() + 1);
            buf.push(' ');
            buf.push_str(tail);
            string_literals.push(buf);
        }
    }
    if other_expressions.is_empty() {
        // Concatenate the already-owned fragments into one presized buffer instead
        // of `join("")`, which allocates a fresh Vec-backed buffer internally after
        // summing lengths. Same summed-capacity, same byte order, then `.trim()` —
        // byte-identical to `string_literals.join("")`.
        let mut merged = String::with_capacity(string_literals.iter().map(String::len).sum());
        for frag in &string_literals {
            merged.push_str(frag);
        }
        return Some(Expression::new_string_literal(
            SPAN,
            Str::from_in(merged.trim(), ast_builder.allocator()),
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
                let name = if let Some(name) = get_str_by_property_key(&p.key) {
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
                        // Build the `--`-prefixed key directly into a presized owned
                        // buffer instead of going through `format!`'s `Arguments`
                        // machinery; `Str::from_in` copies the bytes into the arena
                        // regardless, so this is the same single owned allocation
                        // minus the formatting overhead. Output is byte-identical.
                        let mut prefixed = String::with_capacity(name.len() + 2);
                        prefixed.push_str("--");
                        prefixed.push_str(&name);
                        p.key = PropertyKey::StringLiteral(StringLiteral::boxed(
                            SPAN,
                            Str::from_in(&prefixed, ast_builder.allocator()),
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

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::extract_style::{
        extract_dynamic_style::ExtractDynamicStyle, extract_static_style::ExtractStaticStyle,
    };
    use crate::utils::expression_to_code;
    use oxc_allocator::Allocator;

    #[test]
    fn test_apply_style_order_to_all_styles() {
        let mut styles = [
            ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None)),
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("padding", 0, "size", None)),
        ];

        apply_style_order_to_styles(&mut styles, Some(7));

        let ExtractStyleValue::Static(static_style) = &styles[0] else {
            panic!("expected static style");
        };
        let ExtractStyleValue::Dynamic(dynamic_style) = &styles[1] else {
            panic!("expected dynamic style");
        };
        assert_eq!(static_style.style_order(), Some(7));
        assert_eq!(dynamic_style.style_order(), Some(7));
    }

    #[test]
    fn test_merge_string_expressions_builds_template() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let expressions = [
            Expression::new_string_literal(SPAN, "base", None, &builder),
            Expression::new_identifier(SPAN, "dynamicClass", &builder),
        ];

        let merged = merge_string_expressions(&builder, &expressions).unwrap();

        assert_eq!(expression_to_code(&merged), "`base ${dynamicClass}`;");
    }
}
