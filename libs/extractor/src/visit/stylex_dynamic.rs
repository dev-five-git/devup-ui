use super::{
    Argument, BinaryOperator, CallExpression, CloneIn, DevupVisitor, Expression, ExtractStyleProp,
    ExtractStyleValue, FromIn, GetAllocator, NumberBase, ObjectPropertyKind, PropertyKey,
    PropertyKind, SPAN, Str, StringLiteral, StylexDynamicInfo, StylexNamespaceValue, UnaryOperator,
    build_time_error, call_with_values, gen_class_names, get_string_by_literal_expression,
    readable_code, unwrap_syntax_only,
};
use crate::extractor::extract_style_from_stylex::raw_static_style;
use crate::stylex::Scalar;
use crate::utils::js_number_string;

mod checks;

impl<'a> DevupVisitor<'a> {
    pub(super) fn stylex_dynamic_info(
        &self,
        callee: &Expression<'a>,
    ) -> Option<&StylexDynamicInfo> {
        let (object, name) = match unwrap_syntax_only(callee) {
            Expression::StaticMemberExpression(member) => {
                (&member.object, member.property.name.to_string())
            }
            Expression::ComputedMemberExpression(member) => (
                &member.object,
                get_string_by_literal_expression(&member.expression)?.into_owned(),
            ),
            _ => return None,
        };
        match self.stylex_namespace(object)?.get(&name)? {
            StylexNamespaceValue::Dynamic(info) => Some(info),
            StylexNamespaceValue::Static(_) => None,
        }
    }

