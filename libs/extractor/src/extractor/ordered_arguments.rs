use super::extract_style_from_expression::{LiteralHandling, extract_style_from_expression};
use crate::ExtractStyleProp;
use crate::utils::unwrap_syntax_only;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Argument, Expression},
    builder::AstBuilder,
};

pub(super) fn extract<'a>(
    builder: &AstBuilder<'a>,
    arguments: &[Argument<'a>],
) -> Option<Vec<ExtractStyleProp<'a>>> {
    let mut styles = Vec::new();
    for argument in arguments {
        collect(builder, argument.as_expression()?, &mut styles)?;
    }
    Some(styles)
}

fn collect<'a>(
    builder: &AstBuilder<'a>,
    value: &Expression<'a>,
    styles: &mut Vec<ExtractStyleProp<'a>>,
) -> Option<()> {
    match unwrap_syntax_only(value) {
        Expression::ArrayExpression(array) => {
            for element in &array.elements {
                collect(builder, element.as_expression()?, styles)?;
            }
        }
        Expression::ObjectExpression(_)
        | Expression::ConditionalExpression(_)
        | Expression::LogicalExpression(_) => {
            let mut value = value.clone_in(builder.allocator());
            styles.extend(
                extract_style_from_expression(
                    builder,
                    None,
                    &mut value,
                    0,
                    &None,
                    LiteralHandling::ExpandResponsiveThemeToken,
                )
                .styles,
            );
        }
        _ => return None,
    }
    Some(())
}
