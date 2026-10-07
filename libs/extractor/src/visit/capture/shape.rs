use super::{Captured, literal_choice};
use crate::utils::{readable_code, unwrap_syntax_only, unwrap_syntax_only_mut};
use crate::visit::{
    DevupVisitor, call_order,
    order::{self, Reach, reach},
};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKind, Str};
use oxc_span::SPAN;

impl<'a> DevupVisitor<'a> {
    pub(in crate::visit) fn capture_order_shape(
        &mut self,
        value: &mut Expression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) {
        if crate::style_order::parse_typed(value, self.ast.allocator()).is_err() {
            return;
        }
        let outer = std::mem::replace(
            &mut self.order_metadata_capture,
            crate::style_order::MetadataContext::Order,
        );
        self.capture_shape(value, captured);
        self.order_metadata_capture = outer;
    }
    pub(in crate::visit) fn capture_shape(
        &mut self,
        value: &mut Expression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) {
        if let Expression::TemplateLiteral(template) = unwrap_syntax_only_mut(value)
            && self
                .css_texts
                .contains(&(template.span.start, template.span.end))
        {
            for value in &mut template.expressions {
                self.capture_shape(value, captured);
            }
            return;
        }
        if let Some(array) = self.style_values.arrays(value) {
            let length = array.len();
            let name = self.names.fresh("__devupClasses");
            let (name, original) = self.capture_as(name, value);
            let saved = self.names.fresh("__devupClassArray");
            let entries = (0..length).map(|index| {
                Expression::new_computed_member_expression(
                    SPAN,
                    Expression::new_identifier(
                        SPAN,
                        Str::from_in(saved.as_str(), self.ast.allocator()),
                        &self.ast,
                    ),
                    Expression::new_string_literal(
                        SPAN,
                        Str::from_in(index.to_string().as_str(), self.ast.allocator()),
                        None,
                        &self.ast,
                    ),
                    false,
                    &self.ast,
                )
                .into()
            });
            let snapshot = Expression::new_array_expression(
                SPAN,
                oxc_allocator::Vec::from_iter_in(entries, &self.ast),
                &self.ast,
            );
            captured.push((
                name,
                crate::utils::call_with_values(&self.ast, vec![(saved, original)], snapshot),
            ));
            return;
        }
        if self.style_values.finite(value).is_some() {
            self.capture_leaf(value, captured);
            return;
        }
        if !self.order_metadata_capture.preserves_order()
            && let Expression::LogicalExpression(logical) = unwrap_syntax_only(value)
            && let Some(left) = literal_choice(&self.bindings, &logical.left, logical.operator)
        {
            *value = if left {
                logical
                    .left
                    .clone_in_with_semantic_ids(self.ast.allocator())
            } else {
                logical
                    .right
                    .clone_in_with_semantic_ids(self.ast.allocator())
            };
            self.capture_shape(value, captured);
            return;
        }
        let whole = match unwrap_syntax_only_mut(value) {
            Expression::ClassExpression(_)
            | Expression::JSXElement(_)
            | Expression::JSXFragment(_) => false,
            Expression::ArrayExpression(array) => {
                for element in &mut array.elements {
                    match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            let name = self.names.fresh("__devupValue");
                            let (name, original) = self.capture_as(name, &mut spread.argument);
                            let copy = ArrayExpressionElement::new_spread_element(
                                SPAN, original, &self.ast,
                            );
                            captured.push((
                                name,
                                Expression::new_array_expression(
                                    SPAN,
                                    oxc_allocator::Vec::from_array_in([copy], &self.ast),
                                    &self.ast,
                                ),
                            ));
                        }
                        ArrayExpressionElement::Elision(_) => {}
                        element => {
                            if let Some(element) = element.as_expression_mut() {
                                self.capture_shape(element, captured);
                            }
                        }
                    }
                }
                false
            }
            Expression::ObjectExpression(object) => {
                for property in &mut object.properties {
                    match property {
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            if (reach(&self.bindings, &spread.argument) == Reach::Constant
                                || matches!(
                                    unwrap_syntax_only(&spread.argument),
                                    Expression::ObjectExpression(_)
                                        | Expression::ConditionalExpression(_)
                                        | Expression::LogicalExpression(_)
                                ))
                                && !call_order::unsafe_to_extract(&spread.argument)
                            {
                                self.capture_shape(&mut spread.argument, captured);
                                continue;
                            }
                            let name = self.names.fresh("__devupSpread");
                            captured.push(self.snapshot_as(name, &mut spread.argument));
                        }
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed
                                && property
                                    .key
                                    .static_name()
                                    .or_else(|| {
                                        crate::utils::get_str_by_property_key(&property.key)
                                    })
                                    .is_none()
                                && let Some(key) = property.key.as_expression_mut()
                            {
                                self.capture_leaf(key, captured);
                            }
                            if property.kind == PropertyKind::Init {
                                property.shorthand = false;
                                if crate::utils::get_str_by_property_key(&property.key)
                                    .is_some_and(|key| crate::style_order::reserved(&key))
                                {
                                    self.capture_order_shape(&mut property.value, captured);
                                } else {
                                    self.capture_shape(&mut property.value, captured);
                                }
                            }
                        }
                    }
                }
                false
            }
            Expression::ComputedMemberExpression(member)
                if matches!(
                    unwrap_syntax_only(&member.object),
                    Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
                ) && !call_order::unsafe_to_extract(&member.object) =>
            {
                !self.capture_selection(member, captured)
            }
            Expression::ConditionalExpression(conditional) => {
                let between_constants = reach(&self.bindings, &conditional.consequent)
                    .max(reach(&self.bindings, &conditional.alternate))
                    == Reach::Constant
                    || [&conditional.consequent, &conditional.alternate]
                        .iter()
                        .all(|value| {
                            reach(&self.bindings, value) == Reach::Constant
                                || self
                                    .style_values
                                    .styles(unwrap_syntax_only(value))
                                    .is_some()
                        });
                if between_constants {
                    self.capture_leaf(&mut conditional.test, captured);
                } else {
                    self.capture_conditional(conditional, captured);
                }
                false
            }
            Expression::LogicalExpression(logical) => {
                if order::suspends(&logical.right) {
                    self.errors.push((logical.span.start, crate::utils::build_time_error("style value", &readable_code(&Expression::LogicalExpression(logical.clone_in(self.ast.allocator()))), "a lazy await or yield cannot move into the generated synchronous props wrapper; evaluate the logical value into a variable in the original async or generator scope before rendering")));
                    return;
                }
                let right_is_constant = reach(&self.bindings, &logical.right) == Reach::Constant
                    || self
                        .style_values
                        .styles(unwrap_syntax_only(&logical.right))
                        .is_some();
                if right_is_constant {
                    self.capture_leaf(&mut logical.left, captured);
                } else {
                    self.capture_logical(logical, captured);
                }
                false
            }
            _ => true,
        };
        if whole {
            self.capture_leaf(value, captured);
        }
    }
}
