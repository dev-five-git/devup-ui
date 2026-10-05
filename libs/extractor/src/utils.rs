use std::borrow::Cow;

use crate::extract_style::constant::MAINTAIN_VALUE_PROPERTIES;
use css::utils::to_kebab_case;
use oxc_allocator::{Allocator, CloneIn, GetAllocator};
use oxc_ast::{
    ast::{
        Argument, CallExpression, Expression, ExpressionStatement, IdentifierName,
        JSXAttributeValue, ObjectExpression, ObjectProperty, ObjectPropertyKind, Program,
        PropertyKey, Statement, StaticMemberExpression,
    },
    builder::AstBuilder,
};

use oxc_codegen::{Codegen, CodegenOptions};
#[cfg(test)]
use oxc_parser::Parser;
use oxc_span::{GetSpan, SPAN, SourceType};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

/// Check if a filename is a vanilla-extract style file.
pub(super) fn is_vanilla_extract_file(filename: &str) -> bool {
    filename.ends_with(".css.ts") || filename.ends_with(".css.js")
}

/// Whether vanilla-extract leaves a number on `key` without a unit: unitless
/// properties, custom properties, and array indices.
pub(super) fn is_unitless_key(key: &str) -> bool {
    key.starts_with("--")
        || key.starts_with("var(")
        || key.bytes().all(|byte| byte.is_ascii_digit())
        || MAINTAIN_VALUE_PROPERTIES.contains(to_kebab_case(key).as_ref())
}

/// Whether a number on `key` stays bare in a library whose numbers mean pixels:
/// unitless keys, and Devup UI shorthands (`p`, `bg`, `mx`, ...), which keep
/// Devup UI's spacing scale because no such library defines them.
pub(super) fn keeps_bare_number(key: &str) -> bool {
    if is_unitless_key(key) {
        return true;
    }
    let mut properties = css::disassemble_property(key);
    let kebab = to_kebab_case(key);
    !(properties.len() == 1
        && properties.next().is_some_and(|property| {
            property.trim_start_matches('-') == kebab.trim_start_matches('-')
        }))
}

/// A JS number literal (`8`, `-8`, `(8)`), unlike a numeric string.
pub(super) fn js_number_literal(value: &Expression) -> Option<f64> {
    match value {
        Expression::NumericLiteral(number) => Some(number.value),
        Expression::ParenthesizedExpression(inner) => js_number_literal(&inner.expression),
        Expression::UnaryExpression(unary) => {
            js_number_literal(&unary.argument).and_then(|number| match unary.operator {
                UnaryOperator::UnaryNegation => Some(-number),
                UnaryOperator::UnaryPlus => Some(number),
                _ => None,
            })
        }
        _ => None,
    }
}

/// Strip the wrappers that exist only in the source text: TypeScript's `as` /
/// `satisfies` / `!` / explicit type arguments, plus redundant parentheses. Every
/// one is erased before the code runs, so extraction must see through them —
/// otherwise a plain `as const` silently turns styling off.
pub(super) fn unwrap_syntax_only<'a, 'b>(expression: &'b Expression<'a>) -> &'b Expression<'a> {
    match expression {
        Expression::TSAsExpression(e) => unwrap_syntax_only(&e.expression),
        Expression::TSSatisfiesExpression(e) => unwrap_syntax_only(&e.expression),
        Expression::TSNonNullExpression(e) => unwrap_syntax_only(&e.expression),
        Expression::TSInstantiationExpression(e) => unwrap_syntax_only(&e.expression),
        Expression::ParenthesizedExpression(e) => unwrap_syntax_only(&e.expression),
        _ => expression,
    }
}

/// Mutable counterpart of [`unwrap_syntax_only`].
pub(super) fn unwrap_syntax_only_mut<'a, 'b>(
    expression: &'b mut Expression<'a>,
) -> &'b mut Expression<'a> {
    match expression {
        Expression::TSAsExpression(e) => unwrap_syntax_only_mut(&mut e.expression),
        Expression::TSSatisfiesExpression(e) => unwrap_syntax_only_mut(&mut e.expression),
        Expression::TSNonNullExpression(e) => unwrap_syntax_only_mut(&mut e.expression),
        Expression::TSInstantiationExpression(e) => unwrap_syntax_only_mut(&mut e.expression),
        Expression::ParenthesizedExpression(e) => unwrap_syntax_only_mut(&mut e.expression),
        _ => expression,
    }
}

/// Convert a value to a pixel value.
///
/// Returns `Cow::Borrowed(value)` for the overwhelmingly common non-numeric
/// case (e.g. `red`, `100%`, `8px`) so the caller can feed the borrowed input
/// straight into `optimize_value` without an intermediate heap copy. Only the
/// numeric branch (`4` → `16px`) allocates, exactly as before.
pub(super) fn convert_value(value: &str) -> Cow<'_, str> {
    value.parse::<f64>().map_or_else(
        |_| Cow::Borrowed(value),
        |num| Cow::Owned(format!("{}px", num * 4.0)),
    )
}

/// Loop-invariant source type for the throwaway codegen `Program`. `d_ts()` is a
/// `const fn`, so this is a compile-time constant reused across every call
/// instead of being re-derived per invocation.
const CODEGEN_SOURCE_TYPE: SourceType = SourceType::d_ts();

/// Build the loop-invariant minify codegen options in one place so the intent
/// (minified output) is documented and the config lives outside the per-call
/// body. Options are cheap to build (no heap allocation for the empty defaults).
#[inline]
fn minify_codegen_options() -> CodegenOptions {
    CodegenOptions {
        minify: true,
        ..Default::default()
    }
}

pub(super) fn expression_to_code(expression: &Expression) -> String {
    generate_code(expression, minify_codegen_options())
}

/// `expression` as it would be written, on one line, for messages
pub(super) fn readable_code(expression: &Expression) -> String {
    let code = generate_code(expression, CodegenOptions::default());
    let code = code.trim_end().trim_end_matches(';');
    // The parentheses keep a statement from reading as a block or a directive
    let code = match expression {
        Expression::ObjectExpression(_) | Expression::StringLiteral(_) => code
            .strip_prefix('(')
            .and_then(|code| code.strip_suffix(')'))
            .unwrap_or(code),
        _ => code,
    };
    code.lines().map(str::trim).collect::<Vec<_>>().join(" ")
}

fn generate_code(expression: &Expression, options: CodegenOptions) -> String {
    let allocator = Allocator::default();
    let builder = oxc_ast::builder::AstBuilder::new(&allocator);
    // Build the one-statement `Program` directly instead of parsing an empty
    // source and inserting into it — skips a full parse round-trip per call.
    let mut body = oxc_allocator::Vec::with_capacity_in(1, &builder);
    body.push(Statement::ExpressionStatement(ExpressionStatement::boxed(
        SPAN,
        expression.clone_in(&allocator),
        &builder,
    )));
    let program = Program::new(
        SPAN,
        CODEGEN_SOURCE_TYPE,
        "",
        oxc_allocator::Vec::new_in(&builder),
        None,
        oxc_allocator::Vec::new_in(&builder),
        body,
        &builder,
    );

    Codegen::new().with_options(options).build(&program).code
}

pub(super) fn is_same_expression<'a>(a: &Expression<'a>, b: &Expression<'a>) -> bool {
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
        (Expression::Identifier(a), Expression::Identifier(b)) => a.name == b.name,
        _ => false,
    }
}

/// Represents a parsed styleOrder value that may be conditional
#[derive(Debug)]
pub(super) enum ParsedStyleOrder<'a> {
    /// No styleOrder specified
    None,
    /// Static numeric value: styleOrder={5}
    Static(u8),
    /// Conditional: styleOrder={condition ? consequent : alternate}
    Conditional {
        condition: Expression<'a>,
        consequent: Option<u8>,
        alternate: Option<u8>,
    },
}

impl ParsedStyleOrder<'_> {
    /// Convert to Option<u8> for backward compatibility (returns None for Conditional)
    pub const fn as_static(&self) -> Option<u8> {
        match self {
            ParsedStyleOrder::Static(v) => Some(*v),
            _ => None,
        }
    }
}

