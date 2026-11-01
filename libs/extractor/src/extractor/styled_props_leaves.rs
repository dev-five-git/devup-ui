use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::{
    ast::{
        ArrayExpressionElement, ComputedMemberExpression, Expression, FormalParameters,
        ObjectExpression, ObjectPropertyKind, PropertyKind, Str,
    },
    builder::AstBuilder,
};
use oxc_span::SPAN;

use super::{super::identifier, PropsRulesError, values};
#[path = "styled_props_shape.rs"]
pub(super) mod shape;
use crate::{
    css_utils::{
        reads_unmapped_theme, theme_leaf_reference, theme_var_reference, unmapped_theme_error,
    },
    utils::{
        get_str_by_property_key, unwrap_syntax_only, unwrap_syntax_only_mut, wrap_direct_call,
    },
};

pub(super) struct Leaves<'b, 'p, 'a> {
    pub builder: &'b AstBuilder<'a>,
    pub params: &'p FormalParameters<'a>,
    pub name: &'p str,
    pub values: oxc_allocator::Vec<'a, ArrayExpressionElement<'a>>,
    pub local: Option<String>,
    pub guard: Option<Expression<'a>>,
}

impl<'a> Leaves<'_, '_, 'a> {
    pub fn object(&mut self, object: &mut ObjectExpression<'a>) -> Result<(), PropsRulesError> {
        let start = self.values.len();
        let result = self.object_inner(object);
        if result.is_err() {
            self.values.truncate(start);
        }
        result
    }

    fn object_inner(&mut self, object: &mut ObjectExpression<'a>) -> Result<(), PropsRulesError> {
        for property in &mut object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return Err(PropsRulesError::Rules);
            };
            if property.computed || property.method || property.kind != PropertyKind::Init {
                return Err(PropsRulesError::Rules);
            }
            let key = get_str_by_property_key(&property.key).ok_or(PropsRulesError::Rules)?;
            if let Expression::ObjectExpression(inner) = unwrap_syntax_only_mut(&mut property.value)
            {
                self.object_inner(inner)?;
            } else {
                self.leaf(&mut property.value, &key)?;
            }
            property.shorthand = false;
        }
        Ok(())
    }

    fn leaf(&mut self, value: &mut Expression<'a>, key: &str) -> Result<(), PropsRulesError> {
        if self.project(value, key)? {
            return Ok(());
        }
        let builder = self.builder;
        if matches!(unwrap_syntax_only(value), Expression::BooleanLiteral(value) if !value.value) {
            *value = Expression::new_null_literal(SPAN, builder);
            return Ok(());
        }
        let function = matches!(
            unwrap_syntax_only(value),
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
        );
        if values::is_written_out(value) {
            return Ok(());
        }
        let reference = if function {
            theme_var_reference(unwrap_syntax_only(value))
        } else {
            theme_leaf_reference(self.params, unwrap_syntax_only(value))
        };
        if let Some(reference) = reference {
            *value = Expression::new_string_literal(
                SPAN,
                Str::from_in(reference.as_str(), builder.allocator()),
                None,
                builder,
            );
            return Ok(());
        }
        self.theme(value)?;
        let index = self.values.len().to_string();
        let slot = Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
            SPAN,
            Expression::new_identifier(SPAN, Str::from_in(self.name, builder.allocator()), builder),
            Expression::new_string_literal(
                SPAN,
                Str::from_in(index.as_str(), builder.allocator()),
                None,
                builder,
            ),
            false,
            builder,
        ));
        let source = value.take_in(builder);
        let source = self.guarded(source);
        self.values.push(source.into());
        *value = if function {
            // A member call would incorrectly give an ordinary function the vector as `this`.
            let direct = Expression::new_sequence_expression(
                SPAN,
                oxc_allocator::Vec::from_array_in([values::zero(builder), slot], builder),
                builder,
            );
            wrap_direct_call(builder, &direct, &[identifier(builder, "rest")])
        } else {
            values::captured(builder, key, slot)
        };
        Ok(())
    }

    fn theme(&self, value: &Expression<'a>) -> Result<(), PropsRulesError> {
        let function = matches!(
            unwrap_syntax_only(value),
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
        );
        let plain = values::analysis(
            self.builder,
            self.params,
            value.clone_in_with_semantic_ids(self.builder.allocator()),
        );
        let unmapped = if reads_unmapped_theme(&plain) {
            Some(&plain)
        } else if function && reads_unmapped_theme(unwrap_syntax_only(value)) {
            Some(value)
        } else {
            None
        };
        if let Some(unmapped) = unmapped {
            return Err(PropsRulesError::Theme(unmapped_theme_error(
                "styled", unmapped,
            )));
        }
        Ok(())
    }

    fn guarded(&self, source: Expression<'a>) -> Expression<'a> {
        match &self.guard {
            Some(guard) => Expression::new_conditional_expression(
                SPAN,
                guard.clone_in(self.builder.allocator()),
                source,
                Expression::new_identifier(SPAN, "undefined", self.builder),
                self.builder,
            ),
            None => source,
        }
    }
}

impl<'a> super::branches::Captures<'a> for Leaves<'_, '_, 'a> {
    fn object(
        &mut self,
        source: &ObjectExpression<'a>,
        local: &str,
    ) -> Result<(Expression<'a>, Vec<Expression<'a>>), PropsRulesError> {
        self.local = Some(local.to_string());
        let builder = self.builder;
        let mut skeleton = source.clone_in_with_semantic_ids(builder.allocator());
        let start = self.values.len();
        Leaves::object(self, &mut skeleton)?;
        let values = self
            .values
            .iter()
            .enumerate()
            .skip(start)
            .map(|(index, value)| {
                super::capture::assign(
                    builder,
                    super::capture::slot(builder, local, index),
                    value
                        .to_expression()
                        .clone_in_with_semantic_ids(builder.allocator()),
                )
            })
            .collect();
        Ok((
            Expression::ObjectExpression(oxc_allocator::Box::new_in(skeleton, builder)),
            values,
        ))
    }

    fn discarded_object(&self, source: &Expression<'a>) -> Result<Expression<'a>, PropsRulesError> {
        super::discarded::mapped(self.builder, self.params, source)
    }

    fn theme(&self, source: &Expression<'a>) -> Result<(), PropsRulesError> {
        Leaves::theme(self, source)
    }
}
