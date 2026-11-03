use super::{Leaves, PropsRulesError};
use crate::utils::{unwrap_syntax_only, unwrap_syntax_only_mut};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKind};
use oxc_span::SPAN;
use oxc_syntax::operator::UnaryOperator;

pub(super) fn spread_data(value: &Expression<'_>, allow_object: bool) -> bool {
    let projected = |value| spread_data(value, false);
    match unwrap_syntax_only(value) {
        Expression::ObjectExpression(object) => {
            allow_object
                && object.properties.iter().all(|property| match property {
                    ObjectPropertyKind::ObjectProperty(property) => {
                        !property.computed
                            && !property.method
                            && property.kind == PropertyKind::Init
                            && property
                                .key
                                .static_name()
                                .is_some_and(|key| key != "__proto__")
                            && spread_data(&property.value, true)
                    }
                    ObjectPropertyKind::SpreadProperty(spread) => {
                        matches!(
                            unwrap_syntax_only(&spread.argument),
                            Expression::ObjectExpression(_)
                        ) && spread_data(&spread.argument, true)
                    }
                })
        }
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| {
            matches!(element, ArrayExpressionElement::Elision(_))
                || element.as_expression().is_some_and(projected)
        }),
        Expression::ConditionalExpression(branch) => {
            projected(&branch.consequent) && projected(&branch.alternate)
        }
        Expression::LogicalExpression(logical) => {
            projected(&logical.left) && projected(&logical.right)
        }
        Expression::SequenceExpression(sequence) => sequence.expressions.iter().all(projected),
        _ => true,
    }
}

pub(crate) fn needs_projection(value: &Expression<'_>) -> bool {
    match unwrap_syntax_only(value) {
        Expression::ConditionalExpression(branch) => {
            omitted(&branch.consequent)
                || omitted(&branch.alternate)
                || needs_projection(&branch.consequent)
                || needs_projection(&branch.alternate)
        }
        Expression::ArrayExpression(_) => true,
        Expression::ObjectExpression(object) => {
            object.properties.iter().any(|property| match property {
                oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) => {
                    needs_projection(&property.value)
                }
                oxc_ast::ast::ObjectPropertyKind::SpreadProperty(spread) => {
                    needs_projection(&spread.argument)
                }
            })
        }
        _ => false,
    }
}

impl<'a> Leaves<'_, '_, 'a> {
    pub(super) fn project(
        &mut self,
        value: &mut Expression<'a>,
        key: &str,
    ) -> Result<bool, PropsRulesError> {
        let builder = self.builder;
        match (unwrap_syntax_only_mut(value), self.local.as_deref()) {
            (Expression::ArrayExpression(array), _)
                if !array.elements.iter().any(|element| {
                    matches!(
                        element,
                        oxc_ast::ast::ArrayExpressionElement::SpreadElement(_)
                    )
                }) =>
            {
                for element in &mut array.elements {
                    if let Some(value) = element.as_expression_mut() {
                        self.leaf(value, key)?;
                    }
                }
                Ok(true)
            }
            (Expression::ConditionalExpression(branch), Some(local))
                if omitted(&branch.consequent)
                    || omitted(&branch.alternate)
                    || needs_projection(&branch.consequent)
                    || needs_projection(&branch.alternate) =>
            {
                self.theme(&branch.test)?;
                let index = self.values.len();
                let test =
                    self.guarded(branch.test.clone_in_with_semantic_ids(builder.allocator()));
                self.values.push(test.into());
                let local_test = Expression::ComputedMemberExpression(super::super::capture::slot(
                    builder, local, index,
                ));
                let parent = self.guard.take();
                self.guard = Some(under(
                    builder,
                    parent.as_ref(),
                    local_test.clone_in(builder.allocator()),
                ));
                self.leaf(&mut branch.consequent, key)?;
                let inverted = Expression::new_unary_expression(
                    SPAN,
                    UnaryOperator::LogicalNot,
                    local_test,
                    builder,
                );
                self.guard = Some(under(builder, parent.as_ref(), inverted));
                self.leaf(&mut branch.alternate, key)?;
                self.guard = parent;
                branch.test = Expression::ComputedMemberExpression(super::super::capture::slot(
                    builder, self.name, index,
                ));
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

fn omitted(value: &Expression<'_>) -> bool {
    match unwrap_syntax_only(value) {
        Expression::NullLiteral(_) => true,
        Expression::BooleanLiteral(value) => !value.value,
        _ => false,
    }
}

fn under<'a>(
    builder: &oxc_ast::builder::AstBuilder<'a>,
    parent: Option<&Expression<'a>>,
    test: Expression<'a>,
) -> Expression<'a> {
    match parent {
        Some(parent) => Expression::new_conditional_expression(
            SPAN,
            parent.clone_in(builder.allocator()),
            test,
            Expression::new_boolean_literal(SPAN, false, builder),
            builder,
        ),
        None => test,
    }
}
