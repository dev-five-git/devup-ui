use super::{DevupVisitor, capture::Captured};
use oxc_allocator::{CloneIn, GetAllocator};
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
        let original = object
            .properties
            .clone_in_with_semantic_ids(self.ast.allocator());
        let mut absent_boundary = None;
        for (position, property) in object.properties.iter_mut().enumerate() {
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
                let Some(wrapped) =
                    super::literal_callback_order::wrapped(&self.ast, &mut property.value)
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
                        super::literal_callback_order::value(&self.ast, &parts)
                    {
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

#[cfg(test)]
mod literal_w38f_wrapper;

#[cfg(test)]
mod literal_w38b_sources {
    use super::*;
    use oxc_allocator::CloneIn;
    use oxc_ast::ast::Statement;
    use oxc_ast_visit::VisitMut;
    use serial_test::serial;

    #[test]
    #[serial]
    fn callback_when_nested_text_is_lowered_preserves_real_root_spread() {
        // Given: an actual mixed object, with a nested template lowered by production.
        let source = "({...rest,_hover:`style-order:${p=>p.active?2:3};color:red`});";
        let allocator = oxc_allocator::Allocator::default();
        let mut parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
            panic!("expression fixture")
        };
        let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
        let value = crate::utils::unwrap_syntax_only_mut(&mut statement.expression);
        assert!(crate::css_utils::literal_tree::lower(
            &visitor.ast,
            value,
            crate::css_utils::literal_tree::Scope {
                source: Some(source),
                global: false
            }
        ));
        let mut captures = Vec::new();
        let mut renders = Vec::new();
        // When: the real literal callback preparer receives the produced mixed container.
        let actual = visitor.capture_literal_styled(value, &mut captures, &mut renders);
        // Then: the source spread survives; a nested callback is prepared once.
        assert!(actual);
        let Expression::ObjectExpression(object) = value else {
            panic!("mixed object")
        };
        assert!(matches!(
            object.properties[0],
            ObjectPropertyKind::SpreadProperty(_)
        ));
        assert_eq!(renders.len(), 1);
        assert_eq!(visitor.errors.len(), 0);
    }

    #[test]
    #[serial]
    fn order_wrapper_when_callback_produces_classes_observes_finite_provenance() {
        // Given: public visiting first processes a real class-producing callback.
        let source = "import {css} from '@devup-ui/react';p=>css({color:p.active?'red':'blue'});";
        let allocator = oxc_allocator::Allocator::default();
        let mut parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
        visitor.visit_program(&mut parsed.program);
        let Statement::ExpressionStatement(statement) = parsed
            .program
            .body
            .last_mut()
            .unwrap_or_else(|| panic!("retained expression"))
        else {
            panic!("callback fixture")
        };
        let mut order_callback = statement
            .expression
            .clone_in_with_semantic_ids(visitor.ast.allocator());
        assert!(super::super::literal_callback_order::wrap(
            &visitor.ast,
            &mut statement.expression
        ));
        let mut captures = Vec::new();
        // When: the actual wrapped callback passes through the production preparer.
        let (parts, _) = visitor
            .prepare_callback(&mut statement.expression, &mut captures)
            .unwrap_or_else(|| panic!("prepared wrapper"));
        // Then: finite provenance is observed, not a manufactured Class part.
        assert!(
            matches!(parts.as_slice(), [crate::composition::KnownPart::Styles(styles)] if matches!(styles.as_slice(), [crate::composition::KnownStyles::Finite(_, _)]))
        );
        let wrapped =
            super::super::literal_callback_order::wrapped(&visitor.ast, &mut order_callback)
                .unwrap_or_else(|| panic!("source wrapper"));
        let (parts, _) = visitor
            .prepare_order_callback(&mut order_callback, &mut Vec::new(), wrapped)
            .unwrap_or_else(|| panic!("typed source wrapper"));
        assert!(super::super::literal_callback_order::value(&visitor.ast, &parts).is_none());
    }
}
