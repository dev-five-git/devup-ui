use super::Preparation;
use crate::utils::{
    get_key_by_literal_expression, get_str_by_property_key, js_number_string, unwrap_syntax_only,
};
use oxc_allocator::{CloneIn, FromIn, GetAllocator, Vec as ArenaVec};
use oxc_ast::ast::{
    ComputedMemberExpression, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str,
};
use oxc_span::SPAN;
use oxc_syntax::number::NumberBase;

/// Keys outside this exact literal subset retain native whole-member evaluation.
fn literal_key(source: &Expression<'_>) -> Option<String> {
    match unwrap_syntax_only(source) {
        Expression::StringLiteral(value) => value.value.as_str().map(str::to_string),
        Expression::BooleanLiteral(value) => Some(value.value.to_string()),
        Expression::NumericLiteral(value)
            if value.value.fract() == 0.0 && (0.0..=f64::from(u32::MAX)).contains(&value.value) =>
        {
            Some(js_number_string(value.value))
        }
        _ => None,
    }
}

pub(super) fn property_key(key: &PropertyKey<'_>) -> Option<String> {
    match key {
        PropertyKey::StaticIdentifier(value) => Some(value.name.to_string()),
        _ => key.as_expression().and_then(literal_key),
    }
}

/// The legacy direct extractor must agree with native keys on both sides of a lookup.
pub(super) fn direct_lookup(object: &Expression<'_>, index: &Expression<'_>) -> bool {
    let Some(key) = literal_key(index) else {
        return false;
    };
    if get_key_by_literal_expression(index).as_deref() != Some(key.as_str()) {
        return false;
    }
    match object {
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(property)
                if property_key(&property.key).as_deref()
                    == get_str_by_property_key(&property.key).as_deref())
        }),
        Expression::ArrayExpression(_) => key
            .parse::<u32>()
            .is_ok_and(|index| index.to_string() == key),
        _ => false,
    }
}

impl<'a> Preparation<'_, '_, 'a> {
    pub(super) fn select_lookup(
        &mut self,
        member: &mut ComputedMemberExpression<'a>,
        guard: &Expression<'a>,
    ) -> Option<()> {
        let builder = self.reads.builder;
        let entries: Vec<_> = match &member.object {
            Expression::ObjectExpression(object) => object
                .properties
                .iter()
                .map(|property| match property {
                    ObjectPropertyKind::ObjectProperty(property) => Some((
                        property_key(&property.key)?,
                        property.value.clone_in(builder.allocator()),
                    )),
                    ObjectPropertyKind::SpreadProperty(_) => None,
                })
                .collect::<Option<_>>()?,
            Expression::ArrayExpression(array) => {
                let mut entries: Vec<_> = array
                    .elements
                    .iter()
                    .enumerate()
                    .filter_map(|(index, element)| {
                        element
                            .as_expression()
                            .map(|value| (index.to_string(), value.clone_in(builder.allocator())))
                    })
                    .collect();
                let length = u32::try_from(array.elements.len()).ok()?;
                entries.push((
                    "length".to_string(),
                    Expression::new_numeric_literal(
                        SPAN,
                        f64::from(length),
                        None,
                        NumberBase::Decimal,
                        builder,
                    ),
                ));
                entries
            }
            _ => return None,
        };
        let mut selector = ArenaVec::new_in(builder);
        let mut values = ArenaVec::new_in(builder);
        // No inherited selector result can reach the later class lookup or be coerced again.
        selector.push(ObjectPropertyKind::new_object_property(
            SPAN,
            PropertyKind::Init,
            PropertyKey::new_static_identifier(SPAN, "__proto__", builder),
            Expression::new_null_literal(SPAN, builder),
            false,
            false,
            false,
            builder,
        ));
        for (ordinal, (key, value)) in entries.into_iter().enumerate() {
            let ordinal = u32::try_from(ordinal).ok()?;
            let key = Expression::new_string_literal(
                SPAN,
                Str::from_in(key.as_str(), builder.allocator()),
                None,
                builder,
            );
            selector.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::from(key),
                Expression::new_numeric_literal(
                    SPAN,
                    f64::from(ordinal),
                    None,
                    NumberBase::Decimal,
                    builder,
                ),
                false,
                false,
                true,
                builder,
            ));
            let key = Expression::new_string_literal(
                SPAN,
                Str::from_in(ordinal.to_string().as_str(), builder.allocator()),
                None,
                builder,
            );
            values.push(ObjectPropertyKind::new_object_property(
                SPAN,
                PropertyKind::Init,
                PropertyKey::from(key),
                value,
                false,
                false,
                false,
                builder,
            ));
        }
        let selection = Expression::ComputedMemberExpression(ComputedMemberExpression::boxed(
            SPAN,
            Expression::new_object_expression(SPAN, selector, builder),
            member.expression.clone_in(builder.allocator()),
            false,
            builder,
        ));
        member.expression = self.reads.capture(&selection, guard);
        member.object = Expression::new_object_expression(SPAN, values, builder);
        Some(())
    }
}

#[cfg(test)]
#[path = "styled_record_selection_tests.rs"]
mod tests;
