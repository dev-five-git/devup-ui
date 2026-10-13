//! Closed primitive class results retain declarations without retaining source predicates.

use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{BinaryOperator, Expression, Str},
    builder::AstBuilder,
};
use oxc_span::SPAN;

mod outcomes;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FiniteStyles {
    pub results: Vec<(String, Vec<ExtractStyleValue>)>,
}

impl FiniteStyles {
    pub fn union(mut self, other: Self) -> Option<Self> {
        for entry in other.results {
            if self
                .results
                .iter()
                .any(|(text, values)| *text == entry.0 && *values != entry.1)
            {
                return None;
            }
            if !self.results.contains(&entry) {
                self.results.push(entry);
            }
        }
        Some(self)
    }
    /// Read actual emitted expressions, including their whitespace, rather than rebuilding tokens.
    pub fn emitted<'a>(
        ast: &AstBuilder<'a>,
        props: &[ExtractStyleProp<'a>],
        result: &Expression<'a>,
    ) -> Option<Self> {
        let rows = outcomes::styles(props)?;
        let mut results: Vec<(String, Vec<ExtractStyleValue>)> = Vec::new();
        for row in rows {
            let text = outcomes::text(result, &row)?;
            let mut composition = crate::composition::Composition::default();
            composition.apply(
                ast,
                row.values
                    .into_iter()
                    .map(ExtractStyleProp::Static)
                    .collect(),
            );
            let mut values = composition.unconditional()?;
            values.sort();
            if let Some((_, previous)) = results.iter().find(|(value, _)| *value == text) {
                if *previous != values {
                    return None;
                }
            } else {
                results.push((text, values));
            }
        }
        Some(Self { results })
    }

    /// Only a proven producer result is compared, always as a whole primitive string.
    pub fn props<'a>(
        &self,
        ast: &AstBuilder<'a>,
        saved: &Expression<'a>,
    ) -> Vec<ExtractStyleProp<'a>> {
        if let [(_, values)] = self.results.as_slice() {
            return values
                .iter()
                .cloned()
                .map(ExtractStyleProp::Static)
                .collect();
        }
        let mut choice = None;
        for (text, values) in self
            .results
            .iter()
            .rev()
            .filter(|(_, values)| !values.is_empty())
        {
            choice = Some(ExtractStyleProp::Conditional {
                condition: Expression::new_binary_expression(
                    SPAN,
                    saved.clone_in(ast.allocator()),
                    BinaryOperator::StrictEquality,
                    Expression::new_string_literal(
                        SPAN,
                        Str::from_in(text.as_str(), ast.allocator()),
                        None,
                        ast,
                    ),
                    ast,
                ),
                consequent: Some(Box::new(ExtractStyleProp::StaticArray(
                    values
                        .iter()
                        .cloned()
                        .map(ExtractStyleProp::Static)
                        .collect(),
                ))),
                alternate: choice.map(Box::new),
            });
        }
        choice.into_iter().collect()
    }
}
