//! Taking the expressions of props out of the code that is compiled, to be
//! evaluated once in the arguments of a function the element is built in.

use super::DevupVisitor;
use super::order::{Reach, reach};
use crate::utils::{readable_code, unwrap_syntax_only_mut};
use oxc_allocator::{FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{
    Expression, ObjectPropertyKind, PropertyKey, PropertyKind, Str, TemplateElement,
    TemplateElementValue,
};
use oxc_span::{GetSpan, SPAN};

mod shape;

/// A name, with the expression it is given in the arguments
pub(super) type Captured<'a> = (String, Expression<'a>);

impl<'a> DevupVisitor<'a> {
    /// Put `value` under `name`, returning what it was
    pub(super) fn capture_as(&mut self, name: String, value: &mut Expression<'a>) -> Captured<'a> {
        if let Some(array) = self.style_values.arrays(value).map(<[_]>::to_vec) {
            self.style_values.array_origin(value.span(), array);
        }
        if let Some(finite) = self.style_values.finite(value).cloned() {
            self.style_values.origin(value.span(), finite);
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
        if self.style_values.styles(value).is_some()
            && self
                .style_values
                .finite(value)
                .is_none_or(|finite| finite.results.len() == 1)
        {
            return;
        }
        if reach(&self.bindings, value) != Reach::Constant {
            let mut important = false;
            if let Expression::BinaryExpression(binary) = unwrap_syntax_only_mut(value)
                && binary.operator == oxc_syntax::operator::BinaryOperator::Addition
                && let Expression::StringLiteral(tail) = &mut binary.right
            {
                let text = tail.value.strip_suffix(';').unwrap_or(&tail.value);
                important = text.ends_with(" !important");
                tail.value = Str::from_in(
                    text.strip_suffix(" !important").unwrap_or(text),
                    self.ast.allocator(),
                );
                tail.raw = None;
            }
            if let Expression::TemplateLiteral(template) = unwrap_syntax_only_mut(value)
                && let Some(tail) = template.quasis.last_mut()
            {
                let raw = tail.value.raw.strip_suffix(';').unwrap_or(&tail.value.raw);
                important |= raw.ends_with(" !important");
                tail.value.raw = Str::from_in(
                    raw.strip_suffix(" !important").unwrap_or(raw),
                    self.ast.allocator(),
                );
                tail.value.cooked = tail.value.cooked.as_ref().map(|cooked| {
                    let cooked = cooked.strip_suffix(';').unwrap_or(cooked);
                    Str::from_in(
                        cooked.strip_suffix(" !important").unwrap_or(cooked),
                        self.ast.allocator(),
                    )
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
                                cooked: Some(Str::from_in(text, self.ast.allocator())),
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

fn literal_choice(
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
        Expression::ObjectExpression(_) | Expression::ArrayExpression(_) => (true, false),
        Expression::Identifier(value) if bindings.is_global_undefined(value) => (false, true),
        _ => return None,
    };
    Some(match operator {
        LogicalOperator::And => !truthy,
        LogicalOperator::Or => truthy,
        LogicalOperator::Coalesce => !nullish,
    })
}
