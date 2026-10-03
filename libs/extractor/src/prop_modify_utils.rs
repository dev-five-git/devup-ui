use crate::extract_style::ExtractStyleProperty;
use crate::extract_style::extract_css::ExtractCss;
use crate::extract_style::style_property::StyleProperty;
use crate::gen_class_name::gen_class_names;
use crate::gen_style::gen_styles;
use crate::tailwind::{PROPERTY_RULES_FILE, parse_class};
use crate::utils::{get_str_by_property_key, merge_object_expressions};
use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::JSXAttributeName::Identifier;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, CallExpression, Expression, IdentifierName, JSXAttributeItem,
    JSXAttributeName, JSXAttributeValue, LogicalOperator, ObjectPropertyKind, PropertyKey,
    PropertyKind, StaticMemberExpression, Str, StringLiteral, TemplateElement,
    TemplateElementValue, TemplateLiteral,
};
use oxc_ast::builder::AstBuilder;
use oxc_span::SPAN;
use std::borrow::Cow;
use std::collections::BTreeSet;

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
                if let Some(name) = get_str_by_property_key(&attr.key) {
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
    let mut tailwind = TailwindClassName {
        style_order,
        filename,
        styles: Vec::new(),
        rules: BTreeSet::new(),
    };
    let compiled = class_name_prop
        .as_ref()
        .and_then(|class_name| tailwind.compile_expression(ast_builder, class_name));
    // A rebuilt `cond && "a"` still evaluates to `false`, which React would
    // render as `class="false"`, so it needs the same falsy guard as a passthrough.
    let class_name_to_use = compiled
        .as_ref()
        .or(class_name_prop.as_ref())
        .map(|class_name| convert_class_name(ast_builder, class_name));

    // Merge class names: [className] + [devup-ui component styles]
    let mut class_expressions = Vec::with_capacity(2 + spread_props.len());
    if let Some(class_name) = class_name_to_use {
        class_expressions.push(class_name);
    }
    if let Some(class_name) = gen_class_names(ast_builder, styles, style_order, filename) {
        class_expressions.push(class_name);
    }
    if class_name_prop.is_none() {
        class_expressions.extend(
            spread_props
                .iter()
                .filter(|ex| may_hold(ex, "className"))
                .map(|ex| {
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
                }),
        );
    }
    let expression = merge_string_expressions(ast_builder, &class_expressions);

    (expression, tailwind.into_styles())
}

/// Compiles the Tailwind classes of a className: each becomes the classes of
/// its styles, and every other class stays as written
struct TailwindClassName<'f> {
    style_order: Option<u8>,
    filename: Option<&'f str>,
    styles: Vec<ExtractStyleValue>,
    /// The global rules the compiled classes need, @property and @keyframes
    rules: BTreeSet<Cow<'static, str>>,
}

impl TailwindClassName<'_> {
    fn into_styles(self) -> Vec<ExtractStyleValue> {
        let mut styles = self.styles;
        styles.extend(self.rules.into_iter().map(|rule| {
            ExtractStyleValue::Css(ExtractCss {
                css: rule.to_string(),
                file: PROPERTY_RULES_FILE.to_string(),
            })
        }));
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
                self.rules.extend(tailwind.rules());
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
                    conditional.test.clone_in(ast_builder.allocator()),
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
            Expression::CallExpression(call) => self.compile_call(ast_builder, call),
            Expression::ArrayExpression(array) => {
                let mut compiled = array.clone_in(ast_builder.allocator());
                let mut changed = false;
                for element in &mut compiled.elements {
                    if let Some(expression) = element
                        .as_expression()
                        .and_then(|expression| self.compile_expression(ast_builder, expression))
                    {
                        *element = ArrayExpressionElement::from(expression);
                        changed = true;
                    }
                }
                changed.then_some(Expression::ArrayExpression(compiled))
            }
            Expression::ObjectExpression(object) => {
                let mut compiled = object.clone_in(ast_builder.allocator());
                let mut changed = false;
                for property in &mut compiled.properties {
                    let ObjectPropertyKind::ObjectProperty(property) = property else {
                        continue;
                    };
                    let PropertyKey::StringLiteral(key) = &property.key else {
                        continue;
                    };
                    if property.computed {
                        continue;
                    }
                    if let Some(text) = self.compile_text(&key.value, false, false) {
                        property.key = PropertyKey::from(Expression::new_string_literal(
                            SPAN,
                            Str::from_in(&text, ast_builder.allocator()),
                            None,
                            ast_builder,
                        ));
                        changed = true;
                    }
                }
                changed.then_some(Expression::ObjectExpression(compiled))
            }
            // Variables and the like only hold their classes at runtime
            _ => None,
        }
    }

    /// `clsx(...)`, `classnames(...)` and `[...].join(" ")`, which join the
    /// classes they are given as they are, so each class compiles where it is
    /// written
    fn compile_call<'a>(
        &mut self,
        ast_builder: &AstBuilder<'a>,
        call: &CallExpression<'a>,
    ) -> Option<Expression<'a>> {
        let mut compiled = call.clone_in(ast_builder.allocator());
        let mut changed = false;
        match &call.callee {
            Expression::Identifier(callee)
                if matches!(callee.name.as_str(), "clsx" | "classnames" | "classNames") =>
            {
                for argument in &mut compiled.arguments {
                    if let Some(expression) = argument
                        .as_expression()
                        .and_then(|expression| self.compile_expression(ast_builder, expression))
                    {
                        *argument = Argument::from(expression);
                        changed = true;
                    }
                }
            }
            Expression::StaticMemberExpression(member)
                if member.property.name == "join"
                    && matches!(call.arguments.as_slice(), [Argument::StringLiteral(separator)] if separator.value == " ") =>
            {
                let array = self.compile_expression(ast_builder, &member.object)?;
                let mut callee = member.clone_in(ast_builder.allocator());
                callee.object = array;
                compiled.callee = Expression::StaticMemberExpression(callee);
                changed = true;
            }
            _ => {}
        }
        changed
            .then(|| Expression::CallExpression(oxc_allocator::Box::new_in(compiled, ast_builder)))
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
    compiled.unwrap_or_else(|| original.clone_in(ast_builder.allocator()))
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
        style_expressions.extend(spread_props.iter().filter(|ex| may_hold(ex, "style")).map(
            |ex| {
                Expression::StaticMemberExpression(StaticMemberExpression::boxed(
                    SPAN,
                    ex.clone_in(ast_builder.allocator()),
                    IdentifierName::new(SPAN, "style", ast_builder),
                    true,
                    ast_builder,
                ))
            },
        ));
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