/// Parse styleOrder from a JSX attribute value, supporting conditionals
pub(super) fn jsx_expression_to_style_order<'a>(
    expr: &JSXAttributeValue<'a>,
    allocator: &'a Allocator,
) -> ParsedStyleOrder<'a> {
    match expr {
        JSXAttributeValue::ExpressionContainer(ec) => ec
            .expression
            .as_expression()
            .map_or(ParsedStyleOrder::None, |e| {
                expression_to_style_order(e, allocator)
            }),
        _ => jsx_expression_to_number(expr).map_or(ParsedStyleOrder::None, |n| {
            ParsedStyleOrder::Static(n as u8)
        }),
    }
}

/// Parse styleOrder from an Expression (for call expression / object path), supporting conditionals
pub(super) fn expression_to_style_order<'a>(
    expr: &Expression<'a>,
    allocator: &'a Allocator,
) -> ParsedStyleOrder<'a> {
    // Inspect `expr` ONCE. A numeric-literal probe (`get_number_by_literal_expression`)
    // never matches a conditional/logical node, so folding it into the default arm is
    // behavior-identical to the former "static probe first, then re-match" flow while
    // avoiding the redundant second inspection of `expr`.
    match expr {
        // Conditional: `cond ? a : b` → Conditional with both branches probed.
        Expression::ConditionalExpression(cond) => {
            let consequent = get_number_by_literal_expression(&cond.consequent).map(|n| n as u8);
            let alternate = get_number_by_literal_expression(&cond.alternate).map(|n| n as u8);
            ParsedStyleOrder::Conditional {
                condition: cond.test.clone_in(allocator),
                consequent,
                alternate,
            }
        }
        // Logical &&: `a === 1 && 5` → truthy → right side (number), falsy → None.
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
            let consequent = get_number_by_literal_expression(&logical.right).map(|n| n as u8);
            ParsedStyleOrder::Conditional {
                condition: logical.left.clone_in(allocator),
                consequent,
                alternate: None,
            }
        }
        // Otherwise fall back to static numeric-literal resolution.
        _ => get_number_by_literal_expression(expr).map_or(ParsedStyleOrder::None, |n| {
            ParsedStyleOrder::Static(n as u8)
        }),
    }
}

pub(super) fn jsx_expression_to_number(expr: &JSXAttributeValue) -> Option<f64> {
    match expr {
        JSXAttributeValue::StringLiteral(sl) => sl.value.parse::<f64>().ok(),
        JSXAttributeValue::ExpressionContainer(ec) => ec
            .expression
            .as_expression()
            .and_then(get_number_by_literal_expression),
        _ => None,
    }
}

pub(super) fn get_number_by_literal_expression(expr: &Expression) -> Option<f64> {
    match expr {
        Expression::ParenthesizedExpression(parenthesized) => {
            get_number_by_literal_expression(&parenthesized.expression)
        }
        Expression::StringLiteral(sl) => sl.value.parse::<f64>().ok(),
        Expression::TemplateLiteral(tmp) => {
            // `f64::from_str` succeeds only when every byte belongs to the
            // float grammar: ASCII digits, sign/exponent punctuation, or the
            // letters of the `inf`/`infinity`/`nan` keywords. An allocation-free
            // scan that is STRICTLY more permissive than the parser lets us bail
            // out (returning `None`) on the common non-numeric case — e.g. a
            // `${x}px` template's quasis containing `p`/`x` — without collecting
            // the quasis into a throwaway `String`. Whenever the parser would
            // have returned `Some`, every byte passes this scan, so the result
            // stays byte-identical.
            let can_be_float = tmp.quasis.iter().all(|q| {
                q.value.raw.bytes().all(|b| {
                    b.is_ascii_digit()
                        || matches!(
                            b,
                            b'.' | b'+'
                                | b'-'
                                | b'e'
                                | b'E'
                                | b'i'
                                | b'I'
                                | b'n'
                                | b'N'
                                | b'f'
                                | b'F'
                                | b'a'
                                | b'A'
                                | b't'
                                | b'T'
                                | b'y'
                                | b'Y'
                        )
                })
            });
            if can_be_float {
                tmp.quasis
                    .iter()
                    .map(|q| q.value.raw.as_str())
                    .collect::<String>()
                    .parse::<f64>()
                    .ok()
            } else {
                None
            }
        }
        Expression::NumericLiteral(num) => Some(num.value),
        Expression::UnaryExpression(unary) => get_number_by_literal_expression(&unary.argument)
            .and_then(|num| match unary.operator {
                UnaryOperator::UnaryNegation => Some(-num),
                UnaryOperator::UnaryPlus => Some(num),
                _ => None,
            }),
        _ => None,
    }
}

/// Read a literal expression as a string, borrowing wherever possible.
///
/// String-literal and boolean-literal arms return `Cow::Borrowed` (the
/// `StringLiteral`'s arena-backed `&'a str`, or a static `"true"`/`"false"`),
/// so the overwhelmingly common static-value case (`bg="red"`, `_hover={{bg:
/// "blue"}}`) reads its value with ZERO heap allocation. Only the number and
/// evaluated-template arms — which must synthesize their text — return
/// `Cow::Owned`. The borrow is tied to the arena lifetime `'a`, not the `&expr`
/// reference, so a caller can drop the borrow and mutate `*expr` immediately
/// after (see `as_visit`): the arena bytes outlive the node reassignment.
/// `number` as JavaScript's `String(number)` writes it
pub(crate) fn js_number_string(number: f64) -> String {
    if number.is_infinite() {
        return if number > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        }
        .to_string();
    }
    if number == 0.0 {
        return "0".to_string();
    }
    if (1e-6..1e21).contains(&number.abs()) {
        return number.to_string();
    }
    // Exponent notation, with an explicit sign on a positive exponent
    let exponent = format!("{number:e}");
    match exponent.split_once('e') {
        Some((mantissa, power)) if !power.starts_with('-') => format!("{mantissa}e+{power}"),
        _ => exponent,
    }
}

pub(super) fn get_string_by_literal_expression<'a>(expr: &Expression<'a>) -> Option<Cow<'a, str>> {
    get_number_by_literal_expression(expr)
        .map(|num| Cow::Owned(js_number_string(num)))
        .or_else(|| match expr {
            Expression::ParenthesizedExpression(parenthesized) => {
                get_string_by_literal_expression(&parenthesized.expression)
            }
            Expression::StringLiteral(str) => Some(Cow::Borrowed(str.value.as_str())),
            Expression::BooleanLiteral(bool) => {
                Some(Cow::Borrowed(if bool.value { "true" } else { "false" }))
            }
            Expression::TemplateLiteral(tmp) => {
                let mut collect = String::new();
                for (idx, q) in tmp.quasis.iter().enumerate() {
                    collect.push_str(q.value.raw.as_str());
                    if idx < tmp.expressions.len() {
                        let value = get_string_by_literal_expression(&tmp.expressions[idx])?;
                        collect.push_str(&value);
                    }
                }
                Some(Cow::Owned(collect))
            }
            _ => None,
        })
}

