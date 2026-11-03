use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};

use super::{StyledBindings, identifier, props, records};
use crate::{
    extractor::{rule_array_evaluation, styled_record_evaluation::Evaluations},
    utils::{readable_code, unwrap_syntax_only, wrap_direct_call},
};

pub(super) struct Schedule<'b, 'a> {
    pub builder: &'b AstBuilder<'a>,
    scope: Expression<'a>,
    pub bindings: Vec<(String, Expression<'a>)>,
}

impl<'b, 'a> Schedule<'b, 'a> {
    pub fn new(builder: &'b AstBuilder<'a>, scope: &Expression<'a>) -> Self {
        Self {
            builder,
            scope: scope.clone_in(builder.allocator()),
            bindings: Vec::new(),
        }
    }

    pub fn scalar(&mut self, expression: &mut Expression<'a>) -> Result<(), (u32, String)> {
        if crate::css_utils::reads_unmapped_theme(expression) {
            return Err(crate::css_utils::unmapped_theme_error("styled", expression));
        }
        if crate::utils::get_string_by_literal_expression(expression).is_some()
            || crate::utils::js_number_literal(expression).is_some()
            || matches!(
                unwrap_syntax_only(expression),
                Expression::NullLiteral(_) | Expression::BooleanLiteral(_)
            )
            || crate::css_utils::theme_var_reference(expression).is_some()
        {
            return Ok(());
        }
        let value = match unwrap_syntax_only(expression) {
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
                wrap_direct_call(
                    self.builder,
                    expression,
                    &[identifier(self.builder, "rest")],
                )
            }
            _ => expression.clone_in_with_semantic_ids(self.builder.allocator()),
        };
        let mut evaluations = Evaluations::new(self.builder, readable_code(&self.scope));
        *expression = evaluations.capture(&value, &evaluations.boolean(true));
        self.commit(evaluations.bindings);
        Ok(())
    }

    pub fn rules(
        &mut self,
        expression: &mut Expression<'a>,
        bindings: &StyledBindings<'_>,
    ) -> Result<(), (u32, String)> {
        let mut evaluations = Evaluations::new(self.builder, readable_code(&self.scope));
        match unwrap_syntax_only(expression) {
            Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ObjectExpression(_) => {
                match props::lower(&mut evaluations, expression, Some(bindings))? {
                    Some(props::Plan::Whole(invocation)) => evaluations
                        .bindings
                        .push((invocation.name, invocation.value)),
                    Some(props::Plan::Properties(original)) => {
                        if let Some(prepared) = crate::extractor::styled_record::prepare_callbacks(
                            &mut evaluations,
                            &original,
                        ) {
                            *expression = prepared;
                        }
                    }
                    None => {}
                }
            }
            _ => props::lower_direct(&mut evaluations, expression, (bindings, &self.scope))?,
        }
        if !matches!(
            unwrap_syntax_only(expression),
            Expression::ObjectExpression(_)
        ) && let Some(prepared) =
            rule_array_evaluation::prepare(&mut evaluations, expression, |value| {
                bindings.styles(value).is_some()
            })
        {
            *expression = prepared;
        }
        self.commit(evaluations.bindings);
        records::prepare(
            self.builder,
            expression,
            (&mut self.scope, &mut self.bindings),
        );
        Ok(())
    }

    fn commit(&mut self, bindings: Vec<(String, Expression<'a>)>) {
        records::reserve(self.builder, &mut self.scope, &bindings);
        self.bindings.extend(bindings);
    }
}

#[cfg(test)]
#[path = "template_planning_tests.rs"]
mod tests;
