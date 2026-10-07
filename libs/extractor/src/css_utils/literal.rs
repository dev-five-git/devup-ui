use std::ops::Range;

use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, Str, TemplateElement, TemplateElementValue, TemplateLiteral};
use oxc_ast::builder::AstBuilder;
use oxc_span::{GetSpan, Span};

use super::cursor;

pub(crate) struct CssText<'a> {
    pub text: String,
    pub holes: Vec<(Range<usize>, Expression<'a>)>,
    origins: Vec<(Range<usize>, u32)>,
    span: Span,
    fixed_origin: bool,
}

impl<'a> CssText<'a> {
    pub(crate) fn new(ast: &AstBuilder<'a>, expression: &Expression<'a>) -> Option<Self> {
        Self::with_source(ast, expression, None)
    }

    pub(crate) fn with_source(
        ast: &AstBuilder<'a>,
        expression: &Expression<'a>,
        source: Option<&str>,
    ) -> Option<Self> {
        let mut result = Self {
            text: String::new(),
            holes: Vec::new(),
            origins: Vec::new(),
            span: expression.span(),
            fixed_origin: false,
        };
        match crate::utils::unwrap_syntax_only(expression) {
            Expression::StringLiteral(literal) => {
                result.text.push_str(&literal.value);
                if let Some(raw) = &literal.raw {
                    result.origins = super::literal_origins::string_ranges(
                        raw,
                        &literal.value,
                        literal.span.start + 1,
                    );
                } else {
                    result.fixed_origin = true;
                    result
                        .origins
                        .push((0..result.text.len(), literal.span.start));
                }
            }
            Expression::TemplateLiteral(template) => {
                result = Self::from_template(ast, template, source);
                result.span = expression.span();
            }
            _ => return None,
        }
        Some(result)
    }

    pub(crate) fn from_template(
        ast: &AstBuilder<'a>,
        template: &TemplateLiteral<'_>,
        source: Option<&str>,
    ) -> Self {
        let mut result = Self {
            text: String::new(),
            holes: Vec::new(),
            origins: Vec::new(),
            span: template.span,
            fixed_origin: false,
        };
        for (index, quasi) in template.quasis.iter().enumerate() {
            let start = result.text.len();
            result.text.push_str(&quasi.value.raw);
            if let Some(ranges) = source.and_then(|source| {
                super::literal_origins::quasi_ranges(&quasi.value.raw, source, quasi.span.start)
            }) {
                result.origins.extend(
                    ranges
                        .into_iter()
                        .map(|(range, origin)| (range.start + start..range.end + start, origin)),
                );
            } else {
                result
                    .origins
                    .push((start..result.text.len(), quasi.span.start));
            }
            if let Some(expression) = template.expressions.get(index) {
                let start = result.text.len();
                result.text.push_str("__DEVUP_HOLE_");
                result.text.push_str(&index.to_string());
                result.text.push_str("__");
                result.holes.push((
                    start..result.text.len(),
                    expression.clone_in_with_semantic_ids(ast.allocator()),
                ));
            }
        }
        result
    }

    pub(crate) fn offset(&self, at: usize) -> u32 {
        if self.fixed_origin {
            return self.span.start;
        }
        if let Some((_, expression)) = self.holes.iter().find(|(range, _)| range.contains(&at)) {
            return expression.span().start;
        }
        self.origins
            .iter()
            .find(|(range, _)| range.contains(&at))
            .map_or(self.span.start, |(range, start)| {
                start.saturating_add(u32::try_from(at - range.start).unwrap_or(u32::MAX))
            })
    }

    pub(crate) fn span(&self, range: &Range<usize>) -> Span {
        Span::new(
            self.offset(range.start),
            self.offset(range.end.saturating_sub(1)).saturating_add(1),
        )
    }

    pub(crate) fn value(
        &self,
        ast: &AstBuilder<'a>,
        range: Range<usize>,
        order: bool,
    ) -> Expression<'a> {
        let span = self.span(&range);
        let holes: Vec<_> = self
            .holes
            .iter()
            .filter(|(hole, _)| {
                hole.start >= range.start && hole.end <= range.end && self.visible(hole.start)
            })
            .collect();
        if let [(hole, expression)] = holes.as_slice()
            && cursor::clean(&self.text[range.start..hole.start])
                .trim()
                .is_empty()
            && cursor::clean(&self.text[hole.end..range.end])
                .trim()
                .is_empty()
        {
            return expression.clone_in_with_semantic_ids(ast.allocator());
        }
        if holes.is_empty() {
            let value = super::literal_trivia::clean_value(&self.text[range], order);
            let value = value.trim();
            if order {
                return super::literal_values::token(ast, value, span);
            }
            return Expression::new_string_literal(
                span,
                ast.allocator().alloc_str(value),
                None,
                ast,
            );
        }
        let mut from = range.start;
        let mut quasis = oxc_allocator::Vec::new_in(ast);
        let mut expressions = oxc_allocator::Vec::new_in(ast);
        for (hole, expression) in &holes {
            let raw = ast
                .allocator()
                .alloc_str(&super::literal_trivia::clean_value(
                    &self.text[from..hole.start],
                    order,
                ));
            quasis.push(TemplateElement::new(
                self.span(&(from..hole.start)),
                TemplateElementValue {
                    raw: Str::from(raw),
                    cooked: Some(Str::from(raw)),
                },
                false,
                ast,
            ));
            expressions.push(expression.clone_in_with_semantic_ids(ast.allocator()));
            from = hole.end;
        }
        let raw = ast
            .allocator()
            .alloc_str(&super::literal_trivia::clean_value(
                &self.text[from..range.end],
                order,
            ));
        quasis.push(TemplateElement::new(
            self.span(&(from..range.end)),
            TemplateElementValue {
                raw: Str::from(raw),
                cooked: Some(Str::from(raw)),
            },
            true,
            ast,
        ));
        let value = Expression::new_template_literal(span, quasis, expressions, ast);
        if order {
            super::literal_values::finite_text(ast, value)
        } else {
            value
        }
    }
}

pub(crate) fn lower<'a>(ast: &AstBuilder<'a>, expression: &mut Expression<'a>) -> bool {
    lower_with_source(ast, expression, None)
}

pub(crate) fn is_rule_text(expression: &Expression<'_>) -> bool {
    match crate::utils::unwrap_syntax_only(expression) {
        Expression::StringLiteral(value) => value.value.contains(':'),
        Expression::TemplateLiteral(value) => value
            .quasis
            .iter()
            .any(|quasi| quasi.value.raw.contains(':')),
        _ => false,
    }
}

pub(crate) fn lower_with_source<'a>(
    ast: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    source: Option<&str>,
) -> bool {
    let Some(text) = CssText::with_source(ast, expression, source) else {
        return false;
    };
    if !text.has_order(0..text.text.len()) {
        return false;
    }
    *expression = text.object(ast, 0..text.text.len());
    true
}