pub(super) fn wrap_array_filter<'a>(
    builder: &AstBuilder<'a>,
    expr: &[Expression<'a>],
) -> Option<Expression<'a>> {
    if expr.is_empty() {
        return None;
    }
    if expr.len() == 1 {
        return Some(expr[0].clone_in(builder.allocator()));
    }

    // 1. Create ArrayExpression: [a, b, ...]
    let array_elements = oxc_allocator::Vec::from_iter_in(
        expr.iter().map(|e| e.clone_in(builder.allocator()).into()),
        builder,
    );
    let array_expr = Expression::new_array_expression(SPAN, array_elements, builder);

    // 2. Create StaticMemberExpression: array.filter
    let filter_member = Expression::StaticMemberExpression(StaticMemberExpression::boxed(
        SPAN,
        array_expr,
        IdentifierName::new(SPAN, "filter", builder),
        false,
        builder,
    ));

    // 3. Create CallExpression: array.filter(Boolean)
    let filter_call = Expression::CallExpression(CallExpression::boxed(
        SPAN,
        filter_member,
        None::<oxc_allocator::Box<'_, oxc_ast::ast::TSTypeParameterInstantiation<'_>>>,
        {
            let mut args = oxc_allocator::Vec::with_capacity_in(1, builder);
            args.push(Argument::from(Expression::new_identifier(
                SPAN, "Boolean", builder,
            )));
            args
        },
        false,
        builder,
    ));

    // 4. Create StaticMemberExpression: array.filter(Boolean).join
    let join_member = Expression::StaticMemberExpression(StaticMemberExpression::boxed(
        SPAN,
        filter_call,
        IdentifierName::new(SPAN, "join", builder),
        false,
        builder,
    ));

    // 5. Create CallExpression: array.filter(Boolean).join()
    let join_call = Expression::CallExpression(CallExpression::boxed(
        SPAN,
        join_member,
        None::<oxc_allocator::Box<'_, oxc_ast::ast::TSTypeParameterInstantiation<'_>>>,
        {
            let mut args = oxc_allocator::Vec::with_capacity_in(1, builder);
            args.push(Argument::from(Expression::new_string_literal(
                SPAN, " ", None, builder,
            )));
            args
        },
        false,
        builder,
    ));

    Some(join_call)
}

/// Whether reading `expression` again gives the same value and changes
/// nothing: literals, reads and functions, not calls, `new` or assignments
#[cfg(test)]
pub(super) fn is_pure(expression: &Expression<'_>) -> bool {
    use oxc_ast::ast::{ArrayExpressionElement, PropertyKind};
    match expression {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::Identifier(_)
        | Expression::ThisExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::FunctionExpression(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.iter().all(is_pure),
        Expression::StaticMemberExpression(member) => is_pure(&member.object),
        Expression::PrivateFieldExpression(member) => is_pure(&member.object),
        Expression::ComputedMemberExpression(member) => {
            is_pure(&member.object) && is_pure(&member.expression)
        }
        Expression::UnaryExpression(unary) => {
            unary.operator != UnaryOperator::Delete && is_pure(&unary.argument)
        }
        Expression::BinaryExpression(binary) => is_pure(&binary.left) && is_pure(&binary.right),
        Expression::LogicalExpression(logical) => is_pure(&logical.left) && is_pure(&logical.right),
        Expression::ConditionalExpression(conditional) => {
            is_pure(&conditional.test)
                && is_pure(&conditional.consequent)
                && is_pure(&conditional.alternate)
        }
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::SpreadElement(spread) => is_pure(&spread.argument),
            ArrayExpressionElement::Elision(_) => true,
            element => element.as_expression().is_some_and(is_pure),
        }),
        Expression::ObjectExpression(object) => {
            object.properties.iter().all(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    property.key.as_expression().is_none_or(is_pure)
                        && (property.kind != PropertyKind::Init || is_pure(&property.value))
                }
                ObjectPropertyKind::SpreadProperty(spread) => is_pure(&spread.argument),
            })
        }
        Expression::ParenthesizedExpression(inner) => is_pure(&inner.expression),
        Expression::TSAsExpression(inner) => is_pure(&inner.expression),
        Expression::TSSatisfiesExpression(inner) => is_pure(&inner.expression),
        Expression::TSNonNullExpression(inner) => is_pure(&inner.expression),
        Expression::TSTypeAssertion(inner) => is_pure(&inner.expression),
        _ => false,
    }
}

/// Finds whether code waits (`await`) or yields outside the functions it
/// holds, which no function wrapped around it could do in its place
#[derive(Default)]
#[cfg(test)]
pub(super) struct Suspends {
    pub found: bool,
}

#[cfg(test)]
impl<'a> oxc_ast_visit::Visit<'a> for Suspends {
    fn visit_await_expression(&mut self, _: &oxc_ast::ast::AwaitExpression<'a>) {
        self.found = true;
    }
    fn visit_yield_expression(&mut self, _: &oxc_ast::ast::YieldExpression<'a>) {
        self.found = true;
    }
    fn visit_function(&mut self, _: &oxc_ast::ast::Function<'a>, _: oxc_syntax::scope::ScopeFlags) {
    }
    fn visit_arrow_function_expression(&mut self, _: &oxc_ast::ast::ArrowFunctionExpression<'a>) {}
    fn visit_class(&mut self, _: &oxc_ast::ast::Class<'a>) {}
}

/// Whether the prop `name` stays an attribute of the element built, rather
/// than becoming its classes or style
pub(super) fn stays_attribute(name: &str) -> bool {
    css::is_special_property::is_special_property(name) && !matches!(name, "className" | "style")
}

/// `((name, ...) => body)(value, ...)`: each value is evaluated once, where
/// `body` reads it as often as it needs
pub(super) fn call_with_values<'a>(
    builder: &AstBuilder<'a>,
    values: Vec<(String, Expression<'a>)>,
    body: Expression<'a>,
) -> Expression<'a> {
    use oxc_ast::ast::{BindingPattern, FormalParameter, FormalParameterKind, FormalParameters};
    let mut parameters = oxc_allocator::Vec::with_capacity_in(values.len(), builder);
    let mut arguments = oxc_allocator::Vec::with_capacity_in(values.len(), builder);
    for (name, value) in values {
        parameters.push(FormalParameter::new(
            SPAN,
            oxc_allocator::Vec::new_in(builder),
            BindingPattern::new_binding_identifier(
                SPAN,
                <oxc_ast::ast::Ident<'a> as oxc_allocator::FromIn<'a, &str>>::from_in(
                    name.as_str(),
                    builder.allocator(),
                ),
                builder,
            ),
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
            None::<oxc_allocator::Box<Expression<'a>>>,
            false,
            None,
            false,
            false,
            builder,
        ));
        arguments.push(Argument::from(value));
    }
    let arrow = Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            parameters,
            None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
            builder,
        ),
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body.into(),
        builder,
    );
    Expression::new_call_expression(
        SPAN,
        Expression::new_parenthesized_expression(SPAN, arrow, builder),
        None::<oxc_allocator::Box<'_, oxc_ast::ast::TSTypeParameterInstantiation<'_>>>,
        arguments,
        false,
        builder,
    )
}

pub(super) fn wrap_direct_call<'a>(
    builder: &AstBuilder<'a>,
    expr: &Expression<'a>,
    args: &[Expression<'a>],
) -> Expression<'a> {
    // Built before the call rather than as an inline block argument: a block
    // argument stops rustfmt collapsing the call, and coverage cannot attribute
    // hits to the plain `SPAN,` continuation line that leaves behind.
    let mut call_args = oxc_allocator::Vec::with_capacity_in(args.len(), builder);
    for e in args {
        call_args.push(e.clone_in(builder.allocator()).into());
    }
    Expression::new_call_expression(
        SPAN,
        expr.clone_in(builder.allocator()),
        None::<oxc_allocator::Box<'_, oxc_ast::ast::TSTypeParameterInstantiation<'_>>>,
        call_args,
        false,
        builder,
    )
}
/// merge expressions to object expression
pub(super) fn merge_object_expressions<'a>(
    ast_builder: &AstBuilder<'a>,
    expressions: &[Expression<'a>],
) -> Option<Expression<'a>> {
    if expressions.is_empty() {
        return None;
    }
    if expressions.len() == 1 {
        return Some(expressions[0].clone_in(ast_builder.allocator()));
    }
    // Built before the call for the same reason as `wrap_direct_call`: an inline
    // block argument blocks rustfmt from collapsing the call, and coverage cannot
    // attribute hits to the bare `SPAN,` continuation line that leaves behind.
    let mut props = oxc_allocator::Vec::with_capacity_in(expressions.len(), ast_builder);
    for ex in expressions {
        props.push(ObjectPropertyKind::new_spread_property(
            SPAN,
            ex.clone_in(ast_builder.allocator()),
            ast_builder,
        ));
    }
    Some(Expression::new_object_expression(SPAN, props, ast_builder))
}

