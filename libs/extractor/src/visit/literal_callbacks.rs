use super::{DevupVisitor, capture::Captured};
use oxc_allocator::GetAllocator;
use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKey};
use oxc_span::GetSpan;

impl<'a> DevupVisitor<'a> {
    pub(super) fn capture_literal_styled(
        &mut self,
        value: &mut Expression<'a>,
        captures: &mut Vec<Captured<'a>>,
        renders: &mut Vec<Captured<'a>>,
    ) -> bool {
        let Expression::ObjectExpression(object) = value else {
            return false;
        };
        let has_callback = object.properties.iter().any(|property| matches!(property, ObjectPropertyKind::ObjectProperty(property) if literal_callback(property) || nested_callback(&property.value)));
        if !has_callback {
            return false;
        }
        for property in &mut object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
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
                if !super::literal_callback_order::wrap(&self.ast, &mut property.value) {
                    self.errors.push((
                        property.value.span().start,
                        crate::style_order::invalid_order(&crate::utils::readable_code(
                            &property.value,
                        )),
                    ));
                    continue;
                }
                if let Some((parts, render)) = self.prepare_callback(&mut property.value, captures)
                {
                    if let Some(value) = super::literal_callback_order::value(&self.ast, &parts) {
                        property.value = value;
                    }
                    renders.push(render);
                }
            } else if !self.capture_literal_styled(&mut property.value, captures, renders) {
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
        true
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
