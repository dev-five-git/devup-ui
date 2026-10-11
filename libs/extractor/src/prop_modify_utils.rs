use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::extract_css::ExtractCss;
use crate::extract_style::style_property::StyleProperty;
use crate::gen_class_name::gen_class_names;
use crate::gen_style::gen_styles;
use crate::tailwind::{PROPERTY_RULES, PROPERTY_RULES_FILE, parse_class};
use crate::utils::{get_str_by_property_key, merge_object_expressions};
use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Expression, IdentifierName, JSXAttributeItem, JSXAttributeName, JSXAttributeValue,
    LogicalOperator, ObjectPropertyKind, PropertyKey, PropertyKind, StaticMemberExpression, Str,
    StringLiteral, TemplateElement, TemplateElementValue, TemplateLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;

mod order_forward;
pub(crate) use order_forward::without_order_props;
#[cfg(test)]
mod written_tests;

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
                let mut spread = spread;
                let value = spread.argument.take_in(ast_builder);
                spread.argument = order_forward::without_order_props(ast_builder, value);
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
                let mut spread = spread;
                let value = spread.argument.take_in(ast_builder);
                spread.argument = order_forward::without_order_props(ast_builder, value);
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
    let (tailwind_styles, compiled) =
        compile_tailwind_class_name(ast_builder, class_name_prop.as_ref(), style_order, filename);
    // A rebuilt `cond && "a"` still evaluates to `false`, which React would
    // render as `class="false"`, so it needs the same falsy guard as a passthrough.
    let class_name_to_use = compiled
        .as_ref()
        .or(class_name_prop.as_ref())
        .map(|class_name| convert_class_name(ast_builder, class_name));

    // Merge class names: [className] + [devup-ui component styles]
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

/// Returns extracted Tailwind styles and a replacement only when classes compiled.
pub(crate) fn compile_tailwind_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    class_name: Option<&Expression<'a>>,
    style_order: Option<u8>,
    filename: Option<&str>,
) -> (Vec<ExtractStyleValue>, Option<Expression<'a>>) {
    let mut tailwind = TailwindClassName {
        style_order,
        filename,
        styles: Vec::new(),
        property_rules: false,
    };
    let compiled =
        class_name.and_then(|class_name| tailwind.compile_expression(ast_builder, class_name));
    (tailwind.into_styles(), compiled)
}

/// Compiles the Tailwind classes of a className: each becomes the classes of
/// its styles, and every other class stays as written
struct TailwindClassName<'f> {
    style_order: Option<u8>,
    filename: Option<&'f str>,
    styles: Vec<ExtractStyleValue>,
    /// Whether a compiled class uses the custom properties Tailwind registers
    property_rules: bool,
}