/// Several style arguments, or arrays of them, as vanilla-extract, Emotion and
/// styled-components compose them
pub(super) struct StyleArguments<'a> {
    /// Classes composed as they are: other styles held in variables, strings
    pub classes: Vec<Expression<'a>>,
    /// The rule objects merged: later declarations replace earlier ones and
    /// nested rules merge
    pub rules: Expression<'a>,
}

/// `None` for a single argument read as it is written, or when a part is
/// neither a rule object nor a class (`null`/`undefined`/`false` parts are
/// skipped).
pub(super) fn style_arguments<'a>(
    ast_builder: &AstBuilder<'a>,
    arguments: &[Argument<'a>],
) -> Option<StyleArguments<'a>> {
    if reads_directly(arguments) {
        return None;
    }
    let mut parts = Vec::new();
    let mut classes = Vec::new();
    for argument in arguments {
        let expression = match argument {
            Argument::SpreadElement(spread) => match unwrap_syntax_only(&spread.argument) {
                array @ Expression::ArrayExpression(_) => array,
                _ => return None,
            },
            argument => argument.to_expression(),
        };
        collect_style_parts(ast_builder, expression, &mut parts, &mut classes)?;
    }
    let mut merged = oxc_allocator::Vec::new_in(ast_builder);
    for part in parts {
        match part {
            StylePart::Rules(object) => {
                merge_style_properties(ast_builder, &mut merged, &object.properties);
            }
            StylePart::Conditional {
                test,
                consequent,
                alternate,
            } => merge_conditional_properties(
                ast_builder,
                &mut merged,
                &test,
                consequent.map_or(&[], |object| &object.properties),
                alternate.map_or(&[], |object| &object.properties),
            )?,
        }
    }
    Some(StyleArguments {
        classes,
        rules: Expression::new_object_expression(SPAN, merged, ast_builder),
    })
}

/// Whether a part `css()` or `styled()` joins, or an object a JSX spread
/// gives, reads what `unknown` holds
pub(super) fn reads_unknown(
    expression: &Expression<'_>,
    unknown: &crate::imported_constants::Unknown,
) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::ArrayExpression(array) => array.elements.iter().any(|element| {
            element
                .as_expression()
                .is_some_and(|element| reads_unknown(element, unknown))
        }),
        Expression::LogicalExpression(logical) => {
            (logical.operator != LogicalOperator::And && reads_unknown(&logical.left, unknown))
                || reads_unknown(&logical.right, unknown)
        }
        Expression::ConditionalExpression(conditional) => {
            reads_unknown(&conditional.consequent, unknown)
                || reads_unknown(&conditional.alternate, unknown)
        }
        expression => unknown.read_by(expression),
    }
}

pub(super) fn binding_root<'e>(expression: &'e Expression<'_>) -> Option<&'e str> {
    match expression {
        Expression::Identifier(identifier) => Some(identifier.name.as_str()),
        Expression::StaticMemberExpression(member) => binding_root(&member.object),
        Expression::ComputedMemberExpression(member) => binding_root(&member.object),
        _ => None,
    }
}

/// A single style argument that needs no composing: a rule object, CSS text,
/// or a condition choosing between rule objects
pub(super) fn reads_directly(arguments: &[Argument<'_>]) -> bool {
    let [argument] = arguments else {
        return false;
    };
    let expression = match argument {
        Argument::SpreadElement(spread) => Some(&spread.argument),
        argument => argument.as_expression(),
    };
    match expression.map(unwrap_syntax_only) {
        Some(
            Expression::ObjectExpression(_)
            | Expression::TemplateLiteral(_)
            | Expression::StringLiteral(_),
        ) => true,
        Some(Expression::ConditionalExpression(conditional)) => {
            [&conditional.consequent, &conditional.alternate]
                .into_iter()
                .all(|side| matches!(branch(side), Some(Branch::Rules(_) | Branch::Empty)))
        }
        _ => false,
    }
}

enum StylePart<'b, 'a> {
    Rules(&'b ObjectExpression<'a>),
    Conditional {
        test: Expression<'a>,
        consequent: Option<&'b ObjectExpression<'a>>,
        alternate: Option<&'b ObjectExpression<'a>>,
    },
}

enum Branch<'b, 'a> {
    Empty,
    Rules(&'b ObjectExpression<'a>),
    Class(&'b Expression<'a>),
}

fn branch<'b, 'a>(expression: &'b Expression<'a>) -> Option<Branch<'b, 'a>> {
    let expression = unwrap_syntax_only(expression);
    match expression {
        Expression::ObjectExpression(object) => Some(Branch::Rules(object)),
        Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => Some(Branch::Empty),
        Expression::Identifier(identifier) if identifier.name == "undefined" => Some(Branch::Empty),
        Expression::Identifier(_)
        | Expression::StaticMemberExpression(_)
        | Expression::ComputedMemberExpression(_)
        | Expression::StringLiteral(_)
        | Expression::TemplateLiteral(_) => Some(Branch::Class(expression)),
        _ => None,
    }
}

/// `value` as a class: itself when it is a string, nothing otherwise, as the
/// libraries skip `true` and other non-class values
fn string_class<'a>(ast_builder: &AstBuilder<'a>, value: &Expression<'a>) -> Expression<'a> {
    if matches!(
        value,
        Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
    ) {
        return value.clone_in(ast_builder.allocator());
    }
    let is_string = Expression::new_binary_expression(
        SPAN,
        Expression::new_unary_expression(
            SPAN,
            UnaryOperator::Typeof,
            value.clone_in(ast_builder.allocator()),
            ast_builder,
        ),
        BinaryOperator::StrictEquality,
        Expression::new_string_literal(SPAN, "string", None, ast_builder),
        ast_builder,
    );
    Expression::new_conditional_expression(
        SPAN,
        is_string,
        value.clone_in(ast_builder.allocator()),
        Expression::new_string_literal(SPAN, "", None, ast_builder),
        ast_builder,
    )
}

/// The first value in `props` that is only known at runtime
pub(super) fn runtime_value(props: &[crate::ExtractStyleProp<'_>]) -> Option<String> {
    let mut unreadable = Vec::new();
    unreadable_styles(props, true, &mut unreadable);
    unreadable
        .into_iter()
        .next()
        .map(|(_, code)| code)
        .or_else(|| {
            props
                .iter()
                .flat_map(crate::ExtractStyleProp::extract)
                .find_map(|value| match value {
                    crate::ExtractStyleValue::Dynamic(style) => {
                        Some(style.identifier().to_string())
                    }
                    _ => None,
                })
        })
}

/// The first value in `props` only known at runtime, a runtime condition
/// choosing between values included: what styles with no class to switch
/// between, global styles and keyframes, cannot hold
pub(super) fn fixed_value(props: &[crate::ExtractStyleProp<'_>]) -> Option<String> {
    fn condition(prop: &crate::ExtractStyleProp<'_>) -> Option<String> {
        use crate::ExtractStyleProp;
        match prop {
            ExtractStyleProp::Conditional { condition, .. }
            | ExtractStyleProp::Enum { condition, .. } => Some(readable_code(condition)),
            ExtractStyleProp::MemberExpression { expression, .. } => {
                Some(readable_code(expression))
            }
            ExtractStyleProp::StaticArray(props)
            | ExtractStyleProp::Evaluated { styles: props, .. } => props.iter().find_map(condition),
            _ => None,
        }
    }
    runtime_value(props).or_else(|| props.iter().find_map(condition))
}

/// Where `props` holds styles the build cannot read, with their code; with
/// `keys`, computed keys among an element's props too
pub(super) fn unreadable_styles(
    props: &[crate::ExtractStyleProp<'_>],
    keys: bool,
    found: &mut Vec<(u32, String)>,
) {
    use crate::ExtractStyleProp;
    for prop in props {
        match prop {
            ExtractStyleProp::Unreadable { offset, code, prop } => {
                if keys || !prop {
                    found.push((*offset, code.clone()));
                }
            }
            ExtractStyleProp::StaticArray(props)
            | ExtractStyleProp::Evaluated { styles: props, .. } => {
                unreadable_styles(props, keys, found);
            }
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                for branch in [consequent, alternate].into_iter().flatten() {
                    unreadable_styles(std::slice::from_ref(branch.as_ref()), keys, found);
                }
            }
            ExtractStyleProp::Enum { map, .. } => {
                for props in map.values() {
                    unreadable_styles(props, keys, found);
                }
            }
            ExtractStyleProp::MemberExpression { map, .. } => {
                for prop in map.values() {
                    unreadable_styles(std::slice::from_ref(prop.as_ref()), keys, found);
                }
            }
            ExtractStyleProp::Static(_) | ExtractStyleProp::Expression { .. } => {}
        }
    }
}

pub(super) const STYLE_OBJECT: &str =
    "its styles must be an object literal or a constant object, or be computed from constants";

pub(super) fn build_time_error(api: &str, code: &str, requirement: &str) -> String {
    format!("`{api}()` cannot use `{code}` at build time: {requirement}")
}

pub(super) const RUNTIME_VALUE: &str = "its values must be literals, theme tokens or constants";

const COMPUTED_VALUE: &str =
    "its values must be literals, theme tokens or constants, or be computed from them";

/// `api` has no element to set a runtime value on, so its values must be
/// known at build time
pub(super) fn runtime_value_error(api: &str, value: &str) -> String {
    build_time_error(api, value, COMPUTED_VALUE)
}

pub(super) fn element_error(component: &str, code: &str, requirement: &str) -> String {
    format!("`<{component}>` cannot use `{code}` at build time: {requirement}")
}

pub(super) fn spread_error(api: &str, spread: &oxc_ast::ast::SpreadElement<'_>) -> (u32, String) {
    (
        spread.span.start,
        build_time_error(
            api,
            &format!("...{}", readable_code(&spread.argument)),
            "write every entry out, as its keys must be known",
        ),
    )
}

pub(super) fn key_error(api: &str, key: &PropertyKey<'_>) -> (u32, String) {
    let code = key.as_expression().map_or_else(String::new, readable_code);
    (
        oxc_span::GetSpan::span(key).start,
        build_time_error(api, &format!("[{code}]"), "its keys must be known"),
    )
}

pub(super) fn readable_argument(argument: &Argument<'_>) -> String {
    if let Argument::SpreadElement(spread) = argument {
        return format!("...{}", readable_code(&spread.argument));
    }
    argument
        .as_expression()
        .map_or_else(String::new, readable_code)
}

/// `value` as class names at runtime: a class string, a falsy value, or an
/// array of them nested to any depth
pub(super) fn runtime_classes<'a>(
    builder: &AstBuilder<'a>,
    value: &Expression<'a>,
) -> Expression<'a> {
    let method = |object: Expression<'a>, name: &'static str, argument: Expression<'a>| {
        let mut arguments = oxc_allocator::Vec::with_capacity_in(1, builder);
        arguments.push(Argument::from(argument));
        Expression::new_call_expression(
            SPAN,
            Expression::StaticMemberExpression(StaticMemberExpression::boxed(
                SPAN,
                object,
                IdentifierName::new(SPAN, name, builder),
                false,
                builder,
            )),
            None::<oxc_allocator::Box<'_, oxc_ast::ast::TSTypeParameterInstantiation<'_>>>,
            arguments,
            false,
            builder,
        )
    };
    let mut elements = oxc_allocator::Vec::with_capacity_in(1, builder);
    elements.push(value.clone_in(builder.allocator()).into());
    let array = Expression::new_array_expression(SPAN, elements, builder);
    let flat = method(
        array,
        "flat",
        Expression::new_identifier(SPAN, "Infinity", builder),
    );
    let filtered = method(
        flat,
        "filter",
        Expression::new_identifier(SPAN, "Boolean", builder),
    );
    method(
        filtered,
        "join",
        Expression::new_string_literal(SPAN, " ", None, builder),
    )
}

