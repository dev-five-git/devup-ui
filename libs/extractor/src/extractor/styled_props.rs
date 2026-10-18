//! Fixed rule keys with one original props-callback invocation per render.

use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, ObjectExpression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::{GetSpan, SPAN};

use super::{super::styled_record_evaluation::Evaluations, identifier};
use crate::{
    css_utils::{
        reads_implicit_props_theme, reads_unmapped_theme, theme_var_reference, unmapped_theme_error,
    },
    utils::{
        build_time_error, readable_code, unwrap_syntax_only, unwrap_syntax_only_mut,
        wrap_direct_call,
    },
};

#[path = "styled_props_values.rs"]
mod values;

#[path = "styled_props_leaves.rs"]
mod leaves;

#[path = "styled_props_self_reference.rs"]
mod self_reference;

const PROPS_FUNCTION: &str = "a style function of the props must give one rule object at once, with every entry written out, as `(props) => ({ color: props.color })`";

pub(super) struct Invocation<'a> {
    pub name: String,
    pub value: Expression<'a>,
}

enum PropsRulesError {
    Rules,
    Theme((u32, String)),
}

/// Plan a supported callback without scheduling it ahead of earlier arguments.
pub(super) fn lower<'a>(
    evaluations: &mut Evaluations<'_, 'a>,
    style: &mut Expression<'a>,
) -> Result<Option<Invocation<'a>>, (u32, String)> {
    let builder = evaluations.builder;
    if !matches!(
        unwrap_syntax_only(style),
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    ) {
        if let Expression::ObjectExpression(object) = unwrap_syntax_only_mut(style) {
            call_with_props(builder, object)?;
        }
        return Ok(None);
    }
    let mut function = style.clone_in_with_semantic_ids(builder.allocator());
    let invalid = || {
        (
            style.span().start,
            build_time_error("styled", &readable_code(style), PROPS_FUNCTION),
        )
    };
    let (params, returned) = crate::css_prop::render_function(&mut function).ok_or_else(invalid)?;
    let Expression::ObjectExpression(object) = unwrap_syntax_only(returned) else {
        // Keep the original composition refusal for non-object callbacks.
        return Ok(None);
    };
    if reads_implicit_props_theme(style) {
        return Err(unmapped_theme_error("styled", style));
    }
    let name = evaluations.fresh_name();
    let mut skeleton = object.clone_in_with_semantic_ids(builder.allocator());
    let mut leaves = leaves::Leaves {
        builder,
        params,
        name: &name,
        values: oxc_allocator::Vec::new_in(builder),
    };
    leaves.object(&mut skeleton).map_err(|error| match error {
        PropsRulesError::Rules => invalid(),
        PropsRulesError::Theme(error) => error,
    })?;
    let parameter_reads =
        values::analysis(builder, params, Expression::new_null_literal(SPAN, builder));
    if reads_unmapped_theme(&parameter_reads) {
        return Err(unmapped_theme_error("styled", style));
    }
    if let Some(error) = self_reference::error(style) {
        return Err(error);
    }
    *returned = Expression::new_array_expression(SPAN, leaves.values, builder);
    let value = Expression::new_call_expression(
        SPAN,
        function,
        None,
        oxc_allocator::Vec::from_array_in([identifier(builder, "rest").into()], builder),
        false,
        builder,
    );
    *style = Expression::ObjectExpression(skeleton);
    Ok(Some(Invocation { name, value }))
}

/// Ordinary property callbacks keep their existing invocation and number handling.
fn call_with_props<'a>(
    builder: &AstBuilder<'a>,
    object: &mut ObjectExpression<'a>,
) -> Result<(), (u32, String)> {
    for property in &mut object.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            continue;
        };
        match unwrap_syntax_only_mut(&mut property.value) {
            Expression::ObjectExpression(inner) => call_with_props(builder, inner)?,
            function @ (Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_))
                if theme_var_reference(function).is_none() =>
            {
                if reads_unmapped_theme(function) {
                    return Err(unmapped_theme_error("styled", function));
                }
                *function = wrap_direct_call(builder, function, &[identifier(builder, "rest")]);
            }
            _ => {}
        }
    }
    Ok(())
}
