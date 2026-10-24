use super::{Origin, Preparation};
use crate::utils::{get_string_by_literal_expression, unwrap_syntax_only};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{ArrayExpressionElement, Expression, ObjectPropertyKind, PropertyKind};
use oxc_ast_visit::Visit;
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

#[path = "styled_record_selection.rs"]
mod selection;

fn lookup_object<'a, 'b>(source: &'b Expression<'a>) -> &'b Expression<'a> {
    match source {
        Expression::TSAsExpression(value) => lookup_object(&value.expression),
        Expression::ParenthesizedExpression(value) => lookup_object(&value.expression),
        _ => source,
    }
}

/// Pure template folding belongs to the shared numeric-template correction, not finite lookups.
fn pure_lookup_literal(source: &Expression<'_>) -> bool {
    match unwrap_syntax_only(source) {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::UnaryExpression(unary) => {
            matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
            ) && pure_lookup_literal(&unary.argument)
        }
        _ => false,
    }
}

/// Visible templates anywhere in an index require capturing the whole member value.
fn template_free_lookup_index(source: &Expression<'_>) -> bool {
    struct Templates(bool);

    impl<'a> Visit<'a> for Templates {
        fn visit_template_literal(&mut self, _: &oxc_ast::ast::TemplateLiteral<'a>) {
            self.0 = true;
        }
    }

    let mut templates = Templates(false);
    templates.visit_expression(source);
    !templates.0
}

/// Only literal data can be replaced by the extractor's finite class lookup.
fn static_lookup_value(source: &Expression<'_>) -> bool {
    let source = unwrap_syntax_only(source);
    if pure_lookup_literal(source) && get_string_by_literal_expression(source).is_some() {
        return true;
    }
    match source {
        Expression::ObjectExpression(object) => object.properties.iter().all(|property| {
            matches!(property, ObjectPropertyKind::ObjectProperty(property)
                if !property.method && property.kind == PropertyKind::Init
                    && selection::property_key(&property.key).is_some()
                    && (property.computed
                        || selection::property_key(&property.key).as_deref() != Some("__proto__"))
                    && static_lookup_value(&property.value))
        }),
        Expression::ArrayExpression(array) => array.elements.iter().all(|element| match element {
            ArrayExpressionElement::Elision(_) => true,
            _ => element.as_expression().is_some_and(static_lookup_value),
        }),
        Expression::NullLiteral(_) => true,
        _ => false,
    }
}

impl<'a> Preparation<'_, '_, 'a> {
    pub(super) fn value(
        &mut self,
        source: &Expression<'a>,
        guard: &Expression<'a>,
        key: &str,
    ) -> Option<Expression<'a>> {
        let source = unwrap_syntax_only(source);
        let builder = self.reads.builder;
        if get_string_by_literal_expression(source).is_some() {
            return Some(source.clone_in(builder.allocator()));
        }
        match source {
            Expression::ComputedMemberExpression(member)
                if matches!(
                    lookup_object(&member.object),
                    Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
                ) && static_lookup_value(&member.object)
                    && template_free_lookup_index(&member.expression) =>
            {
                let mut member = member.clone_in(builder.allocator());
                member.object = lookup_object(&member.object).clone_in(builder.allocator());
                if pure_lookup_literal(&member.expression) {
                    if !selection::direct_lookup(&member.object, &member.expression) {
                        return Some(self.reads.capture(source, guard));
                    }
                } else {
                    self.select_lookup(&mut member, guard)?;
                }
                Some(Expression::ComputedMemberExpression(member))
            }
            Expression::ObjectExpression(_) => {
                let entries = self.record(source, guard)?;
                Some(self.materialize(source.span(), entries))
            }
            Expression::ConditionalExpression(choice) => {
                let test = self.reads.capture(&choice.test, guard);
                let yes_guard = self.reads.choose(guard, &test, &self.reads.boolean(false));
                let no_guard = self.reads.choose(
                    guard,
                    &self.reads.choose(
                        &test,
                        &self.reads.boolean(false),
                        &self.reads.boolean(true),
                    ),
                    &self.reads.boolean(false),
                );
                let yes = self.value(&choice.consequent, &yes_guard, key)?;
                let no = self.value(&choice.alternate, &no_guard, key)?;
                Some(self.reads.choose(&test, &yes, &no))
            }
            Expression::LogicalExpression(logical) => {
                let left = self.reads.capture(&logical.left, guard);
                let takes_right = match logical.operator {
                    LogicalOperator::And => left.clone_in(builder.allocator()),
                    LogicalOperator::Or => self.reads.choose(
                        &left,
                        &self.reads.boolean(false),
                        &self.reads.boolean(true),
                    ),
                    LogicalOperator::Coalesce => Expression::new_binary_expression(
                        source.span(),
                        left.clone_in(builder.allocator()),
                        BinaryOperator::Equality,
                        Expression::new_null_literal(source.span(), builder),
                        builder,
                    ),
                };
                let right_guard =
                    self.reads
                        .choose(guard, &takes_right, &self.reads.boolean(false));
                let right = self.value(&logical.right, &right_guard, key)?;
                Some(Expression::new_logical_expression(
                    source.span(),
                    left,
                    logical.operator,
                    right,
                    builder,
                ))
            }
            Expression::ArrayExpression(array) => {
                let mut array = array.clone_in(builder.allocator());
                for element in &mut array.elements {
                    if let Some(value) = element.as_expression_mut() {
                        *value = self.value(value, guard, key)?;
                    } else if matches!(element, ArrayExpressionElement::SpreadElement(_)) {
                        return None;
                    }
                }
                Some(Expression::ArrayExpression(array))
            }
            Expression::FunctionExpression(_) | Expression::ArrowFunctionExpression(_)
                if matches!(self.origin, Origin::OrdinaryCallbacks)
                    && crate::css_utils::theme_var_reference(source).is_none() =>
            {
                let call = crate::utils::wrap_direct_call(
                    builder,
                    source,
                    &[Expression::new_identifier(oxc_span::SPAN, "rest", builder)],
                );
                let slot = self.reads.capture(&call, guard);
                Some(crate::extractor::extract_style_from_styled::scalar::captured(
                    builder,
                    crate::extractor::extract_style_from_styled::scalar::Origin::OrdinaryProperty(key),
                    slot,
                ))
            }
            Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_) => Some(source.clone_in(builder.allocator())),
            Expression::Identifier(identifier)
                if matches!(identifier.name.as_str(), "undefined" | "NaN" | "Infinity") =>
            {
                Some(source.clone_in(builder.allocator()))
            }
            _ => Some(self.reads.capture(source, guard)),
        }
    }
}
