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
                let mut nested = Composition::default();
                for part in parts {
                    let props = match part {
                        KnownPart::Styles(styles) => {
                            self.part_props(property.span.start, styles, element)
                        }
                        KnownPart::Conditional {
                            test,
                            consequent,
                            alternate,
                        } => {
                            let yes = self.part_props(property.span.start, consequent, element);
                            let no = self.part_props(property.span.start, alternate, element);
                            vec![ExtractStyleProp::Conditional {
                                condition: test,
                                consequent: Some(Box::new(ExtractStyleProp::StaticArray(yes))),
                                alternate: Some(Box::new(ExtractStyleProp::StaticArray(no))),
                            }]
                        }
                        KnownPart::Class(expression) => vec![ExtractStyleProp::Expression {
                            expression,
                            styles: Vec::new(),
                        }],
                    };
                    nested.apply(&self.ast, props);
                }
                nested.into_props()
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

    pub(super) fn literal_scope_local<S: super::class_names_parts::LocalSource<'a>>(
        &mut self,
        rules: &Expression<'a>,
        source: &S,
    ) -> Option<Vec<ExtractStyleProp<'a, S::Class>>> {
        super::class_names_rules::LocalRules::new(self, source).literal_scope(rules)
    }
}

#[cfg(test)]
mod literal_w38b_sources {
    use super::*;
    use oxc_ast::ast::Statement;
    use serial_test::serial;

    #[test]
    #[serial]
    fn mixin_scope_when_public_preflight_bypasses_invalid_order_keeps_typed_diagnostic() {
        // Given: the actual scoped producer supplies invalid metadata and an external mixin.
        let source = "tag`background:blue;${base};style-order:255`;";
        let allocator = oxc_allocator::Allocator::default();
        let parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("expression fixture")
        };
        let Expression::TaggedTemplateExpression(tag) = &statement.expression else {
            panic!("tag fixture")
        };
        let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
        let text = crate::css_utils::literal::CssText::from_template(
            &visitor.ast,
            &tag.quasi,
            Some(source),
        );
        let object = text.scoped_object(&visitor.ast, 0..text.text.len(), false);
        // When: the real literal mixin operation sees the production-generated object.
        let actual = visitor
            .literal_scope(&object, None)
            .unwrap_or_else(|| panic!("mixin envelope"));
        // Then: the typed error retains the authored token and forbids evaluator retry.
        assert_eq!(
            visitor.error_disposition,
            crate::ErrorDisposition::Definitive
        );
        assert_eq!(visitor.errors.len(), 1);
        assert_eq!(
            usize::try_from(visitor.errors[0].0)
                .unwrap_or_else(|error| panic!("source offset: {error}")),
            source
                .find("255")
                .unwrap_or_else(|| panic!("authored order"))
        );
        assert!(actual.iter().flat_map(ExtractStyleProp::extract).any(|style| matches!(style, crate::extract_style::extract_style_value::ExtractStyleValue::Static(style) if style.value()=="blue" && style.style_order().is_none())));
    }

    #[test]
    #[serial]
    fn source_marker_when_spread_is_present_still_reports_authored_invalid_order() {
        // Given: source can spell the marker and supply a spread, disproving a generated-only root.
        let source = "({__devupLiteralMixin:base,...{backgroundColor:'blue'},styleOrder:255});";
        let allocator = oxc_allocator::Allocator::default();
        let mut parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
            panic!("expression fixture")
        };
        let value = crate::utils::unwrap_syntax_only_mut(&mut statement.expression);
        let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
        // When: the actual source object reaches literal recognition and its spread guard.
        let actual = visitor.literal_scope(value, None);
        // Then: the guard remains live and metadata stays definitive; lost styling is not canonized.
        assert!(actual.is_some());
        assert_eq!(
            visitor.error_disposition,
            crate::ErrorDisposition::Definitive
        );
        assert_eq!(visitor.errors.len(), 1);
        assert_eq!(
            usize::try_from(visitor.errors[0].0)
                .unwrap_or_else(|error| panic!("source offset: {error}")),
            source
                .find("255")
                .unwrap_or_else(|| panic!("authored order"))
        );
    }
}
