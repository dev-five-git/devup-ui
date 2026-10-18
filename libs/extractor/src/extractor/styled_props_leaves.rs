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
}

impl<'a> Leaves<'_, '_, 'a> {
    pub fn object(&mut self, object: &mut ObjectExpression<'a>) -> Result<(), PropsRulesError> {
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
                self.object(inner)?;
            } else {
                self.leaf(&mut property.value, &key)?;
            }
            property.shorthand = false;
        }
        Ok(())
    }

    fn leaf(&mut self, value: &mut Expression<'a>, key: &str) -> Result<(), PropsRulesError> {
        let builder = self.builder;
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
        let plain = values::analysis(
            builder,
            self.params,
            value.clone_in_with_semantic_ids(builder.allocator()),
        );
        if reads_unmapped_theme(&plain) {
            return Err(PropsRulesError::Theme(unmapped_theme_error(
                "styled", &plain,
            )));
        }
        if function && reads_unmapped_theme(unwrap_syntax_only(value)) {
            return Err(PropsRulesError::Theme(unmapped_theme_error(
                "styled", value,
            )));
        }
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
        self.values.push(value.take_in(builder).into());
        *value = if function {
            // A member call would incorrectly give an ordinary function the vector as `this`.
            let direct = Expression::new_sequence_expression(
                SPAN,
                oxc_allocator::Vec::from_array_in([values::zero(builder), slot], builder),
                builder,
            );
            wrap_direct_call(builder, &direct, &[identifier(builder, "rest")])
        } else if crate::utils::keeps_bare_number(key) {
            slot
        } else {
            values::length(builder, slot)
        };
        Ok(())
    }
}
