use super::{
    AstBuilder, BinaryOperator, CloneIn, Expression, ExtractStyleProp, FromIn, GetAllocator, SPAN,
    Str,
};

pub(super) fn project<'a, C>(
    ast: &AstBuilder<'a>,
    finite: &crate::finite_styles::FiniteStyles,
    saved: &Expression<'a>,
) -> Vec<ExtractStyleProp<'a, C>> {
    if let [(_, values)] = finite.results.as_slice() {
        return values
            .iter()
            .cloned()
            .map(ExtractStyleProp::Static)
            .collect();
    }
    let mut choice: Option<ExtractStyleProp<'a, C>> = None;
    for (text, values) in finite
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