impl TailwindClassName<'_> {
    fn into_styles(self) -> Vec<ExtractStyleValue> {
        let mut styles = self.styles;
        if self.property_rules {
            styles.push(ExtractStyleValue::Css(ExtractCss {
                css: PROPERTY_RULES.to_string(),
                file: PROPERTY_RULES_FILE.to_string(),
            }));
        }
        styles
    }

    /// `text` with its Tailwind classes compiled, or `None` when it has none. A
    /// class running into an end that continues in an interpolation is not
    /// whole, so it stays as written.
    fn compile_text(&mut self, text: &str, open_start: bool, open_end: bool) -> Option<String> {
        let mut compiled = String::with_capacity(text.len());
        let mut written = 0;
        let mut start = 0;
        // Class names are separated by ASCII whitespace, one byte each
        for class in text.split(|c: char| c.is_ascii_whitespace()) {
            let end = start + class.len();
            let whole = (!open_start || start > 0) && (!open_end || end < text.len());
            if let Some(tailwind) = whole.then(|| parse_class(class)).flatten() {
                compiled.push_str(&text[written..start]);
                self.property_rules |= tailwind.uses_properties();
                let mut separator = "";
                for mut style in tailwind.styles() {
                    if let Some(order) = self.style_order {
                        style.style_order = Some(order);
                    }
                    let (StyleProperty::ClassName(name)
                    | StyleProperty::Variable {
                        class_name: name, ..
                    }) = style.extract(self.filename);
                    compiled.push_str(separator);
                    compiled.push_str(&name);
                    separator = " ";
                    self.styles.push(ExtractStyleValue::Static(style));
                }
                written = end;
            }
            start = end + 1;
        }
        (written > 0).then(|| {
            compiled.push_str(&text[written..]);
            compiled
        })
    }

    /// `expression` with the Tailwind classes of its strings compiled, or
    /// `None` when they have none
    fn compile_expression<'a>(
        &mut self,
        ast_builder: &AstBuilder<'a>,
        expression: &Expression<'a>,
    ) -> Option<Expression<'a>> {
        match expression {
            Expression::StringLiteral(literal) => {
                let compiled = self.compile_text(&literal.value, false, false)?;
                Some(Expression::new_string_literal(
                    SPAN,
                    Str::from_in(&compiled, ast_builder.allocator()),
                    None,
                    ast_builder,
                ))
            }
            Expression::TemplateLiteral(template) => self.compile_template(ast_builder, template),
            Expression::ConditionalExpression(conditional) => {
                let consequent = self.compile_expression(ast_builder, &conditional.consequent);
                let alternate = self.compile_expression(ast_builder, &conditional.alternate);
                if consequent.is_none() && alternate.is_none() {
                    return None;
                }
                Some(Expression::new_conditional_expression(
                    conditional.span,
                    conditional
                        .test
                        .clone_in_with_semantic_ids(ast_builder.allocator()),
                    compiled_or(ast_builder, consequent, &conditional.consequent),
                    compiled_or(ast_builder, alternate, &conditional.alternate),
                    ast_builder,
                ))
            }
            Expression::LogicalExpression(logical) => {
                let left = self.compile_expression(ast_builder, &logical.left);
                let right = self.compile_expression(ast_builder, &logical.right);
                if left.is_none() && right.is_none() {
                    return None;
                }
                Some(Expression::new_logical_expression(
                    logical.span,
                    compiled_or(ast_builder, left, &logical.left),
                    logical.operator,
                    compiled_or(ast_builder, right, &logical.right),
                    ast_builder,
                ))
            }
            Expression::ParenthesizedExpression(parenthesized) => {
                let inner = self.compile_expression(ast_builder, &parenthesized.expression)?;
                Some(Expression::new_parenthesized_expression(
                    parenthesized.span,
                    inner,
                    ast_builder,
                ))
            }
            // Variables, calls and the like only hold their classes at runtime
            _ => None,
        }
    }

    fn compile_template<'a>(
        &mut self,
        ast_builder: &AstBuilder<'a>,
        template: &TemplateLiteral<'a>,
    ) -> Option<Expression<'a>> {
        let last = template.quasis.len() - 1;
        let quasis: Vec<Option<String>> = template
            .quasis
            .iter()
            .enumerate()
            .map(|(index, quasi)| {
                // The text on each side of an interpolation runs into it
                self.compile_text(quasi.value.cooked.as_ref()?, index > 0, index < last)
            })
            .collect();
        let expressions: Vec<Option<Expression<'a>>> = template
            .expressions
            .iter()
            .map(|expression| self.compile_expression(ast_builder, expression))
            .collect();
        if quasis.iter().all(Option::is_none) && expressions.iter().all(Option::is_none) {
            return None;
        }
        let quasis = template
            .quasis
            .iter()
            .zip(quasis)
            .map(|(quasi, compiled)| match compiled {
                Some(cooked) => TemplateElement::new(
                    quasi.span,
                    TemplateElementValue {
                        raw: Str::from_in(&template_raw(&cooked), ast_builder.allocator()),
                        cooked: Some(Str::from_in(&cooked, ast_builder.allocator())),
                    },
                    quasi.tail,
                    ast_builder,
                ),
                None => quasi.clone_in(ast_builder.allocator()),
            });
        let expressions = template
            .expressions
            .iter()
            .zip(expressions)
            .map(|(expression, compiled)| compiled_or(ast_builder, compiled, expression));
        Some(Expression::new_template_literal(
            template.span,
            oxc_allocator::Vec::from_iter_in(quasis, ast_builder),
            oxc_allocator::Vec::from_iter_in(expressions, ast_builder),
            ast_builder,
        ))
    }
}

/// `compiled`, or a copy of `original` when nothing in it compiled
fn compiled_or<'a>(
    ast_builder: &AstBuilder<'a>,
    compiled: Option<Expression<'a>>,
    original: &Expression<'a>,
) -> Expression<'a> {
    compiled.unwrap_or_else(|| original.clone_in_with_semantic_ids(ast_builder.allocator()))
}

