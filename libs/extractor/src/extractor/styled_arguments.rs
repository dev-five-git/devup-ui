//! Admission precedes one authored-order schedule shared by callbacks and rule arrays.

use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{Argument, CallExpression, Expression},
    builder::AstBuilder,
};
use oxc_span::SPAN;

use super::props;
use crate::{
    extractor::{
        rule_array_evaluation, styled_record_evaluation::Evaluations,
        styled_rule_arrays::distribute_conditions,
    },
    styled_reads::Reads,
    utils::{StyleArguments, readable_code, reads_directly, style_arguments, uncomposable_error},
};

pub(super) struct Context<'s, 'a> {
    pub scope: &'s mut Expression<'a>,
    pub reads: &'s mut Reads,
    pub refused: &'s [u32],
}

pub(super) struct Prepared<'a> {
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

pub(super) fn prepare<'a>(
    builder: &AstBuilder<'a>,
    call: &mut CallExpression<'a>,
    context: Context<'_, 'a>,
) -> Prepared<'a> {
    let Context {
        scope,
        reads,
        refused,
    } = context;
    let arguments = &mut call.arguments;
    let mut result = Prepared {
        classes: Vec::new(),
        bindings: Vec::new(),
        errors: Vec::new(),
    };
    let mut evaluations = Evaluations::new(builder, readable_code(scope));
    let mut plans = Vec::with_capacity(arguments.len());
    for argument in &mut *arguments {
        let plan = if let Some(rules) = argument.as_expression_mut() {
            reads.read_in(rules);
            match props::lower(&mut evaluations, rules) {
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
    let admitted = reads_directly(&distributed) || style_arguments(builder, &distributed).is_some();
    let callbacks = plans.iter().any(Option::is_some);
    let capture = callbacks
        || arguments.iter().any(|argument| {
            argument
                .as_expression()
                .is_some_and(rule_array_evaluation::composes_array_under_condition)
        });
    if admitted && capture {
        let mut prepared = arguments.clone_in_with_semantic_ids(builder.allocator());
        let mut complete = true;
        for (argument, plan) in prepared.iter_mut().zip(plans) {
            if let Some(plan) = plan {
                evaluations.bindings.push((plan.name, plan.value));
            } else {
                let value = match &*argument {
                    Argument::SpreadElement(spread) => &spread.argument,
                    argument => argument.to_expression(),
                };
                let start = evaluations.bindings.len();
                if let Some(value_read) =
                    rule_array_evaluation::prepare(&mut evaluations, value, |_| false)
                {
                    if callbacks && let Argument::SpreadElement(spread) = argument {
                        spread.argument = value_read;
                    } else {
                        *argument = Argument::from(value_read);
                    }
                } else {
                    evaluations.bindings.truncate(start);
                    if !callbacks {
                        complete = false;
                        break;
                    }
                }
            }
        }
        if complete {
            *arguments = prepared;
            let names = evaluations.bindings.iter().map(|(name, _)| {
                Expression::new_identifier(
                    SPAN,
                    oxc_ast::ast::Ident::from_in(name.as_str(), builder.allocator()),
                    builder,
                )
                .into()
            });
            let mut reserved = oxc_allocator::Vec::from_iter_in(names, builder);
            reserved.push(scope.clone_in(builder.allocator()).into());
            *scope = Expression::new_array_expression(SPAN, reserved, builder);
            result.bindings = evaluations.bindings;
        }
    }
    distribute_arguments(builder, arguments, refused);
    match style_arguments(builder, arguments) {
        Some(StyleArguments { classes, rules }) => {
            *arguments = oxc_allocator::Vec::from_array_in([Argument::from(rules)], builder);
            result.classes = classes;
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
