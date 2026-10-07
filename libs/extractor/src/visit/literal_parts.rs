use super::{DevupVisitor, Text};
use crate::{
    ExtractStyleProp,
    composition::{Composition, KnownPart},
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_span::GetSpan;

impl<'a> DevupVisitor<'a> {
    pub(super) fn prepare_literal_css_prop(&self, value: &mut Expression<'a>) -> bool {
        if let Expression::TaggedTemplateExpression(tag) = value
            && self
                .util_type(&tag.tag)
                .is_some_and(|util| matches!(util.as_ref(), crate::util_type::UtilType::Css))
        {
            let mut literal = Expression::TemplateLiteral(oxc_allocator::Box::new_in(
                tag.quasi.clone_in_with_semantic_ids(self.ast.allocator()),
                &self.ast,
            ));
            if crate::css_utils::literal::lower_with_source(&self.ast, &mut literal, self.source) {
                *value = literal;
                return true;
            }
        }
        crate::css_utils::literal_tree::lower(
            &self.ast,
            value,
            crate::css_utils::literal_tree::Scope {
                source: self.source,
                global: false,
            },
        )
    }

    pub(super) fn literal_scope(
        &mut self,
        rules: &Expression<'a>,
        element: Option<&str>,
    ) -> Option<Vec<ExtractStyleProp<'a>>> {
        let Expression::ObjectExpression(object) = rules else {
            return None;
        };
        if !object.properties.iter().any(|property| matches!(property, ObjectPropertyKind::ObjectProperty(property) if property.key.static_name().is_some_and(|key| key == "__devupLiteralMixin"))) { return None; }
        let mut object = object.clone_in_with_semantic_ids(self.ast.allocator());
        let order = crate::style_order::take(&mut object, self.ast.allocator());
        let mut composition = Composition::default();
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
                if self
                    .known_parts(&property.value, &mut parts, Text::Classes)
                    .is_none()
                {
                    self.errors.push((
                        property.value.span().start,
                        crate::utils::unplaced_error(&property.value),
                    ));
                }
                let mut nested = Vec::new();
                for part in parts {
                    match part {
                        KnownPart::Styles(styles) => {
                            nested.extend(self.part_props(property.span.start, styles, element));
                        }
                        KnownPart::Conditional {
                            test,
                            consequent,
                            alternate,
                        } => {
                            let yes = self.part_props(property.span.start, consequent, element);
                            let no = self.part_props(property.span.start, alternate, element);
                            nested.push(ExtractStyleProp::Conditional {
                                condition: test,
                                consequent: Some(Box::new(ExtractStyleProp::StaticArray(yes))),
                                alternate: Some(Box::new(ExtractStyleProp::StaticArray(no))),
                            });
                        }
                        KnownPart::Class(expression) => nested.push(ExtractStyleProp::Expression {
                            expression,
                            styles: Vec::new(),
                        }),
                    }
                }
                nested
            } else {
                let expression = Expression::new_object_expression(
                    property.span,
                    oxc_allocator::Vec::from_array_in(
                        [ObjectPropertyKind::ObjectProperty(property)],
                        &self.ast,
                    ),
                    &self.ast,
                );
                self.part_props(
                    expression.span().start,
                    vec![crate::composition::KnownStyles::Rules(expression)],
                    element,
                )
            };
            composition.apply(&self.ast, props);
        }
        let props = composition.into_props();
        Some(match order {
            Some(Ok(order)) => crate::style_order::apply(order, props, self.ast.allocator()),
            Some(Err(error)) => {
                self.error_disposition.include(error.disposition);
                self.errors.push(error.diagnostic);
                props
            }
            None => props,
        })
    }
}
