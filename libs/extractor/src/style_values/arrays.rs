use super::StyleValues;
use crate::finite_styles::FiniteStyles;
use oxc_ast::ast::Expression;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

impl StyleValues {
    pub fn arrays(&self, expression: &Expression<'_>) -> Option<&[FiniteStyles]> {
        let span = expression.span();
        self.array_origins
            .get(&(span.start, span.end))
            .or_else(|| {
                self.symbol(crate::utils::unwrap_syntax_only(expression))
                    .and_then(|symbol| self.arrays.get(&symbol))
            })
            .or_else(|| {
                self.imported_name(expression)
                    .and_then(|name| self.imported_arrays.get(&name))
            })
            .map(Vec::as_slice)
    }

    pub fn chosen_array(&self, expression: &Expression<'_>) -> Option<Vec<FiniteStyles>> {
        if let Some(array) = self.arrays(expression) {
            return Some(array.to_vec());
        }
        let Expression::ArrayExpression(array) = crate::utils::unwrap_syntax_only(expression)
        else {
            return None;
        };
        array
            .elements
            .iter()
            .map(|value| self.chosen(value.as_expression()?))
            .collect()
    }

    pub fn remember_array(&mut self, symbol: SymbolId, span: Span, array: Vec<FiniteStyles>) {
        self.array_origins
            .insert((span.start, span.end), array.clone());
        self.arrays.insert(symbol, array);
    }

    pub fn array_origin(&mut self, span: Span, array: Vec<FiniteStyles>) {
        self.array_origins.insert((span.start, span.end), array);
    }
    pub fn array_origins(&self) -> FxHashMap<(u32, u32), Vec<FiniteStyles>> {
        self.array_origins.clone()
    }
    pub fn import_arrays(&mut self, arrays: FxHashMap<String, Vec<FiniteStyles>>) {
        self.imported_arrays = arrays;
    }
}
