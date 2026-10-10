use crate::visit::{DevupVisitor, capture::Captured};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKey};
use oxc_span::GetSpan;

#[derive(Clone, Copy)]
enum Placement {
    Preserve,
    Construction,
}

impl<'a> DevupVisitor<'a> {
    pub(in crate::visit) fn capture_literal_styled(
        &mut self,
        value: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
        renders: &mut Vec<Captured<'a>>,
    ) -> bool {
        self.capture_literal_source(value, captures, renders, Placement::Preserve)
    }

    pub(in crate::visit) fn capture_literal_styled_construction(
        &mut self,
        value: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
        renders: &mut Vec<Captured<'a>>,
    ) -> bool {
        self.capture_literal_source(value, captures, renders, Placement::Construction)
    }

    fn capture_literal_source(
        &mut self,
        value: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
        renders: &mut Vec<Captured<'a>>,
        placement: Placement,
    ) -> bool {
        let Expression::ObjectExpression(object) = value else {
            return false;
        };
        let has_callback = object.properties.iter().any(|property| matches!(property, ObjectPropertyKind::ObjectProperty(property) if literal_callback(property) || nested_callback(&property.value)));
        if !has_callback {
            return false;
        }
        let original = object
            .properties
            .clone_in_with_semantic_ids(self.ast.allocator());
        let mut absent_boundary = None;
        for (position, property) in object.properties.iter_mut().enumerate() {
            let property = match property {
                ObjectPropertyKind::SpreadProperty(spread) => {
                    match placement {
                        Placement::Preserve => {}
                        Placement::Construction => {
                            self.capture_source_spread(&mut spread.argument, captures);
                        }
                    }
                    continue;
                }
                ObjectPropertyKind::ObjectProperty(property) => property,
            };
            if literal_callback(property)
                && property
                    .key
                    .static_name()
                    .is_some_and(|key| key == "__devupLiteralMixin")
            {
                let origin = property.value.span();
                if let Some((parts, render)) = self.prepare_callback(&mut property.value, captures)
                {
                    let (class, _) = self.composed_class(origin, parts);
                    let finite = self.style_values.at(origin).cloned();
                    let name = self.names.fresh("__devupLiteralClass");
                    renders.push((
                        name.clone(),
                        crate::utils::call_with_values(&self.ast, vec![render], class),
                    ));
                    property.value = Expression::new_identifier(
                        origin,
                        self.ast.allocator().alloc_str(&name),
                        &self.ast,
                    );
                    if let Some(finite) = finite {
                        self.style_values.origin(origin, finite);
                    }
                }
            } else if literal_callback(property) {
                let Some(wrapped) =
                    crate::visit::literal_callback_order::wrapped(&self.ast, &mut property.value)
                else {
                    self.errors.push((
                        property.value.span().start,
                        crate::style_order::invalid_order(&crate::utils::readable_code(
                            &property.value,
                        )),
                    ));
                    continue;
                };
                if let Some((parts, render)) =
                    self.prepare_order_callback(&mut property.value, captures, wrapped)
                {
                    if parts.is_empty() {
                        absent_boundary = Some(position);
                    } else if let Some(value) =
                        crate::visit::literal_callback_order::value(&self.ast, &parts)
                    {
                        property.value = value;
                    }
                    renders.push(render);
                }
            } else if !match placement {
                Placement::Preserve => {
                    self.capture_literal_styled(&mut property.value, captures, renders)
                }
                Placement::Construction => {
                    self.capture_literal_styled_construction(&mut property.value, captures, renders)
                }
            } {
                if literal_key(property)
                    && property
                        .key
                        .static_name()
                        .is_none_or(|key| key != "__devupLiteralEffect")
                    && matches!(
                        &property.value,
                        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
                    )
                {
                    let name = self.names.fresh("__devupLiteralValue");
                    let captured = self.names.fresh("__devupLiteralCallback");
                    captures.push(self.capture_as(captured, &mut property.value));
                    let invocation = crate::utils::wrap_direct_call(
                        &self.ast,
                        &property.value,
                        &[Expression::new_identifier(
                            oxc_span::SPAN,
                            "__devupStyleProps",
                            &self.ast,
                        )],
                    );
                    renders.push((name.clone(), invocation));
                    property.value = Expression::new_identifier(
                        property.value.span(),
                        self.ast.allocator().alloc_str(&name),
                        &self.ast,
                    );
                } else {
                    self.capture_shape(&mut property.value, captures);
                }
            }
        }
        if let Some(boundary) = absent_boundary {
            for property in original.iter().take(boundary + 1) {
                if order_property(property)
                    && let ObjectPropertyKind::ObjectProperty(metadata) = property
                    && !literal_callback(metadata)
                {
                    let expression = Expression::new_object_expression(
                        metadata.span,
                        oxc_allocator::Vec::from_array_in(
                            [property.clone_in_with_semantic_ids(self.ast.allocator())],
                            &self.ast,
                        ),
                        &self.ast,
                    );
                    self.check_style_orders(&expression, false);
                }
            }
            let mut position = 0;
            object.properties.retain(|property| {
                let keep = position > boundary || !order_property(property);
                position += 1;
                keep
            });
        }
        true
    }
}

fn order_property(property: &ObjectPropertyKind<'_>) -> bool {
    match property {
        ObjectPropertyKind::ObjectProperty(property) => property
            .key
            .static_name()
            .or_else(|| crate::utils::get_str_by_property_key(&property.key))
            .is_some_and(|name| crate::style_order::reserved(&name)),
        ObjectPropertyKind::SpreadProperty(_) => false,
    }
}

fn literal_callback(property: &oxc_ast::ast::ObjectProperty<'_>) -> bool {
    literal_key(property)
        && property
            .key
            .static_name()
            .is_some_and(|key| crate::style_order::reserved(&key) || key == "__devupLiteralMixin")
        && matches!(
            &property.value,
            Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
        )
}

fn literal_key(property: &oxc_ast::ast::ObjectProperty<'_>) -> bool {
    matches!(&property.key, PropertyKey::StringLiteral(key) if key.raw.is_none() && key.span == property.span)
}

fn nested_callback(expression: &Expression<'_>) -> bool {
    match expression {
        Expression::ObjectExpression(object) => object.properties.iter().any(|property| matches!(property, ObjectPropertyKind::ObjectProperty(property) if literal_callback(property) || nested_callback(&property.value))),
        _ => false,
    }
}
