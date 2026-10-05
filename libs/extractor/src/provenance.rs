//! Where the values of styles come from, as far as production names care:
//! whether a style exists only because of what an import may resolve to, and
//! where in its original file a dynamic style was written.
//!
//! A risky expression is marked in its span, which keeps the position it was
//! written at: the mark is a high bit of the start and the span is empty, so
//! code generation never maps it.

use css::Naming;
use oxc_ast::ast::Expression;
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_span::{GetSpan, GetSpanMut};

use crate::{ExtractStyleProp, utils::unwrap_syntax_only};

pub(crate) use crate::sparse_sites::{SiteScope, normalize_source, site_at, site_errors};

const RISKY: u32 = 1 << 31;

/// A position in the source, whatever mark the span carries.
pub(crate) const fn source_offset(start: u32) -> u32 {
    start & !RISKY
}

pub(crate) const fn naming_of_start(start: u32) -> Naming {
    if start & RISKY == 0 {
        Naming::Own
    } else {
        Naming::Risky
    }
}

const fn naming_of_span(span: oxc_span::Span) -> Naming {
    naming_of_start(span.start)
}

/// Mark `expression` as depending on `naming`.
pub(crate) fn mark(expression: &mut Expression<'_>, naming: Naming) {
    if naming == Naming::Risky {
        let span = expression.span_mut();
        span.start |= RISKY;
        span.end = span.start;
    }
}

struct Marks(Naming);

impl<'a> Visit<'a> for Marks {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        self.0 = self.0.join(naming_of_span(expression.span()));
        walk::walk_expression(self, expression);
    }
}

/// The identity of every mark within `expression`.
fn scan(expression: &Expression<'_>) -> Naming {
    let mut marks = Marks(Naming::Own);
    marks.visit_expression(expression);
    marks.0
}

/// What the styles read from `expression` depend on. An object or array
/// leaves that to its members, a condition to the value it tests too, as
/// that may pick the branches; anything else is one value.
pub(crate) fn provenance_of(expression: &Expression<'_>) -> Naming {
    let inner = unwrap_syntax_only(expression);
    let wrapper = naming_of_span(expression.span()).join(naming_of_span(inner.span()));
    match inner {
        Expression::ObjectExpression(_) | Expression::ArrayExpression(_) => wrapper,
        Expression::ConditionalExpression(conditional) => wrapper.join(scan(&conditional.test)),
        Expression::LogicalExpression(logical) => wrapper.join(scan(&logical.left)),
        _ => wrapper.join(scan(inner)),
    }
}

/// Join `naming` into every style of `props`.
pub(crate) fn join_naming(props: &mut [ExtractStyleProp<'_>], naming: Naming) {
    for prop in props {
        match prop {
            ExtractStyleProp::Static(style) => style.join_naming(naming),
            ExtractStyleProp::StaticArray(props)
            | ExtractStyleProp::Evaluated { styles: props, .. } => join_naming(props, naming),
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                for branch in [consequent, alternate].into_iter().flatten() {
                    join_naming(std::slice::from_mut(branch.as_mut()), naming);
                }
            }
            ExtractStyleProp::Enum { map, .. } => {
                for props in map.values_mut() {
                    join_naming(props, naming);
                }
            }
            ExtractStyleProp::MemberExpression { map, .. } => {
                for prop in map.values_mut() {
                    join_naming(std::slice::from_mut(prop.as_mut()), naming);
                }
            }
            ExtractStyleProp::Expression { styles, .. } => {
                for style in styles {
                    style.join_naming(naming);
                }
            }
            ExtractStyleProp::Unreadable { .. } => {}
        }
    }
}

/// Marks the expressions that lie within `ranges`, which a build-time
/// evaluation wrote in place of values that depend on an import.
pub(crate) struct MarkRanges<'r>(pub &'r [(usize, usize)]);

impl<'a> VisitMut<'a> for MarkRanges<'_> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        let start = source_offset(expression.span().start) as usize;
        if self
            .0
            .iter()
            .any(|(from, to)| (*from..*to).contains(&start))
        {
            mark(expression, Naming::Risky);
        }
        walk_mut::walk_expression(self, expression);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marks_keep_the_position_they_were_written_at() {
        assert_eq!(naming_of_start(10), Naming::Own);
        assert_eq!(naming_of_start(0xa | RISKY), Naming::Risky);
        assert_eq!(source_offset(0xa | RISKY), 10);
    }
}
