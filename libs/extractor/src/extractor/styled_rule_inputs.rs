//! Shared structural admission for composition, without extracting declarations.
use super::super::StyledBindings;
use crate::utils::{StyleArguments, style_arguments, unwrap_syntax_only};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Argument, Expression},
    builder::AstBuilder,
};

pub(super) enum Input<'s, 'a> {
    Rules(&'s Expression<'a>),
    Class(&'s Expression<'a>),
    Prepared(StyleArguments<'a>),
}

pub(super) fn visit<'a>(
    ast: &AstBuilder<'a>,
    (arguments, bindings): (&[Argument<'a>], Option<&StyledBindings<'_>>),
    mut consume: impl FnMut(Input<'_, 'a>),
) -> Option<()> {
    let mut pending = arguments
        .iter()
        .rev()
        .map(|argument| match argument {
            Argument::SpreadElement(spread) => match unwrap_syntax_only(&spread.argument) {
                Expression::ArrayExpression(_) => Some(&spread.argument),
                _ => None,
            },
            argument => argument.as_expression(),
        })
        .collect::<Option<Vec<_>>>()?;
    while let Some(source) = pending.pop() {
        if let Expression::ArrayExpression(array) = unwrap_syntax_only(source) {
            for element in array.elements.iter().rev() {
                let source = match element {
                    oxc_ast::ast::ArrayExpressionElement::SpreadElement(spread) => {
                        match unwrap_syntax_only(&spread.argument) {
                            Expression::ArrayExpression(_) => &spread.argument,
                            _ => return None,
                        }
                    }
                    element => element.as_expression()?,
                };
                pending.push(source);
            }
            continue;
        }
        match unwrap_syntax_only(source) {
            rules if written(rules, bindings) => consume(Input::Rules(rules)),
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => {
                consume(Input::Class(source));
            }
            _ => {
                let argument = Argument::from(source.clone_in_with_semantic_ids(ast.allocator()));
                consume(Input::Prepared(style_arguments(
                    ast,
                    std::slice::from_ref(&argument),
                )?));
            }
        }
    }
    Some(())
}

fn written(rules: &Expression<'_>, bindings: Option<&StyledBindings<'_>>) -> bool {
    if bindings.is_some_and(|bindings| bindings.styles(rules).is_some()) {
        return true;
    }
    match unwrap_syntax_only(rules) {
        Expression::ObjectExpression(_)
        | Expression::NullLiteral(_)
        | Expression::BooleanLiteral(_) => true,
        Expression::Identifier(identifier) => identifier.name == "undefined",
        Expression::ConditionalExpression(branch) => {
            written(&branch.consequent, bindings) && written(&branch.alternate, bindings)
        }
        _ => false,
    }
}