/// The raw text of a template literal part whose value is `cooked`
fn template_raw(cooked: &str) -> String {
    let mut raw = String::with_capacity(cooked.len());
    let mut chars = cooked.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' | '`' => {
                raw.push('\\');
                raw.push(c);
            }
            '$' if chars.peek() == Some(&'{') => raw.push_str("\\$"),
            '\r' => raw.push_str("\\r"),
            c => raw.push(c),
        }
    }
    raw
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
                    if snapshot_reference(spread) {
                        spread.clone_in(ast_builder.allocator())
                    } else {
                        crate::utils::wrap_direct_call(
                            ast_builder,
                            &Expression::new_identifier(SPAN, "Object", ast_builder),
                            &[spread.clone_in(ast_builder.allocator())],
                        )
                    },
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

/// Synthetic hygienic names identify null-prototype data snapshots, not user bindings.
fn snapshot_reference(expression: &Expression<'_>) -> bool {
    matches!(expression, Expression::Identifier(identifier)
        if identifier.span == SPAN && identifier.reference_id.get().is_none()
            && identifier.name.starts_with("__devupSpread"))
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

    let mut list = ClassList {
        texts: vec![String::new()],
        interpolations: Vec::new(),
    };
    for expression in expressions {
        match expression {
            Expression::StringLiteral(literal) => {
                let classes = literal.value.trim();
                if !classes.is_empty() {
                    list.separate();
                    list.text().push_str(classes);
                }
            }
            Expression::TemplateLiteral(template) => {
                let last = template.quasis.len() - 1;
                for (index, quasi) in template.quasis.iter().enumerate() {
                    let text = quasi
                        .value
                        .cooked
                        .as_ref()
                        .map_or(quasi.value.raw.as_str(), |cooked| cooked.as_str());
                    let classes = text.trim();
                    if index == 0 {
                        if !classes.is_empty() || last > 0 {
                            list.separate();
                        }
                    } else if text.starts_with(|c: char| c.is_ascii_whitespace())
                        && (!classes.is_empty() || index < last)
                    {
                        list.text().push(' ');
                    }
                    list.text().push_str(classes);
                    if index < last {
                        // Text running into an interpolation stays attached to
                        // it, as in `icon-${name}`
                        if !classes.is_empty() && text.ends_with(|c: char| c.is_ascii_whitespace())
                        {
                            list.text().push(' ');
                        }
                        list.interpolate(
                            template.expressions[index].clone_in(ast_builder.allocator()),
                        );
                    }
                }
            }
            _ => {
                list.separate();
                list.interpolate(expression.clone_in(ast_builder.allocator()));
            }
        }
    }

    if list.interpolations.is_empty() {
        return Some(Expression::new_string_literal(
            SPAN,
            Str::from_in(list.texts[0].trim(), ast_builder.allocator()),
            None,
            ast_builder,
        ));
    }
    let last = list.texts.len() - 1;
    let quasis = oxc_allocator::Vec::from_iter_in(
        list.texts.iter().enumerate().map(|(index, text)| {
            TemplateElement::new(
                SPAN,
                TemplateElementValue {
                    raw: Str::from_in(&template_raw(text), ast_builder.allocator()),
                    cooked: Some(Str::from_in(text, ast_builder.allocator())),
                },
                index == last,
                ast_builder,
            )
        }),
        ast_builder,
    );
    Some(Expression::new_template_literal(
        SPAN,
        quasis,
        oxc_allocator::Vec::from_iter_in(list.interpolations, ast_builder),
        ast_builder,
    ))
}

/// A className merged from lists of classes, each separated from the next by
/// a space
struct ClassList<'a> {
    /// The text before each interpolation, and after the last
    texts: Vec<String>,
    interpolations: Vec<Expression<'a>>,
}

impl<'a> ClassList<'a> {
    fn text(&mut self) -> &mut String {
        let last = self.texts.len() - 1;
        &mut self.texts[last]
    }

    /// A space before the next list, when anything precedes it
    fn separate(&mut self) {
        let empty = self.interpolations.is_empty() && self.texts[0].is_empty();
        if !empty && !self.text().ends_with(' ') {
            self.text().push(' ');
        }
    }

    fn interpolate(&mut self, expression: Expression<'a>) {
        self.interpolations.push(expression);
        self.texts.push(String::new());
    }
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
    use crate::utils::expression_to_code;
    use oxc_allocator::Allocator;

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