pub(super) fn unplaced_error(expression: &Expression<'_>) -> String {
    format!(
        "Cannot place `{}` at build time: an interpolation in a selector or a property name must be a literal or a constant",
        readable_code(expression)
    )
}

pub(super) fn uncomposable_error(arguments: &[Argument<'_>]) -> String {
    let arguments: Vec<String> = arguments.iter().map(readable_argument).collect();
    format!(
        "Cannot compose `{}` at build time: each style must be a rule object, a class, or a condition choosing between them",
        arguments.join(", ")
    )
}

fn collect_style_parts<'b, 'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &'b Expression<'a>,
    parts: &mut Vec<StylePart<'b, 'a>>,
    classes: &mut Vec<Expression<'a>>,
) -> Option<()> {
    let clone = |expression: &Expression<'a>| expression.clone_in(ast_builder.allocator());
    let class = |branch: &Branch<'b, 'a>| match branch {
        Branch::Class(class) => Some(clone(class)),
        _ => None,
    };
    let rules = |branch: &Branch<'b, 'a>| match branch {
        Branch::Rules(object) => Some(*object),
        _ => None,
    };
    let (test, class_true, rules_true, class_false, rules_false) =
        match unwrap_syntax_only(expression) {
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    collect_style_parts(ast_builder, element.as_expression()?, parts, classes)?;
                }
                return Some(());
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                let right = branch(&logical.right)?;
                (
                    clone(&logical.left),
                    class(&right),
                    rules(&right),
                    None,
                    None,
                )
            }
            // `left || right` and `left ?? right`: `left` while it applies, `right`
            // otherwise
            Expression::LogicalExpression(logical) => match branch(&logical.left)? {
                Branch::Rules(object) => {
                    parts.push(StylePart::Rules(object));
                    return Some(());
                }
                Branch::Empty => {
                    if logical.operator == LogicalOperator::Coalesce
                        && matches!(
                            unwrap_syntax_only(&logical.left),
                            Expression::BooleanLiteral(_)
                        )
                    {
                        return Some(());
                    }
                    return collect_style_parts(ast_builder, &logical.right, parts, classes);
                }
                Branch::Class(left) => {
                    let right = branch(&logical.right)?;
                    let test = if logical.operator == LogicalOperator::Or {
                        clone(left)
                    } else {
                        Expression::new_binary_expression(
                            SPAN,
                            clone(left),
                            BinaryOperator::Inequality,
                            Expression::new_null_literal(SPAN, ast_builder),
                            ast_builder,
                        )
                    };
                    (
                        test,
                        Some(string_class(ast_builder, left)),
                        None,
                        class(&right),
                        rules(&right),
                    )
                }
            },
            Expression::ConditionalExpression(conditional) => {
                let (consequent, alternate) = (
                    branch(&conditional.consequent)?,
                    branch(&conditional.alternate)?,
                );
                (
                    clone(&conditional.test),
                    class(&consequent),
                    rules(&consequent),
                    class(&alternate),
                    rules(&alternate),
                )
            }
            _ => {
                match branch(expression)? {
                    Branch::Rules(object) => parts.push(StylePart::Rules(object)),
                    Branch::Class(class) => classes.push(clone(class)),
                    Branch::Empty => {}
                }
                return Some(());
            }
        };
    if class_true.is_some() || class_false.is_some() {
        let empty = || Expression::new_string_literal(SPAN, "", None, ast_builder);
        classes.push(Expression::new_conditional_expression(
            SPAN,
            clone(&test),
            class_true.unwrap_or_else(empty),
            class_false.unwrap_or_else(empty),
            ast_builder,
        ));
    }
    if rules_true.is_some() || rules_false.is_some() {
        parts.push(StylePart::Conditional {
            test,
            consequent: rules_true,
            alternate: rules_false,
        });
    }
    Some(())
}
/// Merge `test ? consequent : alternate` into `merged` one property at a time,
/// so each property becomes `test ? value : fallback` over what came before.
/// A class per branch would not do: which of two atomic classes wins depends on
/// the stylesheet order, not on the order they were composed in.
fn merge_conditional_properties<'a>(
    ast_builder: &AstBuilder<'a>,
    merged: &mut oxc_allocator::Vec<'a, ObjectPropertyKind<'a>>,
    test: &Expression<'a>,
    consequent: &[ObjectPropertyKind<'a>],
    alternate: &[ObjectPropertyKind<'a>],
) -> Option<()> {
    let mut keys: Vec<Cow<'_, str>> = Vec::new();
    for property in consequent.iter().chain(alternate) {
        let key = static_property(property)?.0;
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    for key in keys {
        let previous = merged
            .iter()
            .position(|property| static_property(property).is_some_and(|(k, _)| k == key))
            .map(|index| merged.remove(index));
        let fallback = previous
            .as_ref()
            .and_then(static_property)
            .map(|(_, property)| &property.value);
        let (when_true, when_false) = (
            find_property(consequent, &key),
            find_property(alternate, &key),
        );
        let source = when_true.or(when_false)?;
        let when_true = when_true.map(|property| &property.value);
        let when_false = when_false.map(|property| &property.value);
        let value = if [when_true, when_false, fallback]
            .iter()
            .any(|value| matches!(value, Some(Expression::ObjectExpression(_))))
        {
            let mut nested = oxc_allocator::Vec::new_in(ast_builder);
            merge_style_properties(ast_builder, &mut nested, object_properties(fallback)?);
            merge_conditional_properties(
                ast_builder,
                &mut nested,
                test,
                object_properties(when_true)?,
                object_properties(when_false)?,
            )?;
            Expression::new_object_expression(SPAN, nested, ast_builder)
        } else {
            let (when_true, when_false) = (when_true.or(fallback), when_false.or(fallback));
            Expression::new_conditional_expression(
                test.span(),
                test.clone_in(ast_builder.allocator()),
                value_or_undefined(ast_builder, when_true),
                value_or_undefined(ast_builder, when_false),
                ast_builder,
            )
        };
        let mut property = source.clone_in(ast_builder.allocator());
        property.value = value;
        merged.push(ObjectPropertyKind::ObjectProperty(property));
    }
    Some(())
}

fn find_property<'b, 'a>(
    properties: &'b [ObjectPropertyKind<'a>],
    key: &str,
) -> Option<&'b oxc_allocator::Box<'a, ObjectProperty<'a>>> {
    properties
        .iter()
        .filter_map(static_property)
        .find_map(|(k, property)| (k == key).then_some(property))
}

