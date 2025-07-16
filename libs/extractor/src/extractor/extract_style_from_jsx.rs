use crate::extractor::{
    ExtractResult, extract_style_from_expression::extract_style_from_expression,
};
use css::style_selector::StyleSelector;
use oxc_allocator::CloneIn;
use oxc_ast::{
    AstBuilder,
    ast::{Expression, JSXAttributeValue},
};

pub fn extract_style_from_jsx<'a>(
    ast_builder: &AstBuilder<'a>,
    name: &str,
    value: &mut JSXAttributeValue<'a>,
    selector: Option<&StyleSelector>,
) -> ExtractResult<'a> {
    match value {
        JSXAttributeValue::ExpressionContainer(expression) => {
            if expression.expression.is_expression() {
                extract_style_from_expression(
                    ast_builder,
                    Some(name),
                    expression.expression.to_expression_mut(),
                    0,
                    selector,
                )
            } else {
                ExtractResult::default()
            }
        }
        JSXAttributeValue::StringLiteral(literal) => extract_style_from_expression(
            ast_builder,
            Some(name),
            &mut Expression::StringLiteral(literal.clone_in(ast_builder.allocator)),
            0,
            selector,
        ),
        _ => ExtractResult::default(),
    }
}
