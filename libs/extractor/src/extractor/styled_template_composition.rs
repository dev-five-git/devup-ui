use super::{StyledBindings, template_bindings::Schedule};
use crate::{
    ExtractStyleProp,
    composition::Composition,
    css_utils::css_to_style_template,
    extractor::style_order_projection::AuthoredStyles,
    utils::{build_time_error, readable_code, unplaced_error, unwrap_syntax_only},
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, TemplateLiteral},
    builder::AstBuilder,
};
use oxc_span::GetSpan;

#[path = "styled_mixin_context.rs"]
mod context;
#[path = "styled_template_parts.rs"]
mod parts;

#[path = "styled_template_mixin.rs"]
mod mixins;

#[cfg(test)]
use mixins::MixinReader;

pub(super) struct TemplateComposition<'a> {
    pub styles: Vec<ExtractStyleProp<'a>>,
    pub statements: Vec<Expression<'a>>,
    pub bindings: Vec<(String, Expression<'a>)>,
    pub errors: Vec<(u32, String)>,
}

#[cfg(test)]
#[path = "w27_styled_template_coverage.rs"]
mod coverage_tests;

#[cfg(test)]
pub(super) fn compose<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    bindings: &StyledBindings<'_>,
) -> TemplateComposition<'a> {
    let scope = Expression::TemplateLiteral(oxc_allocator::Box::new_in(
        template.clone_in(ast.allocator()),
        ast,
    ));
    compose_in(ast, template, (bindings, &scope, "template.tsx"))
}

pub(super) fn compose_in<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    (bindings, scope, filename): (&StyledBindings<'_>, &Expression<'a>, &str),
) -> TemplateComposition<'a> {
    let mut composition = Composition::default();
    let mut statements = Vec::new();
    let mut errors = Vec::new();
    let mut schedule = Schedule::new(ast, scope);
    for part in parts::split(ast, template, |expression| {
        bindings.styles(expression).is_some()
    }) {
        match part {
            parts::Part::Text(mut text) => {
                for expression in &mut text.expressions {
                    if let Err(error) = schedule.scalar(expression) {
                        errors.push(error);
                    }
                }
                AuthoredStyles(
                    css_to_style_template(&text, 0, &None)
                        .styles
                        .into_iter()
                        .map(|style| ExtractStyleProp::Static(style.into()))
                        .collect(),
                )
                .registered(filename)
                .apply(ast, &mut composition);
            }
            parts::Part::Unplaced(index) => {
                let expression = &template.expressions[index];
                errors.push((expression.span().start, unplaced_error(expression)));
            }
            parts::Part::Mixin { index, context } => {
                let expression = &template.expressions[index];
                match mixins::read(&mut schedule, expression, bindings) {
                    Ok(Some(props)) => match context::contextualize(props, &context) {
                        Ok(props) => AuthoredStyles(props)
                            .registered(filename)
                            .apply(ast, &mut composition),
                        Err(context::ContextError::Never | context::ContextError::Opaque) => {
                            errors.push(nested_error(expression));
                        }
                    },
                    Ok(None) if context.is_empty() => {
                        let callback = matches!(
                            unwrap_syntax_only(expression),
                            Expression::ArrowFunctionExpression(_)
                                | Expression::FunctionExpression(_)
                        );
                        let mut value = expression.clone_in_with_semantic_ids(ast.allocator());
                        if let Err(error) = schedule.scalar(&mut value) {
                            errors.push(error);
                        }
                        if callback {
                            value = Expression::new_logical_expression(
                                expression.span(),
                                value,
                                oxc_syntax::operator::LogicalOperator::Or,
                                Expression::new_string_literal(oxc_span::SPAN, "", None, ast),
                                ast,
                            );
                        }
                        statements.push(value);
                    }
                    Ok(None) => errors.push(nested_error(expression)),
                    Err(error) => errors.push(error),
                }
            }
        }
    }
    TemplateComposition {
        styles: composition.into_props(),
        statements,
        bindings: schedule.bindings,
        errors,
    }
}

fn nested_error(expression: &Expression<'_>) -> (u32, String) {
    (
        expression.span().start,
        build_time_error(
            "styled",
            &readable_code(expression),
            "a nested mixin must give styles the build can read, so they inherit its enclosing selector, media query and layer",
        ),
    )
}

#[cfg(test)]
fn mixin<'a>(
    schedule: &mut Schedule<'_, 'a>,
    expression: &Expression<'a>,
    bindings: &StyledBindings<'_>,
) -> Result<Option<Vec<ExtractStyleProp<'a>>>, (u32, String)> {
    mixins::read(schedule, expression, bindings)
}
