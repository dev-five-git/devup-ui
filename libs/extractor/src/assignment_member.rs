use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{ComputedMemberExpression, Expression, ObjectPropertyKind};

use crate::{
    ExtractStyleProp,
    assignment_lowering::Lowering,
    utils::{
        get_number_by_literal_expression, get_string_by_literal_expression,
        get_string_by_property_key, unwrap_syntax_only,
    },
};

impl<'a> Lowering<'_, 'a> {
    pub(super) fn member(
        &self,
        source: &ComputedMemberExpression<'a>,
        styles: &mut [ExtractStyleProp<'a>],
    ) -> Expression<'a> {
        let ast = self.ast;
        let mut member = source.clone_in(ast.allocator());
        if get_string_by_literal_expression(&source.expression).is_some() {
            return self.leaf(
                &Expression::ComputedMemberExpression(oxc_allocator::Box::new_in(member, ast)),
                styles,
            );
        }
        let map = match styles {
            [ExtractStyleProp::MemberExpression { map, .. }] => Some(map),
            _ => {
                return self.leaf(
                    &Expression::ComputedMemberExpression(oxc_allocator::Box::new_in(member, ast)),
                    styles,
                );
            }
        };
        match unwrap_syntax_only(&source.object) {
            Expression::ObjectExpression(object) => {
                let mut object = object.clone_in(ast.allocator());
                let mut map = map;
                for property in &mut object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property {
                        let key = get_string_by_property_key(&property.key);
                        if key.as_deref() == Some("__proto__")
                            && !property.computed
                            && !property.shorthand
                        {
                            continue;
                        }
                        let selected = map
                            .as_mut()
                            .and_then(|map| key.as_ref().and_then(|key| map.get_mut(key)));
                        property.value = match selected {
                            Some(selected) => {
                                self.lower(&property.value, std::slice::from_mut(selected.as_mut()))
                            }
                            None => self.lower(&property.value, &mut []),
                        };
                    }
                }
                member.object = Expression::ObjectExpression(object);
                Expression::ComputedMemberExpression(oxc_allocator::Box::new_in(member, ast))
            }
            Expression::ArrayExpression(array) => {
                let mut array = array.clone_in(ast.allocator());
                let mut map = map;
                for (index, element) in array.elements.iter_mut().enumerate() {
                    if let Some(value) = element.as_expression_mut() {
                        let selected = map.as_mut().and_then(|map| map.get_mut(&index.to_string()));
                        *value = match selected {
                            Some(selected) => {
                                self.lower(value, std::slice::from_mut(selected.as_mut()))
                            }
                            None => self.lower(value, &mut []),
                        };
                    }
                }
                member.object = Expression::ArrayExpression(array);
                Expression::ComputedMemberExpression(oxc_allocator::Box::new_in(member, ast))
            }
            _ => self.leaf(
                &Expression::ComputedMemberExpression(oxc_allocator::Box::new_in(member, ast)),
                styles,
            ),
        }
    }
}

pub(super) fn selected_array(source: &Expression<'_>) -> bool {
    let Expression::ComputedMemberExpression(source) = unwrap_syntax_only(source) else {
        return false;
    };
    let key = get_string_by_literal_expression(&source.expression);
    match unwrap_syntax_only(&source.object) {
        Expression::ObjectExpression(object) => object
            .properties
            .iter()
            .rev()
            .find_map(|property| match property {
                ObjectPropertyKind::ObjectProperty(property)
                    if get_string_by_property_key(&property.key).as_deref() == key.as_deref() =>
                {
                    Some(matches!(
                        unwrap_syntax_only(&property.value),
                        Expression::ArrayExpression(_)
                    ))
                }
                _ => None,
            })
            .unwrap_or(false),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .enumerate()
            .find_map(|(index, value)| {
                (get_number_by_literal_expression(&source.expression)
                    == u32::try_from(index).ok().map(f64::from))
                .then(|| {
                    value.as_expression().is_some_and(|value| {
                        matches!(unwrap_syntax_only(value), Expression::ArrayExpression(_))
                    })
                })
            })
            .unwrap_or(false),
        _ => false,
    }
}
