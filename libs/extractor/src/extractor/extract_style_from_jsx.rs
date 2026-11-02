use super::style_inputs::StyleInputs;
use crate::extractor::{
    ExtractResult,
    extract_style_from_expression::{LiteralHandling, extract_style_from_expression},
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, JSXAttributeValue};

pub fn extract_style_from_jsx<'a>(
    inputs: StyleInputs<'_, 'a>,
    name: &str,
    value: &mut JSXAttributeValue<'a>,
) -> ExtractResult<'a> {
    let ast_builder = inputs.ast;
    let expression = match value {
        JSXAttributeValue::ExpressionContainer(expression) => expression
            .expression
            .as_expression()
            .map(|expression| expression.clone_in(ast_builder.allocator())),
        JSXAttributeValue::StringLiteral(literal) => Some(Expression::StringLiteral(
            literal.clone_in(ast_builder.allocator()),
        )),
        _ => None,
    };

    expression
        .map(|mut expression| {
            extract_style_from_expression(
                inputs,
                Some(name),
                &mut expression,
                0,
                &None,
                LiteralHandling::ExpandResponsiveThemeToken,
            )
        })
        .unwrap_or_default()
}
