use super::super::class_names_parts::{LocalKnownPart, LocalParts};
use super::{
    ClassPayload, CloneIn, Expression, ExtractStyleProp, GetAllocator, GetSpan, KnownStyles,
    LocalRules, LocalSource, ObjectPropertyKind, Text, unplaced_error,
};
use crate::composition::Composition;

impl<'a, S: LocalSource<'a>> LocalRules<'_, '_, 'a, S> {
    pub(in crate::visit) fn literal_scope(
        &mut self,
        rules: &Expression<'a>,
        element: Option<&str>,
    ) -> Option<Vec<ExtractStyleProp<'a, S::Class>>> {
        let Expression::ObjectExpression(object) = rules else {
            return None;
        };
        if !object.properties.iter().any(|property| matches!(property, ObjectPropertyKind::ObjectProperty(property) if property.key.static_name().is_some_and(|key| key == "__devupLiteralMixin"))) {
            return None;
        }
        let mut object = object.clone_in_with_semantic_ids(self.visitor.ast.allocator());
        let order = crate::style_order::take(&mut object, self.visitor.ast.allocator());
        let mut composition: Composition<'a, S::Class> = Default::default();
        for property in object.properties.drain(..) {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            let props = if property
                .key
                .static_name()
                .is_some_and(|key| key == "__devupLiteralMixin")
            {
                let mut parts = Vec::new();
                if LocalParts::new(self.visitor, self.source)
                    .known_parts_local(&property.value, &mut parts, Text::Classes)
                    .is_none()
                {
                    self.visitor
                        .errors
                        .push((property.value.span().start, unplaced_error(&property.value)));
                }
                let mut nested: Composition<'a, S::Class> = Default::default();
                for part in parts {
                    let props = match part {
                        LocalKnownPart::Styles(styles) => {
                            self.part_props_local(property.span.start, styles, element)
                        }
                        LocalKnownPart::Conditional {
                            test,
                            consequent,
                            alternate,
                        } => {
                            let yes =
                                self.part_props_local(property.span.start, consequent, element);
                            let no = self.part_props_local(property.span.start, alternate, element);
                            vec![ExtractStyleProp::Conditional {
                                condition: test,
                                consequent: Some(Box::new(ExtractStyleProp::StaticArray(yes))),
                                alternate: Some(Box::new(ExtractStyleProp::StaticArray(no))),
                            }]
                        }
                        LocalKnownPart::Class(expression) => vec![ExtractStyleProp::Expression {
                            expression,
                            styles: Vec::new(),
                        }],
                    };
                    nested.apply(&self.visitor.ast, props);
                }
                nested.into_props()
            } else {
                let expression = Expression::new_object_expression(
                    property.span,
                    oxc_allocator::Vec::from_array_in(
                        [ObjectPropertyKind::ObjectProperty(property)],
                        &self.visitor.ast,
                    ),
                    &self.visitor.ast,
                );
                self.part_props_local(
                    expression.span().start,
                    vec![KnownStyles::Rules(expression)],
                    element,
                )
            };
            composition.apply(&self.visitor.ast, props);
        }
        let props = composition.into_props();
        Some(match order {
            Some(Ok(order)) => crate::style_order::apply_with_payload(
                order,
                props,
                self.visitor.ast.allocator(),
                ClassPayload::clone_payload,
            ),
            Some(Err(error)) => {
                self.visitor.error_disposition.include(error.disposition);
                self.visitor.errors.push(error.diagnostic);
                props
            }
            None => props,
        })
    }
}

#[cfg(test)]
#[path = "literal_w38n_conditional.rs"]
mod literal_w38n_conditional;
#[cfg(test)]
#[path = "literal_w38n_controls.rs"]
mod literal_w38n_controls;
#[cfg(test)]
#[path = "literal_w38n_diagnostics.rs"]
mod literal_w38n_diagnostics;
#[cfg(test)]
#[path = "literal_w38n_inventory.rs"]
mod literal_w38n_inventory;
#[cfg(test)]
#[path = "literal_w38n_selection.rs"]
mod literal_w38n_selection;
#[cfg(test)]
#[path = "literal_w38n_source.rs"]
mod literal_w38n_source;
