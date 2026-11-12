//! Style values written longer than the one value they always hold: a test
//! whose answer is a literal, a literal key into a literal table. Each is
//! written as that value, so the build emits only the styles it can select.
//! Only literals are dropped, whose evaluation the code cannot observe.

use crate::utils::unwrap_syntax_only;
use crate::visit::{INHERITED, own_key};
use oxc_allocator::{CloneIn, GetAllocator, TakeIn};
use oxc_ast::ast::{ArrayExpressionElement, Expression, NumberBase, ObjectPropertyKind};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::SPAN;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

#[cfg(test)]
#[path = "const_fold_tests.rs"]
mod tests;

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// Write the parts of `value` that always hold one value as that value
pub(crate) fn fold<'a>(ast: &AstBuilder<'a>, value: &mut Expression<'a>) {
    Fold { ast }.visit_expression(value);
}

struct Fold<'s, 'a> {
    ast: &'s AstBuilder<'a>,
}

impl<'a> VisitMut<'a> for Fold<'_, 'a> {
    fn visit_object_property(&mut self, property: &mut oxc_ast::ast::ObjectProperty<'a>) {
        // A `styleOrder` is reported as written
        if property
            .key
            .static_name()
            .is_none_or(|key| key != "styleOrder")
        {
            walk_mut::walk_object_property(self, property);
        }
    }

    fn visit_expression(&mut self, value: &mut Expression<'a>) {
        walk_mut::walk_expression(self, value);
        if let Some(folded) = self.folded(value) {
            *value = folded;
        }
    }
}

impl<'a> Fold<'_, 'a> {
    fn folded(&self, value: &mut Expression<'a>) -> Option<Expression<'a>> {
        match value {
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
                Some(Expression::new_boolean_literal(
                    SPAN,
                    !truthiness(&unary.argument)?,
                    self.ast,
                ))
            }
            Expression::BinaryExpression(binary) => {
                let (Expression::NumericLiteral(left), Expression::NumericLiteral(right)) = (
                    unwrap_syntax_only(&binary.left),
                    unwrap_syntax_only(&binary.right),
                ) else {
                    return None;
                };
                let number = match binary.operator {
                    BinaryOperator::Addition => left.value + right.value,
                    BinaryOperator::Subtraction => left.value - right.value,
                    BinaryOperator::Multiplication => left.value * right.value,
                    _ => return None,
                };
                // A sign or an exponent is written as other code
                (number.is_finite() && number.is_sign_positive()).then(|| {
                    Expression::new_numeric_literal(
                        SPAN,
                        number,
                        None,
                        NumberBase::Decimal,
                        self.ast,
                    )
                })
            }
            Expression::ConditionalExpression(branch) => Some(if truthiness(&branch.test)? {
                branch.consequent.take_in(self.ast)
            } else {
                branch.alternate.take_in(self.ast)
            }),
            Expression::LogicalExpression(branch) => {
                let left = match branch.operator {
                    LogicalOperator::And => !truthiness(&branch.left)?,
                    LogicalOperator::Or => truthiness(&branch.left)?,
                    LogicalOperator::Coalesce => !nullish(&branch.left)?,
                };
                Some(if left {
                    branch.left.take_in(self.ast)
                } else {
                    branch.right.take_in(self.ast)
                })
            }
            Expression::SequenceExpression(sequence) => {
                let (last, before) = sequence.expressions.split_last_mut()?;
                before
                    .iter()
                    .all(|value| truthiness(value).is_some())
                    .then(|| last.take_in(self.ast))
            }
            Expression::ComputedMemberExpression(member) if !member.optional => {
                let member = &mut **member;
                let table = unwrap_syntax_only(&member.object);
                match crate::utils::unwrap_syntax_only_mut(&mut member.expression) {
                    // Either key reads the table the same, when reading it
                    // evaluates nothing to run after the test
                    Expression::ConditionalExpression(key) => {
                        let consequent = self.select(table, &key_name(&key.consequent)?, true)?;
                        let alternate = self.select(table, &key_name(&key.alternate)?, true)?;
                        Some(Expression::new_conditional_expression(
                            SPAN,
                            key.test.take_in(self.ast),
                            consequent,
                            alternate,
                            self.ast,
                        ))
                    }
                    key => self.select(table, &key_name(key)?, false),
                }
            }
            Expression::StaticMemberExpression(member) if !member.optional => self.select(
                unwrap_syntax_only(&member.object),
                member.property.name.as_str(),
                false,
            ),
            _ => None,
        }
    }

    /// What `table` holds at `key`, when reading the rest of it evaluates
    /// only literals, which can be left out: all of it, when `whole`
    fn select(&self, table: &Expression<'a>, key: &str, whole: bool) -> Option<Expression<'a>> {
        let mut entries = Vec::new();
        let selected = match table {
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    entries.push(match element {
                        ArrayExpressionElement::Elision(_) => None,
                        element => Some(element.as_expression()?),
                    });
                }
                match key.parse::<usize>() {
                    Ok(index) if index.to_string() == key => Some(index),
                    // An array inherits no name that is written as a number
                    _ if key.starts_with(|c: char| c.is_ascii_digit() || c == '-') => None,
                    _ => return None,
                }
            }
            Expression::ObjectExpression(object) => {
                let mut keys = Vec::new();
                own_entries(object, &mut keys, &mut entries)?;
                let selected = keys.iter().rposition(|own| own == key);
                if selected.is_none() && INHERITED.contains(&key) {
                    return None;
                }
                selected
            }
            _ => return None,
        };
        let value = selected.and_then(|index| entries.get(index).copied().flatten());
        entries
            .iter()
            .enumerate()
            .all(|(index, entry)| {
                (!whole && Some(index) == selected) || entry.is_none_or(|entry| pure(entry))
            })
            .then(|| match value {
                Some(value) => value.clone_in_with_semantic_ids(self.ast.allocator()),
                None => self.nothing(),
            })
    }

    /// `void 0`, which no binding can rename
    fn nothing(&self) -> Expression<'a> {
        Expression::new_unary_expression(
            SPAN,
            UnaryOperator::Void,
            Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, self.ast),
            self.ast,
        )
    }
}

