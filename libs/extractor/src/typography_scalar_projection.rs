use oxc_ast::ast::{Expression, LogicalOperator};

use crate::{
    ExtractStyleProp, ExtractStyleValue,
    static_assignment::literal_source,
    utils::{expression_to_code, unwrap_syntax_only},
};

#[cfg(test)]
#[path = "typography_projection_boundary_tests.rs"]
mod boundary_tests;

pub(super) fn base_only(style: &ExtractStyleProp<'_>) -> bool {
    match style {
        ExtractStyleProp::Static(ExtractStyleValue::Typography(_)) => true,
        ExtractStyleProp::StaticArray(styles) => styles.iter().all(base_only),
        ExtractStyleProp::Conditional {
            consequent,
            alternate,
            ..
        } => consequent
            .iter()
            .chain(alternate)
            .all(|branch| base_only(branch)),
        ExtractStyleProp::MemberExpression { map, .. } => {
            map.values().all(|branch| base_only(branch))
        }
        ExtractStyleProp::Static(_)
        | ExtractStyleProp::Evaluated { .. }
        | ExtractStyleProp::Enum { .. }
        | ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::Unreadable { .. } => false,
    }
}

pub(super) fn preserves_reads(source: &Expression<'_>, class: &Expression<'_>) -> bool {
    if matches!(class, Expression::StringLiteral(_)) {
        return inert_values(source);
    }
    projected_reads(source, class)
}

fn projected_reads(source: &Expression<'_>, class: &Expression<'_>) -> bool {
    let source = unwrap_syntax_only(source);
    let class = unwrap_syntax_only(class);
    if literal_source(source)
        || matches!(source, Expression::Identifier(value) if value.name == "undefined")
    {
        return true;
    }
    match (source, class) {
        (Expression::ConditionalExpression(source), Expression::ConditionalExpression(class)) => {
            expression_to_code(&source.test) == expression_to_code(&class.test)
                && projected_reads(&source.consequent, &class.consequent)
                && projected_reads(&source.alternate, &class.alternate)
        }
        (
            Expression::ComputedMemberExpression(source),
            Expression::ComputedMemberExpression(class),
        ) => {
            !source.optional
                && literal_source(&source.object)
                && expression_to_code(&source.expression) == expression_to_code(&class.expression)
        }
        (_, Expression::LogicalExpression(class)) if class.operator == LogicalOperator::Or => {
            projected_reads(source, &class.left)
        }
        _ => false,
    }
}

fn inert_values(source: &Expression<'_>) -> bool {
    let source = unwrap_syntax_only(source);
    if literal_source(source) {
        return true;
    }
    match source {
        Expression::Identifier(value) => value.name == "undefined",
        Expression::ConditionalExpression(source) => {
            inert_values(&source.consequent) && inert_values(&source.alternate)
        }
        Expression::ComputedMemberExpression(source) => {
            !source.optional && literal_source(&source.object)
        }
        _ => false,
    }
}