fn object_properties<'b, 'a>(
    value: Option<&'b Expression<'a>>,
) -> Option<&'b [ObjectPropertyKind<'a>]> {
    match value {
        None => Some(&[]),
        Some(Expression::ObjectExpression(object)) => Some(&object.properties),
        Some(_) => None,
    }
}

fn value_or_undefined<'a>(
    ast_builder: &AstBuilder<'a>,
    value: Option<&Expression<'a>>,
) -> Expression<'a> {
    value.map_or_else(
        || Expression::new_identifier(SPAN, "undefined", ast_builder),
        |value| value.clone_in(ast_builder.allocator()),
    )
}
fn static_property<'b, 'a>(
    property: &'b ObjectPropertyKind<'a>,
) -> Option<(Cow<'b, str>, &'b oxc_allocator::Box<'a, ObjectProperty<'a>>)> {
    match property {
        ObjectPropertyKind::ObjectProperty(property) if !property.computed => {
            Some((get_str_by_property_key(&property.key)?, property))
        }
        _ => None,
    }
}

fn merge_style_properties<'a>(
    ast_builder: &AstBuilder<'a>,
    merged: &mut oxc_allocator::Vec<'a, ObjectPropertyKind<'a>>,
    properties: &[ObjectPropertyKind<'a>],
) {
    for property in properties {
        let existing = match property {
            ObjectPropertyKind::ObjectProperty(property) if !property.computed => {
                get_str_by_property_key(&property.key).and_then(|key| {
                    merged.iter().position(|existing| {
                        matches!(existing, ObjectPropertyKind::ObjectProperty(existing)
                            if !existing.computed
                                && get_str_by_property_key(&existing.key).as_deref() == Some(key.as_ref()))
                    })
                })
            }
            _ => None,
        };
        let Some(index) = existing else {
            merged.push(property.clone_in(ast_builder.allocator()));
            continue;
        };
        let previous = merged.remove(index);
        if let (
            ObjectPropertyKind::ObjectProperty(previous),
            ObjectPropertyKind::ObjectProperty(next),
        ) = (&previous, property)
            && let (Expression::ObjectExpression(before), Expression::ObjectExpression(after)) =
                (&previous.value, &next.value)
        {
            let mut nested = oxc_allocator::Vec::new_in(ast_builder);
            merge_style_properties(ast_builder, &mut nested, &before.properties);
            merge_style_properties(ast_builder, &mut nested, &after.properties);
            let mut combined = next.clone_in(ast_builder.allocator());
            combined.value = Expression::new_object_expression(SPAN, nested, ast_builder);
            merged.push(ObjectPropertyKind::ObjectProperty(combined));
        } else {
            merged.push(property.clone_in(ast_builder.allocator()));
        }
    }
}

/// Borrowing variant of [`get_string_by_property_key`].
///
/// For a `StaticIdentifier` key this returns `Cow::Borrowed`, avoiding the heap
/// allocation the owned variant pays on every prop just to read its key name.
/// The literal fallback now also borrows where possible: `get_string_by_literal_expression`
/// returns `Cow::Borrowed` for string/boolean-literal keys and only allocates
/// (`Cow::Owned`) for numeric/template keys it must synthesize.
pub(super) fn get_str_by_property_key<'k>(key: &PropertyKey<'k>) -> Option<Cow<'k, str>> {
    if let PropertyKey::StaticIdentifier(ident) = key {
        Some(Cow::Borrowed(ident.name.as_str()))
    } else if let Some(s) = key.as_expression() {
        get_string_by_literal_expression(s)
    } else {
        None
    }
}

pub(super) fn get_string_by_property_key(key: &PropertyKey) -> Option<String> {
    get_str_by_property_key(key).map(Cow::into_owned)
}