/// Whether the literal `value` is truthy
fn truthiness(value: &Expression<'_>) -> Option<bool> {
    match unwrap_syntax_only(value) {
        Expression::BooleanLiteral(literal) => Some(literal.value),
        Expression::NullLiteral(_) => Some(false),
        Expression::NumericLiteral(literal) => {
            Some(literal.value != 0.0 && !literal.value.is_nan())
        }
        Expression::StringLiteral(literal) => Some(!literal.value.is_empty()),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => {
            Some(template.quasis.iter().any(|quasi| {
                quasi
                    .value
                    .cooked
                    .as_ref()
                    .is_none_or(|text| !text.is_empty())
            }))
        }
        Expression::UnaryExpression(unary) => match unary.operator {
            UnaryOperator::Void => truthiness(&unary.argument).map(|_| false),
            UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus => {
                match unwrap_syntax_only(&unary.argument) {
                    Expression::NumericLiteral(_) => truthiness(&unary.argument),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// Whether the literal `value` is `null` or `undefined`
fn nullish(value: &Expression<'_>) -> Option<bool> {
    truthiness(value)?;
    Some(match unwrap_syntax_only(value) {
        Expression::NullLiteral(_) => true,
        Expression::UnaryExpression(unary) => unary.operator == UnaryOperator::Void,
        _ => false,
    })
}

/// The keys and values an object literal holds as its own, in the order
/// written, with those of the object literals it spreads
fn own_entries<'s, 'a>(
    object: &'s oxc_ast::ast::ObjectExpression<'a>,
    keys: &mut Vec<String>,
    values: &mut Vec<Option<&'s Expression<'a>>>,
) -> Option<()> {
    for property in &object.properties {
        match property {
            ObjectPropertyKind::ObjectProperty(written) => {
                keys.push(own_key(property)?);
                values.push(Some(&written.value));
            }
            ObjectPropertyKind::SpreadProperty(spread) => {
                match unwrap_syntax_only(&spread.argument) {
                    Expression::ObjectExpression(inner) => own_entries(inner, keys, values)?,
                    _ => return None,
                }
            }
        }
    }
    Some(())
}

/// Whether evaluating `value` evaluates nothing but literals
fn pure(value: &Expression<'_>) -> bool {
    match unwrap_syntax_only(value) {
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::Elision(_) => true,
            element => element.as_expression().is_some_and(pure),
        }),
        Expression::ObjectExpression(object) => {
            let mut values = Vec::new();
            own_entries(object, &mut Vec::new(), &mut values).is_some()
                && values.into_iter().flatten().all(pure)
        }
        value => truthiness(value).is_some(),
    }
}

/// The property name a literal key reads, spelled as JavaScript spells it
fn key_name(key: &Expression<'_>) -> Option<String> {
    let integer = |number: f64| {
        (number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER)
            .then(|| format!("{}", number as i64))
    };
    match unwrap_syntax_only(key) {
        Expression::StringLiteral(text) => text.value.as_str().map(str::to_string),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|text| text.as_str().map(str::to_string)),
        Expression::NumericLiteral(number) => integer(number.value),
        Expression::UnaryExpression(unary) => {
            match (unary.operator, unwrap_syntax_only(&unary.argument)) {
                (UnaryOperator::UnaryPlus, Expression::NumericLiteral(number)) => {
                    integer(number.value)
                }
                (UnaryOperator::UnaryNegation, Expression::NumericLiteral(number)) => {
                    integer(-number.value)
                }
                _ => None,
            }
        }
        _ => None,
    }
}