    fn scalar_expression(&self, value: &Scalar) -> Expression<'a> {
        match value {
            Scalar::Undefined => Expression::new_unary_expression(
                SPAN,
                UnaryOperator::Void,
                Expression::new_numeric_literal(SPAN, 0.0, None, NumberBase::Decimal, &self.ast),
                &self.ast,
            ),
            Scalar::Null => Expression::new_null_literal(SPAN, &self.ast),
            Scalar::Text(value) => Expression::new_string_literal(
                SPAN,
                Str::from_in(value, self.ast.allocator()),
                None,
                &self.ast,
            ),
            Scalar::Number(value) => {
                Expression::new_numeric_literal(SPAN, *value, None, NumberBase::Decimal, &self.ast)
            }
            Scalar::Boolean(value) => Expression::new_boolean_literal(SPAN, *value, &self.ast),
        }
    }

    fn stylex_scalar(&self, value: &Expression<'a>) -> Option<Scalar> {
        match unwrap_syntax_only(value) {
            Expression::Identifier(ident)
                if ident.name == "undefined" && self.bindings.symbol(ident).is_none() =>
            {
                Some(Scalar::Undefined)
            }
            value => Scalar::literal(value),
        }
    }

    fn stylex_assignment(&self, name: &str, value: Expression<'a>) -> ObjectPropertyKind<'a> {
        ObjectPropertyKind::new_object_property(
            SPAN,
            PropertyKind::Init,
            PropertyKey::StringLiteral(StringLiteral::boxed(
                SPAN,
                Str::from_in(name, self.ast.allocator()),
                None,
                &self.ast,
            )),
            value,
            false,
            false,
            false,
            &self.ast,
        )
    }

    pub(super) fn resolve_stylex_dynamic_call(
        &mut self,
        call: &CallExpression<'a>,
        attrs: bool,
    ) -> Option<(Expression<'a>, Vec<ObjectPropertyKind<'a>>)> {
        let info = self.stylex_dynamic_info(&call.callee)?.clone();
        if call.optional
            || call
                .arguments
                .iter()
                .any(|arg| arg.as_expression().is_none())
        {
            self.errors.push((call.span.start, build_time_error("stylex.props", &readable_code(&call.callee), "optional/spread dynamic calls cannot be compiled exactly; pass explicit scalar arguments to a direct namespace call")));
            return Some((self.scalar_expression(&Scalar::Text(String::new())), vec![]));
        }
        let known: Option<Vec<_>> = call
            .arguments
            .iter()
            .map(|arg| {
                arg.as_expression()
                    .and_then(|value| self.stylex_scalar(value))
            })
            .collect();
        if let Some(known) = known {
            let mut styles: Vec<_> = info
                .styles
                .iter()
                .filter(|style| matches!(style, ExtractStyleValue::Static(_)))
                .cloned()
                .map(ExtractStyleProp::Static)
                .collect();
            for (index, _, unit, property) in &info.namespace.css_vars {
                let value = known.get(*index).cloned().unwrap_or(Scalar::Undefined);
                let value = match value {
                    Scalar::Undefined => info.namespace.defaults[*index]
                        .clone()
                        .unwrap_or(Scalar::Undefined),
                    value => value,
                };
                let text = match value {
                    Scalar::Number(number) => Some(format!("{}{}", js_number_string(number), unit)),
                    Scalar::Text(text) => Some(text),
                    Scalar::Boolean(value) => Some(value.to_string()),
                    Scalar::Null | Scalar::Undefined => None,
                };
                if let Some(value) = text {
                    styles.push(raw_static_style(property.clone(), &value, None));
                }
            }
            let class =
                gen_class_names(&self.ast, &mut styles, None, self.split_filename.as_deref())
                    .unwrap_or_else(|| self.scalar_expression(&Scalar::Text(String::new())));
            self.styles
                .extend(styles.into_iter().flat_map(ExtractStyleProp::into_extract));
            return Some((class, vec![]));
        }
        if info.namespace.defaults.iter().all(Option::is_none)
            && info.namespace.defaults.len() == call.arguments.len()
            && info.namespace.css_vars.len() == call.arguments.len()
            && info
                .namespace
                .css_vars
                .iter()
                .enumerate()
                .all(|(position, (index, _, _, _))| position == *index)
            && call.arguments.iter().all(|argument| {
                matches!(
                    argument.as_expression().map(unwrap_syntax_only),
                    Some(Expression::Identifier(_))
                )
            })
        {
            let props = info
                .namespace
                .css_vars
                .iter()
                .zip(&call.arguments)
                .filter_map(|((_, name, unit, _), argument)| {
                    argument.as_expression().map(|value| {
                        self.stylex_assignment(
                            name,
                            self.with_number_unit(
                                value.clone_in_with_semantic_ids(self.ast.allocator()),
                                unit,
                            ),
                        )
                    })
                })
                .collect();
            return Some((
                self.scalar_expression(&Scalar::Text(info.class_name)),
                props,
            ));
        }
        let count = call.arguments.len().max(info.namespace.defaults.len());
        let values: Vec<_> = (0..count)
            .map(|index| {
                let value = call
                    .arguments
                    .get(index)
                    .and_then(Argument::as_expression)
                    .map_or_else(
                        || self.scalar_expression(&Scalar::Undefined),
                        |value| value.clone_in_with_semantic_ids(self.ast.allocator()),
                    );
                (format!("sx{index}"), value)
            })
            .collect();
        let mut props = oxc_allocator::Vec::new_in(&self.ast);
        for (index, name, unit, _) in &info.namespace.css_vars {
            let ident = || {
                Expression::new_identifier(
                    SPAN,
                    oxc_ast::ast::Ident::from_in(
                        format!("sx{index}").as_str(),
                        self.ast.allocator(),
                    ),
                    &self.ast,
                )
            };
            let value = match &info.namespace.defaults[*index] {
                Some(default) => Expression::new_conditional_expression(
                    SPAN,
                    Expression::new_binary_expression(
                        SPAN,
                        ident(),
                        BinaryOperator::StrictEquality,
                        self.scalar_expression(&Scalar::Undefined),
                        &self.ast,
                    ),
                    self.scalar_expression(default),
                    ident(),
                    &self.ast,
                ),
                None => ident(),
            };
            props.push(self.stylex_assignment(name, self.with_number_unit(value, unit)));
        }
        let body = if attrs {
            self.style_text(props.into_iter().collect())
        } else {
            Expression::new_object_expression(SPAN, props, &self.ast)
        };
        let captured = call_with_values(&self.ast, values, body);
        let spread = ObjectPropertyKind::SpreadProperty(oxc_ast::ast::SpreadElement::boxed(
            SPAN, captured, &self.ast,
        ));
        Some((
            self.scalar_expression(&Scalar::Text(info.class_name)),
            vec![spread],
        ))
    }

    pub(super) fn has_runtime_stylex_call(&self, arguments: &[Argument<'a>]) -> bool {
        arguments.iter().filter_map(Argument::as_expression).any(|value| {
            matches!(unwrap_syntax_only(value), Expression::CallExpression(call)
                if self.stylex_dynamic_info(&call.callee).is_some()
                && call.arguments.iter().any(|arg| arg.as_expression().and_then(|value| self.stylex_scalar(value)).is_none()))
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;