pub const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use oxc_allocator::{FromIn, Vec};
    use oxc_ast::ast::{
        JSXAttribute, JSXAttributeName, JSXClosingElement, JSXElementName, JSXOpeningElement,
        NumberBase, Str, TSTypeParameterInstantiation, TemplateElement,
    };

    use super::*;

    #[test]
    fn test_is_vanilla_extract_file() {
        assert!(is_vanilla_extract_file("styles.css.ts"));
        assert!(is_vanilla_extract_file("theme.css.js"));
        assert!(is_vanilla_extract_file("path/to/styles.css.ts"));
        assert!(!is_vanilla_extract_file("styles.ts"));
        assert!(!is_vanilla_extract_file("styles.css"));
        assert!(!is_vanilla_extract_file("component.tsx"));
    }

    #[test]
    fn test_convert_value() {
        assert_eq!(convert_value("1px").as_ref(), "1px");
        assert_eq!(convert_value("1%").as_ref(), "1%");
        assert_eq!(convert_value("foo").as_ref(), "foo");
        assert_eq!(convert_value("4").as_ref(), "16px");
        // Non-numeric values borrow the input; only the numeric branch allocates.
        assert!(matches!(convert_value("foo"), Cow::Borrowed(_)));
        assert!(matches!(convert_value("4"), Cow::Owned(_)));
    }

    #[test]
    fn test_js_number_literal() {
        let allocator = Allocator::default();
        for (source, expected) in [
            ("8", Some(8.0)),
            ("(8)", Some(8.0)),
            ("+8", Some(8.0)),
            ("-8", Some(-8.0)),
            ("-(1.5)", Some(-1.5)),
            ("!8", None),
            ("'8'", None),
            ("-x", None),
        ] {
            let expression = Parser::new(&allocator, source, SourceType::ts())
                .parse_expression()
                .unwrap();
            assert_eq!(js_number_literal(&expression), expected, "{source}");
        }
        assert!(is_unitless_key("lineHeight"));
        assert!(is_unitless_key("--gap"));
        assert!(is_unitless_key("0"));
        assert!(!is_unitless_key("padding"));
        assert!(keeps_bare_number("lineHeight"));
        assert!(keeps_bare_number("p"));
        assert!(keeps_bare_number("mx"));
        assert!(!keeps_bare_number("padding"));
        assert!(!keeps_bare_number("backgroundColor"));
        assert!(!keeps_bare_number("WebkitTextStrokeWidth"));
    }

    #[test]
    fn test_is_pure_and_suspends() {
        use oxc_ast_visit::Visit;
        let allocator = Allocator::default();
        for (source, pure) in [
            ("true", true),
            ("null", true),
            ("1", true),
            ("1n", true),
            ("'a'", true),
            ("/a/", true),
            ("a", true),
            ("this", true),
            ("() => f()", true),
            ("function () { f(); }", true),
            ("`a${b}`", true),
            ("`a${f()}`", false),
            ("a.b", true),
            ("a[b]", true),
            ("a[f()]", false),
            ("-a", true),
            ("delete a.b", false),
            ("a + b", true),
            ("a || f()", false),
            ("a ? b : c", true),
            ("[a, , ...b]", true),
            ("[f()]", false),
            ("({ a, [b]: c, get d() { return f(); }, ...e })", true),
            ("({ [f()]: 1 })", false),
            ("({ ...f() })", false),
            ("(a)", true),
            ("a as T", true),
            ("a satisfies T", true),
            ("a!", true),
            ("<T>a", true),
            ("f()", false),
            ("new A()", false),
            ("a = 1", false),
            ("a++", false),
            ("tag`a`", false),
        ] {
            let expression = Parser::new(&allocator, source, SourceType::ts())
                .parse_expression()
                .unwrap();
            assert_eq!(is_pure(&expression), pure, "{source}");
        }
        for (source, found) in [
            (
                "[await a, async function () { await b; }, class { m() {} }]",
                true,
            ),
            (
                "[async function () { await b; }, class { m() { f(); } }]",
                false,
            ),
            ("yield 1", true),
            ("async () => await a", false),
        ] {
            let code = format!("async function* wrapper() {{ return ({source}); }}");
            let program = Parser::new(&allocator, &code, SourceType::ts())
                .parse()
                .program;
            let Statement::FunctionDeclaration(function) = &program.body[0] else {
                panic!("{source}");
            };
            let mut suspends = Suspends::default();
            suspends.visit_statements(&function.body.as_ref().unwrap().statements);
            assert_eq!(suspends.found, found, "{source}");
        }
    }

    #[test]
    #[allow(clippy::literal_string_with_formatting_args)]
    fn test_style_arguments() {
        let allocator = Allocator::default();
        let ast_builder = AstBuilder::new(&allocator);
        let compose = |source: &str| {
            let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
            let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
                unreachable!()
            };
            let Expression::CallExpression(call) = &statement.expression else {
                unreachable!()
            };
            style_arguments(&ast_builder, &call.arguments).map(
                |StyleArguments { classes, rules }| {
                    (
                        classes
                            .iter()
                            .map(expression_to_code)
                            .collect::<std::vec::Vec<_>>(),
                        expression_to_code(&rules),
                    )
                },
            )
        };
        assert_eq!(compose("f({ a: 1 })"), None);
        assert_eq!(compose("f(`a: 1;`)"), None);
        assert_eq!(compose("f(...{ a: 1 })"), None);
        assert_eq!(compose("f(cond ? { a: 1 } : null)"), None);
        assert_eq!(
            compose("f(x)"),
            Some((vec!["x;".to_string()], "({});".to_string()))
        );
        assert_eq!(
            compose("f(cond ? x : { a: 1 })"),
            Some((
                vec!["cond?x:``;".to_string()],
                "({a:cond?undefined:1});".to_string()
            ))
        );
        assert_eq!(compose("f(...x)"), None);
        assert_eq!(
            compose("f([{ a: 1 }, [cond && x]])"),
            Some((vec!["cond?x:``;".to_string()], "({a:1});".to_string()))
        );
        assert_eq!(
            compose(
                "f({ a: 1, b: { c: 1 } }, cond && { a: 2, b: { d: 2 } }, flag ? { e: 1 } : null, (on ? x : { a: 3 }), off ? null : undefined)"
            ),
            Some((
                vec!["on?x:``;".to_string()],
                "({b:{c:1,d:cond?2:undefined},e:flag?1:undefined,a:on?cond?2:1:3});".to_string()
            ))
        );
        assert_eq!(compose("f({ a: 1 }, cond && y())"), None);
        assert_eq!(compose("f({ a: 1 }, flag ? y() : null)"), None);
        assert_eq!(compose("f({ a: 1 }, flag ? null : y())"), None);
        assert_eq!(compose("f({ a: 1 }, cond && { [k]: 1 })"), None);
        assert_eq!(compose("f({ b: 1 }, cond && { b: { c: 1 } })"), None);
        let classes = |classes: &[&str]| classes.iter().map(ToString::to_string).collect();
        assert_eq!(
            compose("f({ a: 1 }, cond || { a: 2 })"),
            Some((
                classes(&["cond?typeof cond===`string`?cond:``:``;"]),
                "({a:cond?1:2});".to_string()
            ))
        );
        assert_eq!(
            compose("f({ a: 1 }, cond ?? { a: 2 })"),
            Some((
                classes(&["cond!=null?typeof cond===`string`?cond:``:``;"]),
                "({a:cond!=null?1:2});".to_string()
            ))
        );
        assert_eq!(
            compose("f({ a: 1 }, 's' || 'b')"),
            Some((classes(&["`s`?`s`:`b`;"]), "({a:1});".to_string()))
        );
        assert_eq!(
            compose("f({ a: 1 }, null || { a: 2 }, false ?? { a: 3 }, { b: 1 } || x)"),
            Some((classes(&[]), "({a:2,b:1});".to_string()))
        );
        assert_eq!(compose("f({ a: 1 }, cond || y())"), None);
        assert_eq!(compose("f({ a: 1 }, y() || { a: 2 })"), None);
        assert_eq!(
            compose(
                "f([{ a: 1, b: { c: 1 }, d: { e: 1 } }, null, undefined, false, [{ a: 2, b: { f: 2 }, d: 3, [g]: 1, ...h, 'i': 1 }]], { [g]: 2, i: 2 })"
            ),
            Some((
                std::vec::Vec::new(),
                "({a:2,b:{c:1,f:2},d:3,[g]:1,...h,[g]:2,i:2});".to_string()
            ))
        );
        assert_eq!(
            compose("f([x, a.b, a[b], 'c', `d`, { e: 1 }])"),
            Some((
                ["x;", "a.b;", "a[b];", "`c`;", "`d`;"]
                    .map(ToString::to_string)
                    .to_vec(),
                "({e:1});".to_string()
            ))
        );
    }

    #[test]
    fn test_get_number_by_literal_expression() {
        let allocator = Allocator::default();
        {
            let parsed = Parser::new(&allocator, "1", SourceType::d_ts()).parse();
            assert_eq!(parsed.program.body.len(), 1);
            assert!(matches!(
                parsed.program.body[0],
                Statement::ExpressionStatement(_)
            ));
            if let Statement::ExpressionStatement(expr) = &parsed.program.body[0] {
                assert_eq!(
                    get_number_by_literal_expression(&expr.expression),
                    Some(1.0)
                );
            }
        }
        {
            let parsed = Parser::new(&allocator, "-1", SourceType::d_ts()).parse();
            assert_eq!(parsed.program.body.len(), 1);
            assert!(matches!(
                parsed.program.body[0],
                Statement::ExpressionStatement(_)
            ));
            if let Statement::ExpressionStatement(expr) = &parsed.program.body[0] {
                assert_eq!(
                    get_number_by_literal_expression(&expr.expression),
                    Some(-1.0)
                );
            }
        }
        {
            let parsed = Parser::new(&allocator, "1.5", SourceType::d_ts()).parse();
            assert_eq!(parsed.program.body.len(), 1);
            assert!(matches!(
                parsed.program.body[0],
                Statement::ExpressionStatement(_)
            ));
            if let Statement::ExpressionStatement(expr) = &parsed.program.body[0] {
                assert_eq!(
                    get_number_by_literal_expression(&expr.expression),
                    Some(1.5)
                );
            }
        }
        {
            let parsed = Parser::new(&allocator, "delete 1", SourceType::d_ts()).parse();
            assert_eq!(parsed.program.body.len(), 1);
            assert!(matches!(
                parsed.program.body[0],
                Statement::ExpressionStatement(_)
            ));
            if let Statement::ExpressionStatement(expr) = &parsed.program.body[0] {
                assert_eq!(get_number_by_literal_expression(&expr.expression), None);
            }
        }
    }

    #[test]
    fn test_jsx_expression_to_number() {
        let allocator = Allocator::default();
        let builder = oxc_ast::builder::AstBuilder::new(&allocator);
        assert_eq!(
            jsx_expression_to_number(
                JSXAttribute::new(
                    SPAN,
                    JSXAttributeName::new_identifier(SPAN, "styleOrder", &builder),
                    Some(JSXAttributeValue::new_string_literal(
                        SPAN, "1", None, &builder
                    )),
                    &builder,
                )
                .value
                .as_ref()
                .unwrap()
            ),
            Some(1.0)
        );

        assert_eq!(
            jsx_expression_to_number(
                JSXAttribute::new(
                    SPAN,
                    JSXAttributeName::new_identifier(SPAN, "styleOrder", &builder),
                    Some(JSXAttributeValue::new_element(
                        SPAN,
                        JSXOpeningElement::boxed(
                            SPAN,
                            JSXElementName::new_identifier(SPAN, "div", &builder),
                            Some(TSTypeParameterInstantiation::boxed(
                                SPAN,
                                Vec::new_in(&builder),
                                &builder,
                            )),
                            Vec::new_in(&builder),
                            &builder,
                        ),
                        Vec::new_in(&builder),
                        Some(JSXClosingElement::boxed(
                            SPAN,
                            JSXElementName::new_identifier(SPAN, "div", &builder),
                            &builder,
                        )),
                        &builder,
                    )),
                    &builder,
                )
                .value
                .as_ref()
                .unwrap()
            ),
            None
        );

        assert_eq!(
            jsx_expression_to_number(&JSXAttributeValue::new_expression_container(
                SPAN,
                Expression::new_numeric_literal(SPAN, 2.0, None, NumberBase::Decimal, &builder)
                    .into(),
                &builder,
            )),
            Some(2.0)
        );
    }
    #[test]
    fn test_get_string_by_literal_expression() {
        let allocator = Allocator::default();
        let builder = oxc_ast::builder::AstBuilder::new(&allocator);

        let expr = Expression::new_string_literal(SPAN, "hello", None, &builder);
        assert_eq!(
            super::get_string_by_literal_expression(&expr).as_deref(),
            Some("hello")
        );

        let expr = Expression::new_numeric_literal(SPAN, 42.0, None, NumberBase::Decimal, &builder);
        assert_eq!(
            super::get_string_by_literal_expression(&expr).as_deref(),
            Some("42")
        );

        let expr = Expression::new_boolean_literal(SPAN, true, &builder);
        assert_eq!(
            super::get_string_by_literal_expression(&expr).as_deref(),
            Some("true")
        );

        let expr = Expression::new_template_literal(
            SPAN,
            oxc_allocator::Vec::from_iter_in(
                vec![TemplateElement::new(
                    SPAN,
                    oxc_ast::ast::TemplateElementValue {
                        cooked: Some(Str::from("template")),
                        raw: Str::from("template"),
                    },
                    true,
                    &builder,
                )],
                &builder,
            ),
            oxc_allocator::Vec::new_in(&builder),
            &builder,
        );
        assert_eq!(
            super::get_string_by_literal_expression(&expr).as_deref(),
            Some("template")
        );

        let expr = Expression::new_template_literal(
            SPAN,
            oxc_allocator::Vec::from_iter_in(
                vec![
                    TemplateElement::new(
                        SPAN,
                        oxc_ast::ast::TemplateElementValue {
                            cooked: Some(Str::from("a")),
                            raw: Str::from("a"),
                        },
                        false,
                        &builder,
                    ),
                    TemplateElement::new(
                        SPAN,
                        oxc_ast::ast::TemplateElementValue {
                            cooked: Some(Str::from("b")),
                            raw: Str::from("b"),
                        },
                        true,
                        &builder,
                    ),
                ],
                &builder,
            ),
            oxc_allocator::Vec::from_iter_in(
                vec![Expression::new_identifier(SPAN, "x", &builder)],
                &builder,
            ),
            &builder,
        );
        assert_eq!(super::get_string_by_literal_expression(&expr), None);

        // Identifier 등 기타 타입 - None 반환
        let expr = Expression::new_identifier(SPAN, "foo", &builder);
        assert_eq!(super::get_string_by_literal_expression(&expr), None);
    }

    #[test]
    fn test_merge_object_expressions() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let expressions = [
            Expression::new_identifier(SPAN, "first", &builder),
            Expression::new_identifier(SPAN, "second", &builder),
        ];

        let merged = merge_object_expressions(&builder, &expressions).unwrap();

        assert_eq!(expression_to_code(&merged), "({...first,...second});");
    }

    #[test]
    fn test_wrap_direct_call() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let expression = Expression::new_identifier(SPAN, "callback", &builder);
        let arguments = [Expression::new_identifier(SPAN, "value", &builder)];

        let wrapped = wrap_direct_call(&builder, &expression, &arguments);

        assert_eq!(expression_to_code(&wrapped), "callback(value);");
    }

    #[test]
    fn test_parsed_style_order_non_static() {
        assert_eq!(ParsedStyleOrder::None.as_static(), None);
    }

    use insta::assert_snapshot;
    use rstest::rstest;

    #[rstest]
    #[case::empty_array(&[] as &[&str], None)]
    #[case::single_identifier(&["a"], Some("[a].filter(Boolean).join()"))]
    #[case::multiple_identifiers(&["a", "b"], Some("[a, b].filter(Boolean).join()"))]
    #[case::identifier_and_string(&["className", "\"class-name\""], Some("[className, \"class-name\"].filter(Boolean).join()"))]
    fn test_wrap_array_filter(#[case] input: &[&str], #[case] _expected: Option<&str>) {
        let allocator = Allocator::default();
        let builder = oxc_ast::builder::AstBuilder::new(&allocator);

        // Create expressions from input strings
        let expressions: std::vec::Vec<oxc_ast::ast::Expression> = input
            .iter()
            .map(|s| {
                if s.starts_with('"') && s.ends_with('"') {
                    // String literal
                    let value = s.trim_matches('"');
                    Expression::new_string_literal(
                        SPAN,
                        Str::from_in(value, builder.allocator()),
                        None,
                        &builder,
                    )
                } else {
                    // Identifier
                    Expression::new_identifier(
                        SPAN,
                        Str::from_in(*s, builder.allocator()),
                        &builder,
                    )
                }
            })
            .collect();

        let result = super::wrap_array_filter(&builder, &expressions);

        if input.is_empty() {
            assert!(
                result.is_none(),
                "Expected None for empty array, but got Some"
            );
        } else {
            assert!(
                result.is_some(),
                "Expected Some, but got None for input: {input:?}"
            );
            if let Some(expr) = result {
                let code = super::expression_to_code(&expr);
                let snapshot_name = format!(
                    "wrap_array_filter_{}",
                    input.join("_").replace('"', "quote")
                );
                assert_snapshot!(snapshot_name, code);
            }
        }
    }

    #[test]
    fn test_get_str_by_property_key_variants() {
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);

        // Static identifier: borrowed straight out of the AST.
        let ident = PropertyKey::new_static_identifier(SPAN, "color", &builder);
        assert_eq!(get_str_by_property_key(&ident).as_deref(), Some("color"));

        // Expression key: resolved through the literal reader.
        let literal = PropertyKey::new_string_literal(SPAN, "padding", None, &builder);
        assert_eq!(
            get_str_by_property_key(&literal).as_deref(),
            Some("padding")
        );

        // `#private` keys are neither a static identifier nor an expression, so
        // there is no name to read.
        let private = PropertyKey::new_private_identifier(SPAN, "secret", &builder);
        assert_eq!(get_str_by_property_key(&private), None);
    }
}
