use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, FormalParameters, ObjectExpression, ObjectPropertyKind, PropertyKind, Str},
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::SPAN;

use super::{PropsRulesError, branches::Captures, values};
use crate::{
    css_utils::{
        reads_unmapped_theme, theme_leaf_reference, theme_var_reference, unmapped_theme_error,
    },
    utils::{unwrap_syntax_only, unwrap_syntax_only_mut},
};

pub(super) fn schedule<'a>(
    builder: &AstBuilder<'a>,
    leaves: &mut impl Captures<'a>,
    object: &ObjectExpression<'a>,
    local: &str,
) -> Result<Expression<'a>, PropsRulesError> {
    match leaves.object(object, local) {
        Ok((_, values)) if values.is_empty() => {
            Ok(Expression::new_boolean_literal(SPAN, true, builder))
        }
        Ok((_, values)) => Ok(Expression::new_sequence_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(values, builder),
            builder,
        )),
        Err(PropsRulesError::Rules) => {
            leaves.discarded_object(&Expression::ObjectExpression(oxc_allocator::Box::new_in(
                object.clone_in_with_semantic_ids(builder.allocator()),
                builder,
            )))
        }
        Err(error) => Err(error),
    }
}

pub(super) fn mapped<'a>(
    builder: &AstBuilder<'a>,
    params: &FormalParameters<'a>,
    source: &Expression<'a>,
) -> Result<Expression<'a>, PropsRulesError> {
    let mut transformed = source.clone_in_with_semantic_ids(builder.allocator());
    let policy = Theme {
        builder,
        params,
        structured: false,
    };
    if let Expression::ObjectExpression(object) = unwrap_syntax_only_mut(&mut transformed) {
        policy.object(object)?;
    }
    Ok(transformed)
}

/// The admitted finite-record path maps CSS leaves, never branch conditions.
pub(super) fn mapped_rules<'a>(
    builder: &AstBuilder<'a>,
    params: &FormalParameters<'a>,
    source: &Expression<'a>,
) -> Result<Expression<'a>, PropsRulesError> {
    let mut transformed = source.clone_in_with_semantic_ids(builder.allocator());
    Theme {
        builder,
        params,
        structured: true,
    }
    .record(&mut transformed)?;
    Ok(transformed)
}

struct Theme<'b, 'p, 'a> {
    builder: &'b AstBuilder<'a>,
    params: &'p FormalParameters<'a>,
    structured: bool,
}

impl<'a> Theme<'_, '_, 'a> {
    fn object(&self, object: &mut ObjectExpression<'a>) -> Result<(), PropsRulesError> {
        for property in &mut object.properties {
            match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    if let Some(key) = property.key.as_expression() {
                        self.check(key)?;
                    }
                    if property.method || property.kind != PropertyKind::Init {
                        self.check(&property.value)?;
                    } else {
                        self.value(&mut property.value)?;
                    }
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    if self.structured {
                        self.record(&mut spread.argument)?;
                    } else {
                        self.check(&spread.argument)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn record(&self, source: &mut Expression<'a>) -> Result<(), PropsRulesError> {
        match unwrap_syntax_only_mut(source) {
            Expression::ObjectExpression(object) => self.object(object),
            Expression::ConditionalExpression(branch) => {
                self.check(&branch.test)?;
                self.record(&mut branch.consequent)?;
                self.record(&mut branch.alternate)
            }
            Expression::LogicalExpression(logical)
                if logical.operator == oxc_syntax::operator::LogicalOperator::And =>
            {
                self.check(&logical.left)?;
                self.record(&mut logical.right)
            }
            _ => self.check(source),
        }
    }

    fn value(&self, value: &mut Expression<'a>) -> Result<(), PropsRulesError> {
        if let Expression::ObjectExpression(inner) = unwrap_syntax_only_mut(value) {
            return self.object(inner);
        }
        if self.structured {
            match unwrap_syntax_only_mut(value) {
                Expression::ArrayExpression(array) => {
                    for element in &mut array.elements {
                        if let Some(value) = element.as_expression_mut() {
                            self.value(value)?;
                        }
                    }
                    return Ok(());
                }
                Expression::ConditionalExpression(branch) => {
                    self.check(&branch.test)?;
                    self.value(&mut branch.consequent)?;
                    return self.value(&mut branch.alternate);
                }
                _ => {}
            }
        }
        let source = unwrap_syntax_only(value);
        if let Some(reference) =
            theme_leaf_reference(self.params, source).or_else(|| theme_var_reference(source))
        {
            *value = Expression::new_string_literal(
                SPAN,
                Str::from_in(reference.as_str(), self.builder.allocator()),
                None,
                self.builder,
            );
            return Ok(());
        }
        self.check(value)
    }

    fn check(&self, source: &Expression<'a>) -> Result<(), PropsRulesError> {
        let analyzed = values::analysis(
            self.builder,
            self.params,
            source.clone_in_with_semantic_ids(self.builder.allocator()),
        );
        if theme_leaf_reference(self.params, unwrap_syntax_only(source)).is_some()
            || reads_unmapped_theme(&analyzed)
        {
            return Err(PropsRulesError::Theme(unmapped_theme_error(
                "styled", &analyzed,
            )));
        }
        // Outer props analysis does not bind a nested function's own props parameter.
        let mut functions = Functions(None);
        functions.visit_expression(source);
        match functions.0 {
            Some(error) => Err(PropsRulesError::Theme(error)),
            None => Ok(()),
        }
    }
}

struct Functions(Option<(u32, String)>);

#[cfg(test)]
#[path = "styled_props_discarded_tests.rs"]
mod tests;

impl<'a> Visit<'a> for Functions {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        if self.0.is_some() {
            return;
        }
        if matches!(
            expression,
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
        ) && (reads_unmapped_theme(expression) || theme_var_reference(expression).is_some())
        {
            self.0 = Some(unmapped_theme_error("styled", expression));
            return;
        }
        walk::walk_expression(self, expression);
    }
}
