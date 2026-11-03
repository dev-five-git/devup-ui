//! Taking the expressions of props out of the code that is compiled, to be
//! evaluated once in the arguments of a function the element is built in.

use super::DevupVisitor;
use super::order::{Reach, reach};
use crate::utils::{readable_code, unwrap_syntax_only_mut};
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{
    ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str,
    TemplateElement, TemplateElementValue,
};
use oxc_span::{GetSpan, SPAN};

/// A name, with the expression it is given in the arguments
pub(super) type Captured<'a> = (String, Expression<'a>);

impl<'a> DevupVisitor<'a> {
    /// Put `value` under `name`, returning what it was
    pub(super) fn capture_as(&mut self, name: String, value: &mut Expression<'a>) -> Captured<'a> {
        let styles = self.class_styles(value);
        if !styles.is_empty() {
            self.captured_class_styles.insert(name.clone(), styles);
        }
        self.names.stands_for(&name, readable_code(value));
        let read = Expression::new_identifier(
            value.span(),
            Str::from_in(name.as_str(), self.ast.allocator()),
            &self.ast,
        );
        (name, std::mem::replace(value, read))
    }

    /// Put `value` under a name of its own when evaluating it can be told
    /// from evaluating it anywhere else
    fn capture_leaf(&mut self, value: &mut Expression<'a>, captured: &mut Vec<Captured<'a>>) {
        if reach(&self.bindings, value) != Reach::Constant {
            let mut important = false;
            if let Expression::BinaryExpression(binary) = unwrap_syntax_only_mut(value)
                && binary.operator == oxc_syntax::operator::BinaryOperator::Addition
                && let Expression::StringLiteral(tail) = &mut binary.right
                && let Some(text) = tail.value.as_str()
            {
                let text = text.strip_suffix(';').unwrap_or(text);
                important = text.ends_with(" !important");
                tail.value = Str::from_in(
                    text.strip_suffix(" !important").unwrap_or(text),
                    self.ast.allocator(),
                )
                .into();
                tail.raw = None;
            }
            if let Expression::TemplateLiteral(template) = unwrap_syntax_only_mut(value)
                && let Some(tail) = template.quasis.last_mut()
            {
                let raw = tail.value.raw.as_str();
                let raw = raw.strip_suffix(';').unwrap_or(raw);
                important |= raw.ends_with(" !important");
                tail.value.raw = Str::from_in(
                    raw.strip_suffix(" !important").unwrap_or(raw),
                    self.ast.allocator(),
                );
                tail.value.cooked = tail.value.cooked.as_ref().map(|cooked| {
                    let Some(text) = cooked.as_str() else {
                        return *cooked;
                    };
                    let cooked = text.strip_suffix(';').unwrap_or(text);
                    Str::from_in(
                        cooked.strip_suffix(" !important").unwrap_or(cooked),
                        self.ast.allocator(),
                    )
                    .into()
                });
            }
            let name = self.names.fresh("__devupValue");
            captured.push(self.capture_as(name, value));
            if important {
                let read = value.take_in(&self.ast);
                let quasis = ["", " !important"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, text)| {
                        TemplateElement::new(
                            SPAN,
                            TemplateElementValue {
                                raw: Str::from_in(text, self.ast.allocator()),
                                cooked: Some(Str::from_in(text, self.ast.allocator()).into()),
                            },
                            index == 1,
                            &self.ast,
                        )
                    });
                *value = Expression::new_template_literal(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(quasis, &self.ast),
                    oxc_allocator::Vec::from_array_in([read], &self.ast),
                    &self.ast,
                );
            }
        }
    }

