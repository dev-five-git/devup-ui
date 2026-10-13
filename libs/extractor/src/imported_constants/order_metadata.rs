use crate::css_utils::{cursor, literal::CssText};
use oxc_ast::{ast::TemplateLiteral, builder::AstBuilder};
use oxc_span::Span;
use std::ops::Range;

pub(crate) fn holes(ast: &AstBuilder<'_>, template: &TemplateLiteral<'_>) -> Vec<Span> {
    let text = CssText::from_template(ast, template, None);
    let mut ranges = Vec::new();
    declarations(&text, 0..text.text.len(), &mut ranges);
    text.holes
        .iter()
        .filter(|(hole, _)| {
            ranges.iter().any(|range| {
                hole.start >= range.start
                    && hole.end <= range.end
                    && cursor::clean(&text.text[range.start..hole.start])
                        .trim()
                        .is_empty()
                    && cursor::clean(&text.text[hole.end..range.end])
                        .trim()
                        .is_empty()
            })
        })
        .map(|(_, expression)| oxc_span::GetSpan::span(expression))
        .collect()
}

fn declarations(text: &CssText<'_>, range: Range<usize>, values: &mut Vec<Range<usize>>) {
    for item in cursor::items(&text.text[range.clone()]) {
        match item {
            cursor::Item::Declaration { key, value } => {
                if text
                    .key(key.start + range.start..key.end + range.start)
                    .is_ok_and(|key| crate::style_order::reserved(&key))
                {
                    values.push(value.start + range.start..value.end + range.start);
                }
            }
            cursor::Item::Block { body, .. } => declarations(
                text,
                body.start + range.start..body.end + range.start,
                values,
            ),
            cursor::Item::Statement(_) => {}
        }
    }
}
