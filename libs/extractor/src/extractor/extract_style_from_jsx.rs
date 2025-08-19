use crate::extractor::{
    ExtractResult, extract_style_from_expression::extract_style_from_expression,
};
use oxc_allocator::CloneIn;
use oxc_ast::{
    AstBuilder,
    ast::{Expression, JSXAttributeValue},
};

pub fn extract_style_from_jsx<'a>(
    ast_builder: &AstBuilder<'a>,
    name: &str,
    value: &mut JSXAttributeValue<'a>,
) -> ExtractResult<'a> {
    match value {
        JSXAttributeValue::ExpressionContainer(expression) => expression
            .expression
            .as_expression()
            .map(|expression| expression.clone_in(ast_builder.allocator)),
        JSXAttributeValue::StringLiteral(literal) => Some(Expression::StringLiteral(
            literal.clone_in(ast_builder.allocator),
        )),
        _ => None,
    }
    .map(|mut expression| {
        extract_style_from_expression(ast_builder, Some(name), &mut expression, 0, &None)
    })
    .unwrap_or_default()
}