    /// Put what `value` evaluates, in the order it does, under names, while it
    /// keeps the shape the build reads styles from: the elements of an array,
    /// the values of an object and the test of a condition between constants
    pub(super) fn capture_shape(
        &mut self,
        value: &mut Expression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) {
        if self.css_capture.is_some() && self.capture_css_boundary(value, captured) {
            return;
        }
        if let Expression::LogicalExpression(logical) = crate::utils::unwrap_syntax_only(value)
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
                        ArrayExpressionElement::SpreadElement(spread)
                            if matches!(
                                crate::utils::unwrap_syntax_only(&spread.argument),
                                Expression::ArrayExpression(_)
                            ) =>
                        {
                            self.capture_shape(&mut spread.argument, captured);
                        }
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
                            let name = self.names.fresh("__devupSpread");
                            captured.push(self.snapshot_as(name, &mut spread.argument));
                        }
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed
                                && self.css_capture.is_none()
                                && let Some(key) = property.key.as_expression_mut()
                            {
                                self.capture_leaf(key, captured);
                            }
                            if property.kind == PropertyKind::Init {
                                property.shorthand = false;
                                let css = if matches!(
                                    crate::utils::unwrap_syntax_only(&property.value),
                                    Expression::ObjectExpression(_)
                                ) {
                                    None
                                } else {
                                    self.css_capture.take()
                                };
                                self.capture_shape(&mut property.value, captured);
                                if css.is_some() {
                                    self.css_capture = css;
                                }
                            }
                        }
                    }
                }
                false
            }
            Expression::ComputedMemberExpression(member)
                if matches!(
                    crate::utils::unwrap_syntax_only(&member.object),
                    Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
                ) && !super::call_order::unsafe_to_extract(&member.object) =>
            {
                !self.capture_selection(member, captured)
            }
            Expression::ConditionalExpression(conditional) => {
                let between_constants = reach(&self.bindings, &conditional.consequent)
                    .max(reach(&self.bindings, &conditional.alternate))
                    == Reach::Constant;
                if between_constants {
                    self.capture_leaf(&mut conditional.test, captured);
                } else {
                    self.capture_conditional(conditional, captured);
                }
                false
            }
            Expression::LogicalExpression(logical) => {
                if super::order::suspends(&logical.right) {
                    self.errors.push((logical.span.start,crate::utils::build_time_error("style value",&readable_code(&Expression::LogicalExpression(logical.clone_in(self.ast.allocator()))),"a lazy await or yield cannot move into the generated synchronous props wrapper; evaluate the logical value into a variable in the original async or generator scope before rendering")));
                    return;
                }
                if self.css_capture.is_some() {
                    self.capture_css_logical(value, captured);
                    return;
                }
                let right_is_constant = reach(&self.bindings, &logical.right) == Reach::Constant;
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

    /// Preserve CSS admission syntax while capturing only declaration values.
    fn capture_css_boundary(
        &mut self,
        value: &mut Expression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) -> bool {
        if self.reads_local_styles(value) {
            return true;
        }
        let approved = match crate::utils::unwrap_syntax_only(value) {
            Expression::CallExpression(call) => self
                .util_type(&call.callee)
                .is_some_and(|util| util.is_css()),
            Expression::TaggedTemplateExpression(tag) => {
                self.util_type(&tag.tag).is_some_and(|util| util.is_css())
            }
            _ => false,
        };
        match unwrap_syntax_only_mut(value) {
            Expression::CallExpression(call) => {
                if approved {
                    for argument in &mut call.arguments {
                        if let Some(value) = argument.as_expression_mut() {
                            self.capture_shape(value, captured);
                        }
                    }
                }
            }
            Expression::TaggedTemplateExpression(tag) if approved => {
                self.capture_css_template(&mut tag.quasi, captured, true);
            }
            Expression::TemplateLiteral(template) => {
                self.capture_css_template(template, captured, false);
            }
            Expression::TaggedTemplateExpression(_) => {}
            _ => return false,
        }
        true
    }

    fn capture_css_template(
        &mut self,
        template: &mut oxc_ast::ast::TemplateLiteral<'a>,
        captured: &mut Vec<Captured<'a>>,
        tagged: bool,
    ) {
        if crate::css_prop::template_parts(&self.ast, template, tagged).is_err() {
            return;
        }
        for (index, (place, _)) in crate::css_utils::template_interpolation_places(template)
            .into_iter()
            .enumerate()
        {
            let value = unwrap_syntax_only_mut(&mut template.expressions[index]);
            if tagged && matches!(place, crate::css_utils::Place::Statement) {
                self.capture_shape(value, captured);
            } else if reach(&self.bindings, value) != Reach::Constant
                || (!tagged
                    && matches!(
                        value,
                        Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
                    ))
            {
                if tagged {
                    self.capture_leaf(value, captured);
                } else {
                    let expression = value.take_in(&self.ast);
                    let quasis = [false, true].map(|tail| {
                        TemplateElement::new(
                            SPAN,
                            crate::utils::template_element_value("", &self.ast),
                            tail,
                            &self.ast,
                        )
                    });
                    *value = Expression::new_template_literal(
                        SPAN,
                        oxc_allocator::Vec::from_array_in(quasis, &self.ast),
                        oxc_allocator::Vec::from_array_in([expression], &self.ast),
                        &self.ast,
                    );
                    let name = self.names.fresh("__devupValue");
                    captured.push(self.capture_as(name, value));
                }
            }
        }
    }

    /// `value` copied once into an object without a prototype, which keeps
    /// what a spread of it assigns (even `undefined`) as data
    pub(super) fn snapshot(&self, value: Expression<'a>) -> Expression<'a> {
        let prototype = ObjectPropertyKind::new_object_property(
            SPAN,
            PropertyKind::Init,
            PropertyKey::new_static_identifier(SPAN, "__proto__", &self.ast),
            Expression::new_null_literal(SPAN, &self.ast),
            false,
            false,
            false,
            &self.ast,
        );
        let copy = ObjectPropertyKind::new_spread_property(SPAN, value, &self.ast);
        Expression::new_object_expression(
            SPAN,
            oxc_allocator::Vec::from_array_in([prototype, copy], &self.ast),
            &self.ast,
        )
    }

    /// `value` with its place taken by a name, for [`Self::snapshot`] to
    /// be evaluated in its arguments
    pub(super) fn snapshot_as(&mut self, name: String, value: &mut Expression<'a>) -> Captured<'a> {
        let (name, original) = self.capture_as(name, value);
        if let Expression::Identifier(read) = value {
            read.span = SPAN;
        }
        (name, self.snapshot(original))
    }
}

pub(super) fn literal_choice(
    bindings: &crate::scope::Bindings,
    value: &Expression<'_>,
    operator: oxc_syntax::operator::LogicalOperator,
) -> Option<bool> {
    use oxc_syntax::operator::LogicalOperator;
    let (truthy, nullish) = match crate::utils::unwrap_syntax_only(value) {
        Expression::NullLiteral(_) => (false, true),
        Expression::BooleanLiteral(value) => (value.value, false),
        Expression::NumericLiteral(value) => (value.value != 0.0 && !value.value.is_nan(), false),
        Expression::StringLiteral(value) => (!value.value.is_empty(), false),
        Expression::TemplateLiteral(value) if value.expressions.is_empty() => (
            !value.quasis.first()?.value.cooked?.as_str()?.is_empty(),
            false,
        ),
        Expression::Identifier(value) if bindings.is_global_undefined(value) => (false, true),
        _ => return None,
    };
    Some(match operator {
        LogicalOperator::And => !truthy,
        LogicalOperator::Or => truthy,
        LogicalOperator::Coalesce => !nullish,
    })
}
