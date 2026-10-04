use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, Str, TemplateElement, TemplateElementValue, TemplateLiteral},
    builder::AstBuilder,
};

// Use the CSS parser's lexer so quotes, escapes, comments and functions never
// turn data braces into enclosing rule contexts.
mod blocks {
    include!("../css_utils/layer_blocks.rs");
}

pub(super) enum Part<'a> {
    Text(TemplateLiteral<'a>),
    Mixin { index: usize, context: Vec<String> },
    Unplaced(usize),
}

pub(super) fn split<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    known: impl Fn(&Expression<'a>) -> bool,
) -> Vec<Part<'a>> {
    let mut parts = Vec::new();
    let mut before = String::new();
    let mut from = 0;
    let mut opening = Vec::new();
    for (index, expression) in template.expressions.iter().enumerate() {
        before.push_str(&template.quasis[index].value.raw);
        let place = place(&before, &template.quasis[index + 1..]);
        match place {
            crate::css_utils::Place::Statement
                if known(expression)
                    || crate::utils::get_string_by_literal_expression(expression).is_none() =>
            {
                let context = contexts(&before);
                parts.push(Part::Text(segment(
                    ast,
                    template,
                    Segment {
                        range: from..=index,
                        opening: &opening,
                        close: context.len(),
                    },
                )));
                parts.push(Part::Mixin {
                    index,
                    context: context.clone(),
                });
                from = index + 1;
                opening = context;
                before.push(';');
            }
            crate::css_utils::Place::Value => before.push_str("__devupValue"),
            _ => {
                if let Some(text) = crate::utils::get_string_by_literal_expression(expression) {
                    before.push_str(&text);
                } else {
                    parts.push(Part::Unplaced(index));
                }
            }
        }
    }
    parts.push(Part::Text(segment(
        ast,
        template,
        Segment {
            range: from..=template.expressions.len(),
            opening: &opening,
            close: 0,
        },
    )));
    parts
}

fn place(before: &str, after: &[TemplateElement<'_>]) -> crate::css_utils::Place {
    let from = blocks::boundaries(before)
        .last()
        .map_or(0, |(index, _)| index + 1);
    let head = css::rm_css_comment::rm_css_comment(&before[from..]);
    let rest: String = after.iter().map(|quasi| quasi.value.raw.as_str()).collect();
    if blocks::boundaries(&rest)
        .next()
        .is_some_and(|(_, boundary)| boundary == '{')
    {
        return crate::css_utils::Place::Other;
    }
    if head.contains(':') {
        crate::css_utils::Place::Value
    } else if head.trim().is_empty() {
        crate::css_utils::Place::Statement
    } else {
        crate::css_utils::Place::Other
    }
}

fn contexts(text: &str) -> Vec<String> {
    let mut stack = Vec::new();
    let mut from = 0;
    for (index, boundary) in blocks::boundaries(text) {
        match boundary {
            '{' => stack.push(text[from..=index].to_string()),
            '}' => {
                stack.pop();
            }
            _ => {}
        }
        from = index + 1;
    }
    stack
}

struct Segment<'s> {
    range: std::ops::RangeInclusive<usize>,
    opening: &'s [String],
    close: usize,
}

fn segment<'a>(
    ast: &AstBuilder<'a>,
    template: &TemplateLiteral<'a>,
    segment: Segment<'_>,
) -> TemplateLiteral<'a> {
    let Segment {
        range,
        opening,
        close,
    } = segment;
    let from = *range.start();
    let to = *range.end();
    let quasis = template.quasis[range]
        .iter()
        .enumerate()
        .map(|(index, quasi)| {
            let raw = format!(
                "{}{}{}",
                if index == 0 {
                    opening.concat()
                } else {
                    String::new()
                },
                quasi.value.raw,
                if index == to - from {
                    "}".repeat(close)
                } else {
                    String::new()
                }
            );
            TemplateElement::new(
                quasi.span,
                TemplateElementValue {
                    raw: Str::from_in(raw.as_str(), ast.allocator()),
                    cooked: None,
                },
                index == to - from,
                ast,
            )
        });
    TemplateLiteral::new(
        template.span,
        oxc_allocator::Vec::from_iter_in(quasis, ast),
        oxc_allocator::Vec::from_iter_in(
            template.expressions[from..to]
                .iter()
                .map(|expression| expression.clone_in_with_semantic_ids(ast.allocator())),
            ast,
        ),
        ast,
    )
}
