use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{BinaryOperator, Expression, LogicalOperator, Str, UnaryOperator},
    builder::AstBuilder,
};
use oxc_span::SPAN;
use oxc_syntax::number::NumberBase;

use crate::{
    ExtractStyleValue,
    extract_style::{extract_dynamic_style::ExtractDynamicStyle, style_property::StyleProperty},
};

pub(super) fn class<'a>(
    ast: &AstBuilder<'a>,
    style: &ExtractStyleValue,
    filename: Option<&str>,
) -> Option<Expression<'a>> {
    let name = match style.extract(filename)? {
        StyleProperty::ClassName(name)
        | StyleProperty::Variable {
            class_name: name, ..
        } => name,
    };
    let class =
        Expression::new_string_literal(SPAN, Str::from_in(&name, ast.allocator()), None, ast);
    Some(match style {
        ExtractStyleValue::Dynamic(style) => {
            if style.presence() {
                Expression::new_conditional_expression(
                    SPAN,
                    present(ast, style),
                    class,
                    Expression::new_string_literal(SPAN, "", None, ast),
                    ast,
                )
            } else {
                class
            }
        }
        ExtractStyleValue::Static(_)
        | ExtractStyleValue::Typography(_)
        | ExtractStyleValue::Css(_)
        | ExtractStyleValue::Import(_)
        | ExtractStyleValue::FontFace(_)
        | ExtractStyleValue::Keyframes(_) => class,
    })
}

fn present<'a>(ast: &AstBuilder<'a>, style: &ExtractDynamicStyle) -> Expression<'a> {
    let raw =
        || Expression::new_identifier(SPAN, Str::from_in(style.identifier(), ast.allocator()), ast);
    let typeof_raw = || Expression::new_unary_expression(SPAN, UnaryOperator::Typeof, raw(), ast);
    let number =
        |value| Expression::new_numeric_literal(SPAN, value, None, NumberBase::Decimal, ast);
    let compare =
        |left, operator, right| Expression::new_binary_expression(SPAN, left, operator, right, ast);
    let and = |left, right| {
        Expression::new_logical_expression(SPAN, left, LogicalOperator::And, right, ast)
    };
    let infinity = compare(number(1.0), BinaryOperator::Division, number(0.0));
    let finite = and(
        compare(raw(), BinaryOperator::StrictEquality, raw()),
        and(
            compare(
                raw(),
                BinaryOperator::StrictInequality,
                infinity.clone_in(ast.allocator()),
            ),
            compare(
                raw(),
                BinaryOperator::StrictInequality,
                Expression::new_unary_expression(SPAN, UnaryOperator::UnaryNegation, infinity, ast),
            ),
        ),
    );
    let numeric = Expression::new_logical_expression(
        SPAN,
        compare(
            typeof_raw(),
            BinaryOperator::StrictInequality,
            Expression::new_string_literal(SPAN, "number", None, ast),
        ),
        LogicalOperator::Or,
        finite,
        ast,
    );
    let mut condition = and(
        compare(
            raw(),
            BinaryOperator::StrictInequality,
            Expression::new_null_literal(SPAN, ast),
        ),
        and(
            compare(
                raw(),
                BinaryOperator::StrictInequality,
                Expression::new_unary_expression(SPAN, UnaryOperator::Void, number(0.0), ast),
            ),
            compare(
                typeof_raw(),
                BinaryOperator::StrictInequality,
                Expression::new_string_literal(SPAN, "boolean", None, ast),
            ),
        ),
    );
    if !style.property().starts_with("--") {
        condition = and(
            condition,
            compare(
                raw(),
                BinaryOperator::StrictInequality,
                Expression::new_string_literal(SPAN, "", None, ast),
            ),
        );
    }
    and(condition, numeric)
}
