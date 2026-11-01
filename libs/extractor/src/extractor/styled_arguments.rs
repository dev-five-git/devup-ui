//! Admission precedes one authored-order schedule shared by callbacks and rule arrays.

use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Argument, CallExpression, Expression},
    builder::AstBuilder,
};
use oxc_span::SPAN;

use super::{ExtractResult, StyledBindings, props, records, rule_parts};
use crate::{
    extractor::{
        rule_array_evaluation, styled_record_evaluation::Evaluations,
        styled_rule_arrays::distribute_conditions,
    },
    styled_reads::Reads,
    utils::{StyleArguments, readable_code, reads_directly, style_arguments, uncomposable_error},
};

pub(super) struct Context<'s, 'a> {
    pub filename: &'s str,
    pub scope: &'s mut Expression<'a>,
    pub reads: &'s mut Reads,
    pub refused: &'s [u32],
}

pub(super) struct Prepared<'a> {
    pub rules: Option<ExtractResult<'a>>,
    pub classes: Vec<Expression<'a>>,
    pub bindings: Vec<(String, Expression<'a>)>,
    pub errors: Vec<(u32, String)>,
}

fn distribute_arguments<'a>(
    builder: &AstBuilder<'a>,
    arguments: &mut [Argument<'a>],
    refused: &[u32],
) {
    for argument in arguments {
        if let Some(part) = argument.as_expression_mut() {
            distribute_conditions(builder, part, refused);
        }
    }
}

#[cfg(test)]
pub(super) fn prepare<'a>(
    builder: &AstBuilder<'a>,
    call: &mut CallExpression<'a>,
    context: Context<'_, 'a>,
) -> Prepared<'a> {
    prepare_with_bindings(builder, call, (context, None))
}

