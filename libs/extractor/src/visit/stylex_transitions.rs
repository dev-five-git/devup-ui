use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::ast::{Argument, Expression, Str, UnaryOperator};
use oxc_span::SPAN;
use oxc_syntax::symbol::SymbolId;

use super::DevupVisitor;
use crate::ExtractStyleValue;
use crate::extract_style::extract_css::ExtractCss;
use crate::extractor::extract_style_from_stylex::declaration_error;
use crate::stylex::transitions::{DeclarationValue, TransitionApi, TransitionRules};
use crate::stylex::{StylexFunction, stylex_value};

mod dependencies;

impl<'a> DevupVisitor<'a> {
    pub(super) fn compile_stylex_transition(&mut self, expression: &mut Expression<'a>) -> bool {
        let Expression::CallExpression(call) = expression else {
            return false;
        };
        let api = match self.bindings.stylex_function(&call.callee) {
            Some(StylexFunction::PositionTry) => TransitionApi::Position,
            Some(StylexFunction::ViewTransitionClass) => TransitionApi::View,
            _ => return false,
        };
        let [Argument::ObjectExpression(object)] = call.arguments.as_slice() else {
            return false;
        };
        let visible = self.bindings.visible(object);
        let (names, vars) = self.stylex_names(&visible);
        let reader = |property: &str, value: &Expression<'_>| {
            if self.omitted_transition_value(value) {
                return Ok(DeclarationValue::Omitted);
            }
            if matches!(
                crate::utils::unwrap_syntax_only(value),
                Expression::BooleanLiteral(_)
            ) {
                return Err(declaration_error(api.name(), value, &|callee| {
                    self.bindings.stylex_function(callee)
                }));
            }
            let resolved = match value {
                Expression::Identifier(identifier) => names.get(identifier.name.as_str()).cloned(),
                Expression::StaticMemberExpression(member) => match &member.object {
                    Expression::Identifier(identifier) => vars
                        .get(&format!("{}.{}", identifier.name, member.property.name))
                        .cloned(),
                    _ => None,
                },
                _ => None,
            };
            resolved
                .or_else(|| stylex_value(property, value).map(std::borrow::Cow::into_owned))
                .map(DeclarationValue::Scalar)
                .ok_or_else(|| {
                    declaration_error(api.name(), value, &|callee| {
                        self.bindings.stylex_function(callee)
                    })
                })
        };
        let rules = match TransitionRules::parse(api, object, &reader) {
            Ok(rules) => rules,
            Err(errors) => {
                self.errors.extend(errors);
                return true;
            }
        };
        let (name, css) = rules.compile(&self.filename);
        if !css.is_empty() {
            self.styles.insert(ExtractStyleValue::Css(ExtractCss {
                css,
                file: self.filename.clone(),
            }));
        }
        self.stylex_pending_transition = Some((call.span.start, name.clone()));
        *expression = Expression::new_string_literal(
            SPAN,
            Str::from_in(&name, self.ast.allocator()),
            None,
            &self.ast,
        );
        true
    }

    fn omitted_transition_value(&self, value: &Expression<'_>) -> bool {
        match crate::utils::unwrap_syntax_only(value) {
            Expression::NullLiteral(_) => true,
            Expression::BooleanLiteral(boolean) => !boolean.value,
            Expression::Identifier(identifier) => {
                identifier.name == "undefined" && self.bindings.symbol(identifier).is_none()
            }
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Void => {
                matches!(
                    &unary.argument,
                    Expression::NumericLiteral(_)
                        | Expression::StringLiteral(_)
                        | Expression::NullLiteral(_)
                        | Expression::BooleanLiteral(_)
                )
            }
            _ => false,
        }
    }

    pub(super) fn bind_stylex_transition(&mut self, symbol: Option<SymbolId>, start: Option<u32>) {
        if let Some((at, name)) = self.stylex_pending_transition.take()
            && start == Some(at)
            && let Some(symbol) = symbol
            && self.bindings.unchanged(symbol)
        {
            self.stylex_transition_names.insert(symbol, name);
        }
    }
}

#[cfg(test)]
mod tests;
