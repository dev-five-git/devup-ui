//! Capture authored evaluations before rule-array composition duplicates controllers.

use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKind};
use oxc_span::SPAN;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};

use super::styled_record_evaluation::Evaluations;
use crate::utils::{
    Suspends, get_string_by_literal_expression, unwrap_syntax_only, unwrap_syntax_only_mut,
};

/// Arrays here compose rules, rather than selecting responsive property values.
pub(crate) fn composes_array_under_condition(expression: &Expression<'_>) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::ArrayExpression(array) => array.elements.iter().any(|element| {
            element
                .as_expression()
                .is_some_and(composes_array_under_condition)
        }),
        Expression::ConditionalExpression(choice) => [&choice.consequent, &choice.alternate]
            .into_iter()
            .any(|side| {
                matches!(unwrap_syntax_only(side), Expression::ArrayExpression(_))
                    || composes_array_under_condition(side)
            }),
        Expression::LogicalExpression(logical) => {
            [&logical.left, &logical.right].into_iter().any(|side| {
                matches!(unwrap_syntax_only(side), Expression::ArrayExpression(_))
                    || composes_array_under_condition(side)
            })
        }
        _ => false,
    }
}

/// The caller first checks the original composition, so captures cannot turn a
/// refused call, spread or controller into an accepted identifier.
pub(crate) fn prepare<'a>(
    reads: &mut Evaluations<'_, 'a>,
    source: &Expression<'a>,
    known: impl Fn(&Expression<'a>) -> bool,
) -> Option<Expression<'a>> {
    let mut suspends = Suspends::default();
    oxc_ast_visit::Visit::visit_expression(&mut suspends, source);
    if suspends.found {
        return None;
    }
    let mut result = source.clone_in_with_semantic_ids(reads.builder.allocator());
    let guard = reads.boolean(true);
    Preparation { reads, known }.value(&mut result, &guard)?;
    Some(result)
}

struct Preparation<'r, 'b, 'a, K> {
    reads: &'r mut Evaluations<'b, 'a>,
    known: K,
}

impl<'a, K: Fn(&Expression<'a>) -> bool> Preparation<'_, '_, 'a, K> {
    fn capture(&mut self, value: &mut Expression<'a>, guard: &Expression<'a>) {
        if get_string_by_literal_expression(value).is_none()
            && !matches!(unwrap_syntax_only(value), Expression::NullLiteral(_))
            && !matches!(unwrap_syntax_only(value), Expression::Identifier(identifier)
                if matches!(identifier.name.as_str(), "undefined" | "NaN" | "Infinity"))
            && !(self.known)(unwrap_syntax_only(value))
        {
            *value = self.reads.capture(value, guard);
        }
    }

    fn value(&mut self, value: &mut Expression<'a>, guard: &Expression<'a>) -> Option<()> {
        if (self.known)(unwrap_syntax_only(value))
            || get_string_by_literal_expression(value).is_some()
        {
            return Some(());
        }
        let builder = self.reads.builder;
        match unwrap_syntax_only_mut(value) {
            Expression::ConditionalExpression(choice) => {
                self.capture(&mut choice.test, guard);
                let yes = self
                    .reads
                    .choose(guard, &choice.test, &self.reads.boolean(false));
                let no = self.reads.choose(
                    guard,
                    &self.reads.choose(
                        &choice.test,
                        &self.reads.boolean(false),
                        &self.reads.boolean(true),
                    ),
                    &self.reads.boolean(false),
                );
                self.value(&mut choice.consequent, &yes)?;
                self.value(&mut choice.alternate, &no)?;
            }
            Expression::LogicalExpression(logical) => {
                // A rule object/array is truthy, but evaluating its authored
                // leaves still precedes everything after this part.
                let takes_right = if matches!(
                    unwrap_syntax_only(&logical.left),
                    Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
                ) {
                    self.value(&mut logical.left, guard)?;
                    self.reads.boolean(logical.operator == LogicalOperator::And)
                } else {
                    if !(self.known)(unwrap_syntax_only(&logical.left)) {
                        self.capture(&mut logical.left, guard);
                    }
                    match logical.operator {
                        LogicalOperator::And => logical.left.clone_in(builder.allocator()),
                        LogicalOperator::Or => self.reads.choose(
                            &logical.left,
                            &self.reads.boolean(false),
                            &self.reads.boolean(true),
                        ),
                        LogicalOperator::Coalesce => Expression::new_binary_expression(
                            SPAN,
                            logical.left.clone_in(builder.allocator()),
                            BinaryOperator::Equality,
                            Expression::new_null_literal(SPAN, builder),
                            builder,
                        ),
                    }
                };
                let right = self
                    .reads
                    .choose(guard, &takes_right, &self.reads.boolean(false));
                self.value(&mut logical.right, &right)?;
            }
            Expression::ArrayExpression(array) => {
                for element in &mut array.elements {
                    match element {
                        oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                            self.value(&mut spread.argument, guard)?;
                        }
                        element => {
                            if let Some(value) = element.as_expression_mut() {
                                self.value(value, guard)?;
                            }
                        }
                    }
                }
            }
            Expression::ObjectExpression(object) => {
                for property in &mut object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.method || property.kind != PropertyKind::Init {
                                return None;
                            }
                            // Keep authored keys and literal lookup tables intact:
                            // the extractor still owns their policy and atom choices.
                            self.value(&mut property.value, guard)?;
                            property.shorthand = false;
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.value(&mut spread.argument, guard)?;
                        }
                    }
                }
            }
            Expression::ComputedMemberExpression(member)
                if matches!(
                    unwrap_syntax_only(&member.object),
                    Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
                ) =>
            {
                self.value(&mut member.object, guard)?;
                self.capture(&mut member.expression, guard);
            }
            Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_) => {}
            Expression::Identifier(identifier)
                if matches!(identifier.name.as_str(), "undefined" | "NaN" | "Infinity") => {}
            _ => self.capture(value, guard),
        }
        Some(())
    }
}
