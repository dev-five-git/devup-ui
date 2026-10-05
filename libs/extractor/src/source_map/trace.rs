//! Offsets of generated text, traced back to the text it was made from

use oxc_ast::{
    AstKind, AstType,
    ast::{Expression, ParenthesizedExpression, Program},
};
use oxc_ast_visit::Visit;
use oxc_sourcemap::SourceMap;
use oxc_span::{GetSpan, SourceType};

use super::Lines;
use crate::import_alias_visit::{Edit, source_offset};

#[cfg(test)]
mod tests;

/// `(offset in generated, offset in parsed)` for every token of `map`, the map
/// of `generated` made from `parsed`; in ascending order
pub(crate) fn marks(map: &SourceMap<'_>, generated: &str, parsed: &str) -> Vec<(usize, usize)> {
    let (generated_lines, parsed_lines) = (Lines::new(generated), Lines::new(parsed));
    map.get_tokens()
        .map(|token| {
            (
                generated_lines.offset(token.get_dst_line(), token.get_dst_col()),
                parsed_lines.offset(token.get_src_line(), token.get_src_col()),
            )
        })
        .collect()
}

#[derive(Default)]
struct Syntax {
    shape: Vec<(AstType, bool)>,
    starts: Vec<(usize, usize)>,
}

/// The first expression token, not the parentheses codegen uses to group it.
fn expression_start(expression: &Expression<'_>) -> usize {
    let expression = expression.get_inner_expression();
    match expression {
        Expression::CallExpression(call) => expression_start(&call.callee),
        Expression::StaticMemberExpression(member) => expression_start(&member.object),
        Expression::ComputedMemberExpression(member) => expression_start(&member.object),
        Expression::PrivateFieldExpression(member) => expression_start(&member.object),
        _ => expression.span().start as usize,
    }
}

impl<'a> Visit<'a> for Syntax {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.shape.push((kind.ty(), true));
        let original = match kind {
            AstKind::CallExpression(call) => expression_start(&call.callee),
            AstKind::StaticMemberExpression(member) => expression_start(&member.object),
            AstKind::ComputedMemberExpression(member) => expression_start(&member.object),
            AstKind::PrivateFieldExpression(member) => expression_start(&member.object),
            _ => kind.span().start as usize,
        };
        self.starts.push((kind.span().start as usize, original));
    }

    fn leave_node(&mut self, kind: AstKind<'a>) {
        self.shape.push((kind.ty(), false));
    }

    fn visit_parenthesized_expression(&mut self, expression: &ParenthesizedExpression<'a>) {
        self.visit_expression(&expression.expression);
    }
}

/// Completes sparse codegen marks with matching syntax starts. Pairing is
/// structural, after transformation, so duplicate expressions stay distinct.
pub(crate) fn complete_marks(
    program: &Program<'_>,
    generated: &str,
    marks: &mut Vec<(usize, usize)>,
) {
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, generated, SourceType::default()).parse();
    let mut original = Syntax::default();
    original.visit_program(program);
    let mut printed = Syntax::default();
    printed.visit_program(&parsed.program);
    if !parsed.diagnostics.is_empty() || original.shape != printed.shape {
        return;
    }
    let sparse = std::mem::take(marks);
    marks.extend(
        printed
            .starts
            .into_iter()
            .zip(original.starts)
            .map(|((at, _), (_, original))| (at, original))
            .filter(|(at, _)| sparse.binary_search_by_key(at, |(at, _)| *at).is_err()),
    );
    marks.extend(sparse);
    marks.sort_by_key(|&(generated, _)| generated);
}

/// Where the text of a stage came from in the source as written
pub(crate) struct Trace {
    /// `(offset in generated, offset in source)`, ascending, the start of the
    /// generated text first
    marks: Vec<(usize, usize)>,
}

impl Trace {
    /// `marks` tie the generated text to the code it was parsed from; each layer
    /// of `edits`, last made first, maps that code back to the source
    pub(crate) fn new(marks: &[(usize, usize)], edits: &[&[Edit]]) -> Self {
        let back = |offset| {
            edits
                .iter()
                .fold(offset, |offset, edits| source_offset(edits, offset))
        };
        Self {
            marks: std::iter::once((0, back(0)))
                .chain(
                    marks
                        .iter()
                        .map(|&(generated, parsed)| (generated, back(parsed))),
                )
                .collect(),
        }
    }

    /// The offset in `source` of the byte `offset` of `generated`: the one of
    /// the closest mark before it, moved on by the text both share after it
    pub(crate) fn resolve(&self, generated: &str, source: &str, offset: usize) -> usize {
        let at = self.marks.partition_point(|(from, _)| *from <= offset);
        let (from, to) = self.marks[at - 1];
        let shared = generated.as_bytes().get(from..offset).unwrap_or_default();
        if source.as_bytes().get(to..to + shared.len()) == Some(shared) {
            to + shared.len()
        } else {
            to
        }
    }
}
