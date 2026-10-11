use super::{NESTED_MIXIN, UNPLACED};
use crate::css_utils::{Place, interpolation_place};
use crate::style_values::StyleValues;
use crate::utils::get_string_by_literal_expression;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, TemplateElement, TemplateElementValue, TemplateLiteral};
use oxc_ast::builder::AstBuilder;

enum Context<'s> {
    Default(bool),
    Styled(&'s StyleValues),
}

/// CSS text as the parts of a `css` prop: when `mixins`, an interpolation
/// standing where a declaration would is a mixin composed there, splitting the
/// text around it. `Err` holds an interpolation the parts cannot place, with
/// what the text requires of it.
pub(crate) fn template_parts<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    mixins: bool,
) -> Result<Vec<Expression<'a>>, (Expression<'a>, &'static str)> {
    parts(ast, template, Context::Default(mixins))
}

/// Styled CSS text retains a singleton producer's statement as a mixin rather
/// than folding its emitted primitive into rule text.
pub(crate) fn styled_template_parts<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    values: &StyleValues,
) -> Result<Vec<Expression<'a>>, (Expression<'a>, &'static str)> {
    parts(ast, template, Context::Styled(values))
}

fn parts<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    context: Context<'_>,
) -> Result<Vec<Expression<'a>>, (Expression<'a>, &'static str)> {
    let mixins = match &context {
        Context::Default(mixins) => *mixins,
        Context::Styled(_) => true,
    };
    let mut parts = Vec::new();
    let mut text = String::new();
    let mut from = 0;
    for (index, expression) in template.expressions.iter().enumerate() {
        let quasi = template.quasis[index].value.raw.as_str();
        text.push_str(quasi);
        let depth =
            crate::css_utils::cursor::boundaries(&text)
                .iter()
                .fold(0usize, |depth, (_, byte)| match byte {
                    b'{' => depth + 1,
                    b'}' => depth.saturating_sub(1),
                    _ => depth,
                });
        let place = interpolation_place(&text, &template.quasis[index + 1..]);
        let singleton_mixin = match &place {
            Place::Statement if depth == 0 => match &context {
                Context::Default(_) => false,
                Context::Styled(values) => values
                    .finite(expression)
                    .is_some_and(|finite| finite.results.len() == 1),
            },
            Place::Value | Place::Statement | Place::Other => false,
        };
        if !singleton_mixin
            && (matches!(place, Place::Value)
                || get_string_by_literal_expression(expression).is_some())
        {
            continue;
        }
        let requirement = match place {
            Place::Statement if mixins && depth == 0 => {
                parts.extend(segment(ast, template, from, index));
                parts.push(expression.clone_in_with_semantic_ids(ast.allocator()));
                from = index + 1;
                continue;
            }
            Place::Statement if mixins => NESTED_MIXIN,
            _ => UNPLACED,
        };
        return Err((expression.clone_in(ast.allocator()), requirement));
    }
    parts.extend(segment(ast, template, from, template.expressions.len()));
    Ok(parts)
}

/// The text of `template` from the quasi `from` to the quasi `to`, with the
/// values between them; `None` when it holds nothing
fn segment<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    from: usize,
    to: usize,
) -> Option<Expression<'a>> {
    let quasis = &template.quasis[from..=to];
    if from == to && quasis[0].value.raw.trim().is_empty() {
        return None;
    }
    let allocator = ast.allocator();
    let quasis = quasis.iter().enumerate().map(|(index, quasi)| {
        TemplateElement::new(
            quasi.span,
            TemplateElementValue {
                raw: quasi.value.raw,
                cooked: quasi.value.cooked,
            },
            index == to - from,
            ast,
        )
    });
    let expressions = template.expressions[from..to]
        .iter()
        .map(|expression| expression.clone_in_with_semantic_ids(allocator));
    Some(Expression::new_template_literal(
        template.span,
        oxc_allocator::Vec::from_iter_in(quasis, ast),
        oxc_allocator::Vec::from_iter_in(expressions, ast),
        ast,
    ))
}
