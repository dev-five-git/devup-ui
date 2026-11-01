use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::SPAN;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};

use super::super::{
    StyledBindings,
    rule_choices::{self, Choice},
};
use super::{PropsRulesError, capture};
use crate::utils::unwrap_syntax_only;

pub(super) trait Captures<'a> {
    fn object(
        &mut self,
        source: &oxc_ast::ast::ObjectExpression<'a>,
        local: &str,
    ) -> Result<(Expression<'a>, Vec<Expression<'a>>), PropsRulesError>;
    fn discarded_object(&self, source: &Expression<'a>) -> Result<Expression<'a>, PropsRulesError>;
    fn theme(&self, source: &Expression<'a>) -> Result<(), PropsRulesError>;
}

pub(super) struct Branches<'b, 'p, 'a, 's, C> {
    pub builder: &'b AstBuilder<'a>,
    pub name: &'p str,
    pub leaves: C,
    pub local: &'p str,
    pub bindings: &'p StyledBindings<'s>,
    pub rules: Vec<Expression<'a>>,
}

fn facts(choice: Choice<'_>) -> (Option<bool>, Option<bool>) {
    match choice {
        Choice::Rules { truthy, .. } => (Some(truthy), Some(false)),
        Choice::Scalar {
            truthy, nullish, ..
        } => (truthy, nullish),
        Choice::Branch { yes, no, .. } => {
            let yes = facts(*yes);
            let no = facts(*no);
            (
                if yes.0 == no.0 { yes.0 } else { None },
                if yes.1 == no.1 { yes.1 } else { None },
            )
        }
    }
}

impl<'a, C: Captures<'a>> Branches<'_, '_, 'a, '_, C> {
    pub fn schedule(&mut self, source: &Expression<'a>) -> Result<Expression<'a>, PropsRulesError> {
        let builder = self.builder;
        let source = unwrap_syntax_only(source);
        if let Some(styles) = self.bindings.styles(source) {
            let truthy = !styles.is_empty();
            self.rules
                .push(source.clone_in_with_semantic_ids(builder.allocator()));
            return Ok(self.selected(Vec::new(), truthy));
        }
        match source {
            Expression::ObjectExpression(object) => {
                let (skeleton, values) = self.leaves.object(object, self.local)?;
                self.rules.push(skeleton);
                Ok(self.selected(values, true))
            }
            Expression::ConditionalExpression(branch) => {
                self.leaves.theme(&branch.test)?;
                let yes = self.schedule(&branch.consequent)?;
                let no = self.schedule(&branch.alternate)?;
                Ok(Expression::new_conditional_expression(
                    branch.span,
                    branch.test.clone_in_with_semantic_ids(builder.allocator()),
                    yes,
                    no,
                    builder,
                ))
            }
            Expression::LogicalExpression(logical) => {
                let (truthy, nullish) = facts(
                    rule_choices::read(builder, &logical.left, self.bindings)
                        .map_err(|_| PropsRulesError::Rules)?,
                );
                let right = match logical.operator {
                    LogicalOperator::And => truthy,
                    LogicalOperator::Or => truthy.map(|value| !value),
                    LogicalOperator::Coalesce => nullish,
                };
                let start = self.rules.len();
                let left = if right == Some(true)
                    && let Expression::ObjectExpression(object) = unwrap_syntax_only(&logical.left)
                {
                    super::discarded::schedule(builder, &mut self.leaves, object, self.local)?
                } else {
                    self.schedule(&logical.left)?
                };
                match right {
                    Some(false) => Ok(left),
                    Some(true) => {
                        self.rules.truncate(start);
                        let right = self.schedule(&logical.right)?;
                        Ok(Expression::new_sequence_expression(
                            SPAN,
                            oxc_allocator::Vec::from_array_in([left, right], builder),
                            builder,
                        ))
                    }
                    None => {
                        let right = self.schedule(&logical.right)?;
                        Ok(Expression::new_logical_expression(
                            logical.span,
                            left,
                            logical.operator,
                            right,
                            builder,
                        ))
                    }
                }
            }
            _ => {
                self.leaves.theme(source)?;
                Ok(Expression::new_sequence_expression(
                    SPAN,
                    oxc_allocator::Vec::from_array_in(
                        [
                            capture::assign(
                                builder,
                                capture::slot(builder, self.local, 0),
                                capture::discriminator(builder, 0),
                            ),
                            source.clone_in_with_semantic_ids(builder.allocator()),
                        ],
                        builder,
                    ),
                    builder,
                ))
            }
        }
    }

    fn selected(&self, mut values: Vec<Expression<'a>>, truthy: bool) -> Expression<'a> {
        let builder = self.builder;
        values.push(capture::assign(
            builder,
            capture::slot(builder, self.local, 0),
            capture::discriminator(builder, self.rules.len()),
        ));
        values.push(Expression::new_boolean_literal(SPAN, truthy, builder));
        Expression::new_sequence_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(values, builder),
            builder,
        )
    }

    pub fn skeleton(self) -> Expression<'a> {
        let builder = self.builder;
        let mut result = Expression::new_null_literal(SPAN, builder);
        for (index, rules) in self.rules.into_iter().enumerate().rev() {
            let test = Expression::new_binary_expression(
                SPAN,
                Expression::ComputedMemberExpression(capture::slot(builder, self.name, 0)),
                BinaryOperator::StrictEquality,
                capture::discriminator(builder, index + 1),
                builder,
            );
            result = Expression::new_conditional_expression(SPAN, test, rules, result, builder);
        }
        result
    }
}

pub(super) fn returned<'a>(
    builder: &AstBuilder<'a>,
    schedule: Expression<'a>,
    local: &str,
) -> Expression<'a> {
    Expression::new_sequence_expression(
        SPAN,
        oxc_allocator::Vec::from_array_in(
            [
                schedule,
                Expression::new_identifier(
                    SPAN,
                    oxc_ast::ast::Ident::from_in(local, builder.allocator()),
                    builder,
                ),
            ],
            builder,
        ),
        builder,
    )
}