pub(super) fn prepare_with_bindings<'a>(
    builder: &AstBuilder<'a>,
    call: &mut CallExpression<'a>,
    (context, bindings): (Context<'_, 'a>, Option<&StyledBindings<'_>>),
) -> Prepared<'a> {
    let Context {
        filename,
        scope,
        reads,
        refused,
    } = context;
    let arguments = &mut call.arguments;
    let mut result = Prepared {
        rules: None,
        classes: Vec::new(),
        bindings: Vec::new(),
        errors: Vec::new(),
    };
    let mut evaluations = Evaluations::new(builder, readable_code(scope));
    let mut plans = Vec::with_capacity(arguments.len());
    for argument in &mut *arguments {
        let plan = if let Some(rules) = argument.as_expression_mut() {
            reads.read_in(rules);
            match props::lower(&mut evaluations, rules, bindings) {
                Ok(plan) => plan,
                Err(error) => {
                    result.errors.push(error);
                    *rules = Expression::new_object_expression(
                        SPAN,
                        oxc_allocator::Vec::new_in(builder),
                        builder,
                    );
                    None
                }
            }
        } else {
            None
        };
        plans.push(plan);
    }
    let mut distributed = arguments.clone_in_with_semantic_ids(builder.allocator());
    distribute_arguments(builder, &mut distributed, refused);
    let multipart = distributed.len() > 1
        || distributed.iter().any(|argument| {
            let expression = match argument {
                Argument::SpreadElement(spread) => &spread.argument,
                argument => argument.to_expression(),
            };
            matches!(
                crate::utils::unwrap_syntax_only(expression),
                Expression::ArrayExpression(_)
            )
        });
    let admitted = reads_directly(&distributed)
        || style_arguments(builder, &distributed).is_some()
        || rule_parts::admitted(builder, &distributed, bindings);
    let callbacks = plans.iter().any(Option::is_some);
    let capture = callbacks
        || multipart
        || arguments.iter().any(|argument| {
            argument
                .as_expression()
                .is_some_and(rule_array_evaluation::composes_array_under_condition)
        });
    let mut scheduled_parts = true;
    if admitted && capture {
        let mut prepared = arguments.clone_in_with_semantic_ids(builder.allocator());
        let mut complete = true;
        for (argument, plan) in prepared.iter_mut().zip(plans) {
            if let Some(props::Plan::Properties(original)) = plan {
                if let Some(rules) =
                    super::super::styled_record::prepare_callbacks(&mut evaluations, &original)
                {
                    *argument = Argument::from(rules);
                } else {
                    scheduled_parts = false;
                }
            } else if let Some(props::Plan::Whole(plan)) = plan {
                evaluations.bindings.push((plan.name, plan.value));
                if !matches!(
                    argument.as_expression(),
                    Some(Expression::ObjectExpression(_))
                ) && let Some(rules) = argument.as_expression_mut()
                    && let Some(prepared) =
                        rule_array_evaluation::prepare(&mut evaluations, rules, |value| {
                            bindings.is_some_and(|bindings| bindings.styles(value).is_some())
                        })
                {
                    *rules = prepared;
                }
            } else {
                let value = match &*argument {
                    Argument::SpreadElement(spread) => &spread.argument,
                    argument => argument.to_expression(),
                };
                let start = evaluations.bindings.len();
                if let Some(value_read) =
                    rule_array_evaluation::prepare(&mut evaluations, value, |value| {
                        bindings.is_some_and(|bindings| bindings.styles(value).is_some())
                    })
                {
                    if callbacks && let Argument::SpreadElement(spread) = argument {
                        spread.argument = value_read;
                    } else {
                        *argument = Argument::from(value_read);
                    }
                } else {
                    evaluations.bindings.truncate(start);
                    scheduled_parts = false;
                    if !callbacks {
                        complete = false;
                        break;
                    }
                }
            }
        }
        if complete {
            *arguments = prepared;
            records::reserve(builder, scope, &evaluations.bindings);
            result.bindings = evaluations.bindings;
        }
    }
    distribute_arguments(builder, arguments, refused);
    let known_rules = bindings.is_some_and(|bindings| {
        arguments.iter().any(|argument| {
            argument
                .as_expression()
                .is_some_and(|rules| rule_parts::contains_known_rules(rules, bindings))
        })
    });
    match (!known_rules && !multipart)
        .then(|| style_arguments(builder, arguments))
        .flatten()
    {
        Some(StyleArguments { classes, rules }) => {
            *arguments = oxc_allocator::Vec::from_array_in([Argument::from(rules)], builder);
            result.classes = classes;
        }
        None if (known_rules || !reads_directly(arguments))
            && rule_parts::admitted(builder, arguments, bindings) =>
        {
            for argument in &mut *arguments {
                if (!multipart || scheduled_parts)
                    && let Some(rules) = argument.as_expression_mut()
                {
                    records::prepare(builder, rules, (scope, &mut result.bindings));
                }
            }
            if let Some((rules, classes)) =
                rule_parts::compose_bound(builder, arguments, (bindings, filename))
            {
                result.rules = Some(rules);
                result.classes = classes;
                let carrier = multipart
                    .then(|| style_arguments(builder, arguments))
                    .flatten();
                *arguments = oxc_allocator::Vec::from_array_in(
                    [Argument::from(carrier.map_or_else(
                        || {
                            Expression::new_object_expression(
                                SPAN,
                                oxc_allocator::Vec::new_in(builder),
                                builder,
                            )
                        },
                        |carrier| carrier.rules,
                    ))],
                    builder,
                );
            }
        }
        None if !reads_directly(arguments) => {
            result
                .errors
                .push((call.span.start, uncomposable_error(arguments)));
            *arguments = oxc_allocator::Vec::from_array_in(
                [Argument::from(Expression::new_object_expression(
                    SPAN,
                    oxc_allocator::Vec::new_in(builder),
                    builder,
                ))],
                builder,
            );
        }
        None => {}
    }
    result
}

#[cfg(test)]
#[path = "styled_arguments_tests.rs"]
mod tests;
