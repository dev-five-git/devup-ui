use std::path::Path;

use boa_engine::{Context, vm::SourcePath};
use css::style_origin::{RealLocation, StyleOrigin};
use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, Program};
use oxc_ast_visit::{Visit, walk};
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};
use oxc_transformer::{TransformOptions, Transformer};

use crate::import_alias_visit::{Edit, source_offset};

#[cfg(test)]
#[path = "evaluation_lookup_coverage_tests.rs"]
mod coverage_tests;

#[derive(Clone)]
pub(crate) struct StrippedSource {
    pub code: String,
    calls: Vec<(Span, Span)>,
}

impl StrippedSource {
    pub(crate) fn new(code: &str, filename: &str) -> Self {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(filename).unwrap_or_else(|_| SourceType::ts());
        let mut program = Parser::new(&allocator, code, source_type).parse().program;
        let authored = calls(&program);
        let scoping = SemanticBuilder::new()
            .with_enum_eval(true)
            .build(&program)
            .semantic
            .into_scoping();
        let options = TransformOptions::default();
        let _ = Transformer::new(&allocator, Path::new("input.css.ts"), &options)
            .build_with_scoping(scoping, &mut program);
        let transformed = calls(&program);
        let generated = Codegen::new().build(&program).code;
        let parsed = Parser::new(&allocator, &generated, SourceType::mjs()).parse();
        let printed = calls(&parsed.program);
        let mapped = if printed.len() == transformed.len() {
            printed
                .into_iter()
                .zip(transformed)
                .filter(|(_, original)| authored.contains(original))
                .collect()
        } else {
            Vec::new()
        };
        Self {
            code: generated,
            calls: mapped,
        }
    }

    pub(crate) fn authored_calls(
        &self,
        source: (&str, &str),
        edits: &[Edit],
    ) -> Vec<(Span, StyleOrigin)> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(source.0).unwrap_or_else(|_| SourceType::ts());
        let program = Parser::new(&allocator, source.1, source_type)
            .parse()
            .program;
        let authored = calls(&program);
        self.calls
            .iter()
            .filter_map(|(generated, original)| {
                let start = source_offset(edits, usize::try_from(original.start).ok()?);
                let end = source_offset(edits, usize::try_from(original.end).ok()?);
                let span = Span::new(u32::try_from(start).ok()?, u32::try_from(end).ok()?);
                if !authored.contains(&span) {
                    return None;
                }
                crate::style_origin::evaluated_at(source.0, source.1, span)
                    .map(|origin| (*generated, origin))
            })
            .collect()
    }
}

fn calls(program: &Program<'_>) -> Vec<Span> {
    struct Calls(Vec<Span>);
    impl<'a> Visit<'a> for Calls {
        fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
            self.0.push(call.span);
            walk::walk_call_expression(self, call);
        }
    }
    let mut visitor = Calls(Vec::new());
    visitor.visit_program(program);
    visitor.0
}

#[derive(Clone, Debug)]
pub(crate) struct CopiedSource {
    pub generated: std::ops::Range<usize>,
    pub original: usize,
}

pub(crate) fn append_mapped(
    body: &mut String,
    copies: &mut Vec<CopiedSource>,
    text: (String, Vec<CopiedSource>),
) {
    let start = body.len();
    body.push_str(&text.0);
    copies.extend(text.1.into_iter().map(|mut copy| {
        copy.generated.start += start;
        copy.generated.end += start;
        copy
    }));
}

pub(crate) struct EvaluationLookup {
    pub file: String,
    pub run: String,
    pub prefix: usize,
    pub copies: Vec<CopiedSource>,
    pub calls: Vec<(Span, StyleOrigin)>,
}

impl EvaluationLookup {
    pub(crate) fn produced_by(&self, context: &Context) -> Option<RealLocation> {
        let frames: Vec<_> = context
            .stack_trace()
            .map(boa_engine::vm::CallFrame::position)
            .collect();
        if frames
            .iter()
            .any(|frame| matches!(frame.path, SourcePath::Eval))
        {
            return None;
        }
        for frame in frames.into_iter().rev() {
            let SourcePath::Path(path) = frame.path else {
                continue;
            };
            if path.as_ref() != Path::new(&self.file) {
                continue;
            }
            let Some(offset) = copied_body_offset(
                &self.run,
                frame
                    .position
                    .map(|position| (position.line_number(), position.column_number())),
                self.prefix,
            ) else {
                continue;
            };
            let Some(copy) = self
                .copies
                .iter()
                .find(|copy| copy.generated.contains(&offset))
            else {
                continue;
            };
            let original = copy.original + offset - copy.generated.start;
            let Some((_, origin)) = self
                .calls
                .iter()
                .filter(|(span, _)| {
                    usize::try_from(span.start).is_ok_and(|start| start <= original)
                        && usize::try_from(span.end).is_ok_and(|end| original < end)
                })
                .max_by_key(|(span, _)| span.size())
            else {
                continue;
            };
            return Some(RealLocation::ProducedByCall(origin.clone()));
        }
        None
    }
}

fn copied_body_offset(
    source: &str,
    coordinate: Option<(u32, u32)>,
    prefix: usize,
) -> Option<usize> {
    let (line, column) = coordinate?;
    let row = usize::try_from(line.checked_sub(1)?).ok()?;
    let mut start = 0;
    let text = source
        .split_inclusive('\n')
        .enumerate()
        .find_map(|(index, text)| {
            if index == row {
                Some(text)
            } else {
                start += text.len();
                None
            }
        })?;
    let mut units = column.checked_sub(1)?;
    for character in text.chars() {
        if units == 0 {
            return start.checked_sub(prefix);
        }
        units = units.checked_sub(u32::try_from(character.len_utf16()).ok()?)?;
        start += character.len_utf8();
    }
    (units == 0).then_some(start)?.checked_sub(prefix)
}
