use super::StyleValues;
use crate::finite_styles::FiniteStyles;
use crate::utils::{get_string_by_literal_expression, unwrap_syntax_only};
use oxc_ast::ast::Expression;
use oxc_span::{GetSpan, Span};
use oxc_syntax::{operator::LogicalOperator, symbol::SymbolId};
use rustc_hash::FxHashMap;

impl StyleValues {
    pub fn origin(&mut self, span: Span, styles: FiniteStyles) {
        if span.end > span.start {
            self.origins.insert((span.start, span.end), styles);
        }
    }
    pub fn at(&self, span: Span) -> Option<&FiniteStyles> {
        self.origins.get(&(span.start, span.end))
    }
    pub fn origins(&self) -> FxHashMap<(u32, u32), FiniteStyles> {
        self.origins.clone()
    }
    pub fn import_finite(&mut self, finite: FxHashMap<String, FiniteStyles>) {
        self.imported_finite = finite;
    }
    pub fn remember_finite(&mut self, symbol: SymbolId, styles: FiniteStyles) {
        self.finite.insert(symbol, styles);
    }

    pub fn finite(&self, expression: &Expression<'_>) -> Option<&FiniteStyles> {
        self.at(expression.span()).or_else(|| {
            if let Expression::ComputedMemberExpression(member) = unwrap_syntax_only(expression)
                && let Some(key) = get_string_by_literal_expression(&member.expression)
                && let Ok(index) = key.parse::<usize>()
                && index.to_string() == key
                && let Some(value) = self
                    .arrays(&member.object)
                    .and_then(|array| array.get(index))
            {
                return Some(value);
            }
            self.symbol(unwrap_syntax_only(expression))
                .and_then(|symbol| self.finite.get(&symbol))
                .or_else(|| {
                    self.imported_name(expression)
                        .and_then(|name| self.imported_finite.get(&name))
                })
        })
    }

    pub fn chosen(&self, expression: &Expression<'_>) -> Option<FiniteStyles> {
        if let Some(finite) = self.finite(expression) {
            return Some(finite.clone());
        }
        match unwrap_syntax_only(expression) {
            Expression::ConditionalExpression(value) => self
                .chosen(&value.consequent)?
                .union(self.chosen(&value.alternate)?),
            Expression::LogicalExpression(value) => {
                let left = self.chosen(&value.left)?;
                match value.operator {
                    LogicalOperator::Coalesce => Some(left),
                    LogicalOperator::Or => {
                        if left.results.iter().all(|(text, _)| !text.is_empty()) {
                            return Some(left);
                        }
                        let results = left
                            .results
                            .into_iter()
                            .filter(|(text, _)| !text.is_empty())
                            .collect();
                        FiniteStyles { results }.union(self.chosen(&value.right)?)
                    }
                    LogicalOperator::And => {
                        let results = left
                            .results
                            .iter()
                            .filter(|(text, _)| text.is_empty())
                            .cloned()
                            .collect();
                        let finite = FiniteStyles { results };
                        if left.results.iter().any(|(text, _)| !text.is_empty()) {
                            finite.union(self.chosen(&value.right)?)
                        } else {
                            Some(finite)
                        }
                    }
                }
            }
            _ => None,
        }
    }

    pub(super) fn imported_name(&self, expression: &Expression<'_>) -> Option<String> {
        match unwrap_syntax_only(expression) {
            expression @ Expression::Identifier(_) => {
                let symbol = self.symbol(expression)?;
                let scoping = self.scoping.as_ref()?;
                scoping
                    .symbol_flags(symbol)
                    .is_import()
                    .then(|| scoping.symbol_name(symbol).to_string())
            }
            Expression::StaticMemberExpression(member) => Some(format!(
                "{}.{}",
                self.imported_name(&member.object)?,
                member.property.name
            )),
            Expression::ComputedMemberExpression(member) => Some(format!(
                "{}.{}",
                self.imported_name(&member.object)?,
                get_string_by_literal_expression(&member.expression)?
            )),
            _ => None,
        }
    }
}
