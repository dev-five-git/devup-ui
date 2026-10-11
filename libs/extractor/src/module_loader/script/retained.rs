use std::rc::Rc;

use oxc_allocator::Allocator;
use oxc_ast::ast::{Declaration, Statement};
use oxc_parser::Parser;
use oxc_sourcemap::SourceMap;
use oxc_span::{GetSpan, SourceType};

use super::{Unit, Written};
use crate::source_map::{Trace, marks};
use crate::vanilla_extract::{Stylesheet, strip_typescript_marked};

fn initializers(code: &str) -> Vec<(String, usize, usize)> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
    parsed
        .program
        .body
        .iter()
        .flat_map(|statement| {
            let declaration = match statement {
                Statement::VariableDeclaration(declaration) => Some(declaration.as_ref()),
                Statement::ExportDeclaration(export) => match &export.declaration {
                    Declaration::VariableDeclaration(declaration) => Some(declaration.as_ref()),
                    _ => None,
                },
                _ => None,
            };
            declaration
                .into_iter()
                .flat_map(|declaration| &declaration.declarations)
                .filter_map(|declarator| {
                    let name = declarator.id.get_identifier_name()?;
                    let span = declarator.init.as_ref()?.span();
                    Some((name.to_string(), span.start as usize, span.end as usize))
                })
        })
        .collect()
}

impl Unit {
    pub(crate) fn retained(
        filename: &str,
        output: &crate::ExtractOutput,
        source: &str,
    ) -> Result<Rc<Self>, String> {
        let code = &output.code;
        if code == source {
            return Self::written(filename, code, source, &[]);
        }
        crate::module_loader::validate(Stylesheet {
            filename,
            code,
            source: code,
            edits: &[],
        })?;
        let generated = strip_typescript_marked(code, filename);
        let unavailable = || {
            format!(
                "{filename}:1:1: Cannot trace retained stylesheet code. Fix: report this internal source-map failure"
            )
        };
        let map = SourceMap::from_json_string(output.map.as_deref().ok_or_else(unavailable)?)
            .map_err(|error| format!("{filename}:1:1: Cannot read stylesheet source map: {error}. Fix: report this internal source-map failure"))?;
        let parsed = map.get_source_content(0).ok_or_else(unavailable)?;
        let originals = initializers(source);
        let copied: Vec<_> = initializers(parsed)
            .into_iter()
            .filter_map(|(name, start, end)| {
                let (_, from, _) = originals.iter().find(|(original_name, from, to)| {
                    *original_name == name && source[*from..*to] == parsed[start..end]
                })?;
                Some((start, end, *from))
            })
            .collect();
        let trace = Trace::new(&marks(&map, code, parsed), &[]);
        let mut mapped = Vec::new();
        let mut ranges = Vec::new();
        let mut tokens = generated.marks.iter().peekable();
        while let Some(&(start, at)) = tokens.next() {
            let parsed_at = trace.resolve(code, parsed, at);
            if let Some(&(from, _, original)) = copied
                .iter()
                .find(|(from, to, _)| (*from..*to).contains(&parsed_at))
            {
                let end = tokens
                    .peek()
                    .map_or(generated.code.len(), |(next, _)| *next);
                mapped.push((start, original + parsed_at - from));
                ranges.push((start, end));
            }
        }
        Ok(Rc::new(Self {
            filename: filename.to_string(),
            script: generated.code.clone(),
            written: Some(Written {
                source: source.to_string(),
                trace: Trace::new(&mapped, &[]),
                retained: Some(ranges),
            }),
        }))
    }
}

#[cfg(test)]
mod coverage_tests;
