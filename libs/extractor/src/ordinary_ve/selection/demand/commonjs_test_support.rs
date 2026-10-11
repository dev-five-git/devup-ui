use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{SourceType, Span};

use super::super::{Demand, select_reads};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct Observed {
    pub units: Vec<(Span, Demand)>,
    pub omitted: Vec<Span>,
    pub forwarded: Vec<(String, Demand, Span)>,
    pub errors: Vec<(Span, String)>,
}

pub(super) fn selected(source: &str, demand: &Demand) -> TestResult<Observed> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    let view = select_reads(
        (&parsed.program, &built.semantic),
        (demand, &[]),
        ("/commonjs-contract.ts", "@vanilla-extract/css", None),
    );
    let units = view
        .selection
        .units
        .iter()
        .map(|unit| {
            view.units
                .get(&unit.node)
                .cloned()
                .map(|demand| (unit.span, demand))
                .ok_or_else(|| "selected unit has no demand".into())
        })
        .collect::<TestResult<Vec<_>>>()?;
    let mut omitted: Vec<_> = view
        .properties
        .values()
        .flat_map(|properties| properties.omitted.iter().copied())
        .collect();
    omitted.sort_by_key(|span| span.start);
    Ok(Observed {
        units,
        omitted,
        forwarded: view.forwarded,
        errors: view.selection.provenance_errors,
    })
}

pub(super) fn path(keys: &[&str]) -> Demand {
    keys.iter()
        .rev()
        .fold(Demand::whole(), |child, key| Demand::prefixed(key, &child))
}

pub(super) fn span(source: &str, text: &str) -> TestResult<Span> {
    let start = source.find(text).ok_or("fixture span missing")?;
    let end = start
        .checked_add(text.len())
        .ok_or("fixture span overflow")?;
    Ok(Span::new(u32::try_from(start)?, u32::try_from(end)?))
}
