//! Fixed rule keys with one original props-callback invocation per render.

use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::{GetSpan, SPAN};

use super::{super::styled_record_evaluation::Evaluations, identifier};
use crate::{
    css_utils::{reads_implicit_props_theme, reads_unmapped_theme, unmapped_theme_error},
    utils::{build_time_error, readable_code, unwrap_syntax_only, unwrap_syntax_only_mut},
};

#[path = "styled_props_values.rs"]
mod values;

#[path = "styled_props_leaves.rs"]
mod leaves;

#[path = "styled_props_self_reference.rs"]
mod self_reference;

#[path = "styled_props_capture.rs"]
mod capture;

#[path = "styled_props_branches.rs"]
mod branches;

#[path = "styled_props_discarded.rs"]
mod discarded;

#[path = "styled_direct_rules.rs"]
mod direct;

#[path = "styled_property_callbacks.rs"]
mod properties;

const PROPS_FUNCTION: &str = "a style function of the props must give one rule object at once, with every entry written out, as `(props) => ({ color: props.color })`";

pub(super) struct Invocation<'a> {
    pub name: String,
    pub value: Expression<'a>,
}

pub(super) enum Plan<'a> {
    Whole(Invocation<'a>),
    Properties(Expression<'a>),
}

enum PropsRulesError {
    Rules,
    Theme((u32, String)),
}

/// Plan a supported callback without scheduling it ahead of earlier arguments.
pub(super) fn lower<'a>(
    evaluations: &mut Evaluations<'_, 'a>,
    style: &mut Expression<'a>,
    bindings: Option<&super::StyledBindings<'_>>,
) -> Result<Option<Plan<'a>>, (u32, String)> {
    let builder = evaluations.builder;
    if !matches!(
        unwrap_syntax_only(style),
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    ) {
        return call_with_props(builder, style).map(|original| original.map(Plan::Properties));
    }
    let mut function = style.clone_in_with_semantic_ids(builder.allocator());
    let invalid = || {
        (
            style.span().start,
            build_time_error("styled", &readable_code(style), PROPS_FUNCTION),
        )
    };
    let (params, returned) = crate::css_prop::render_function(&mut function).ok_or_else(invalid)?;
    let object = matches!(
        unwrap_syntax_only(returned),
        Expression::ObjectExpression(_)
    );
    if !object
        && !bindings.is_some_and(|bindings| {
            super::rule_choices::normalize(builder, returned, bindings).is_ok()
        })
    {
        return Err(invalid());
    }
    if reads_implicit_props_theme(style) {
        return Err(unmapped_theme_error("styled", style));
    }
    let parameter_reads =
        values::analysis(builder, params, Expression::new_null_literal(SPAN, builder));
    if reads_unmapped_theme(&parameter_reads) {
        return Err(unmapped_theme_error("styled", style));
    }
    if let Some(error) = self_reference::error(style) {
        return Err(error);
    }
    let name = evaluations.fresh_name();
    let mut leaves = leaves::Leaves {
        builder,
        params,
        name: &name,
        values: oxc_allocator::Vec::new_in(builder),
        local: None,
        guard: None,
    };
    let map_error = |error| match error {
        PropsRulesError::Rules => invalid(),
        PropsRulesError::Theme(error) => error,
    };
    let skeleton = if let Expression::ObjectExpression(object) = unwrap_syntax_only(returned) {
        if leaves::shape::needs_projection(returned) {
            let local = evaluations.fresh_name();
            let (skeleton, assignments) =
                branches::Captures::object(&mut leaves, object, &local).map_err(map_error)?;
            let schedule = if assignments.is_empty() {
                values::zero(builder)
            } else {
                Expression::new_sequence_expression(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(assignments, builder),
                    builder,
                )
            };
            *returned = branches::returned(builder, schedule, &local);
            capture::declare(builder, &mut function, &local);
            skeleton
        } else {
            let mut skeleton = object.clone_in_with_semantic_ids(builder.allocator());
            leaves.object(&mut skeleton).map_err(map_error)?;
            *returned = Expression::new_array_expression(SPAN, leaves.values, builder);
            Expression::ObjectExpression(skeleton)
        }
    } else if let Some(bindings) = bindings {
        let local = evaluations.fresh_name();
        leaves.values.push(values::zero(builder).into());
        let mut plan = branches::Branches {
            builder,
            name: &name,
            leaves,
            local: &local,
            bindings,
            rules: Vec::new(),
        };
        let schedule = plan.schedule(returned).map_err(map_error)?;
        *returned = branches::returned(builder, schedule, &local);
        let skeleton = plan.skeleton();
        capture::declare(builder, &mut function, &local);
        skeleton
    } else {
        return Ok(None);
    };
    let value = Expression::new_call_expression(
        SPAN,
        function,
        None,
        oxc_allocator::Vec::from_array_in([identifier(builder, "rest").into()], builder),
        false,
        builder,
    );
    *style = skeleton;
    Ok(Some(Plan::Whole(Invocation { name, value })))
}

pub(super) fn lower_direct<'a>(
    evaluations: &mut Evaluations<'_, 'a>,
    style: &mut Expression<'a>,
    (bindings, scope): (&super::StyledBindings<'_>, &Expression<'a>),
) -> Result<(), (u32, String)> {
    let builder = evaluations.builder;
    let invalid = || {
        (
            style.span().start,
            build_time_error("styled", &readable_code(style), crate::utils::STYLE_OBJECT),
        )
    };
    let name = evaluations.fresh_name();
    let initial = Expression::new_array_expression(
        SPAN,
        oxc_allocator::Vec::from_array_in([capture::discriminator(builder, 0).into()], builder),
        builder,
    );
    let mut scope = scope.clone_in_with_semantic_ids(builder.allocator());
    super::records::reserve(
        builder,
        &mut scope,
        &[(name.clone(), initial.clone_in(builder.allocator()))],
    );
    let mut plan = branches::Branches {
        builder,
        name: &name,
        leaves: direct::Direct {
            builder,
            name: &name,
            scope,
            next: 1,
        },
        local: &name,
        bindings,
        rules: Vec::new(),
    };
    let schedule = plan.schedule(style).map_err(|error| match error {
        PropsRulesError::Rules => invalid(),
        PropsRulesError::Theme(error) => error,
    })?;
    *style = plan.skeleton();
    evaluations.bindings.push((name, initial));
    evaluations.capture(&schedule, &evaluations.boolean(true));
    Ok(())
}

/// Retain the original property source separately from the admission view.
fn call_with_props<'a>(
    builder: &AstBuilder<'a>,
    style: &mut Expression<'a>,
) -> Result<Option<Expression<'a>>, (u32, String)> {
    let Expression::ObjectExpression(object) = unwrap_syntax_only(style) else {
        return Ok(None);
    };
    if !properties::contains(object) {
        return Ok(None);
    }
    let original = style.clone_in_with_semantic_ids(builder.allocator());
    if let Expression::ObjectExpression(object) = unwrap_syntax_only_mut(style) {
        properties::rewrite(builder, object)?;
    }
    Ok(Some(original))
}

#[cfg(test)]
#[path = "styled_preparation_cost_tests.rs"]
mod preparation_cost_tests;

#[cfg(test)]
#[path = "styled_planning_tests.rs"]
mod planning_tests;
