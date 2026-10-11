use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

use super::{SelectedModule, Slots, Source};
use crate::vanilla_extract::Stylesheet;

#[test]
fn slots_when_consumer_read_overlaps_native_initializer_report_original_place() -> Result<(), String>
{
    // Given
    let code = "const 한글='😀';\r\nimport {style} from '@vanilla-extract/css';\r\nimport {css} from '@devup-ui/react';import {read} from './producer';\r\nconst native=style({color:css({color:read()})});";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let plan = crate::imported_constants::consumer::plan(
        &parsed.program,
        &built.semantic,
        &crate::ExtractOption::default(),
    );
    let unit = selection
        .units
        .iter()
        .find(|unit| unit.bindings.iter().any(|binding| binding.name == "native"))
        .ok_or("fixture native initializer was not selected")?;
    assert_eq!(
        plan.slots
            .iter()
            .map(|span| span.source_text(code))
            .collect::<Vec<_>>(),
        ["read()"]
    );
    assert!(unit.span.contains_inclusive(plan.slots[0]));
    let stylesheet = Stylesheet {
        filename: "/consumer-overlap.tsx",
        code,
        source: code,
        edits: &[],
    };
    let module = SelectedModule {
        stylesheet,
        selection: &selection,
    };
    let mut reserved = selection.reserved_names.clone();
    let observations = super::super::observe::Observations::new(&mut reserved);
    let mut source = Source {
        mapped: crate::module_loader::Mapped::default(),
        captures: Vec::new(),
        identities: Vec::new(),
        reserved,
        observations,
    };
    let mut slots = Slots::new(Some(&plan));
    let place = super::super::policy::place(stylesheet, unit.span.start);
    // When
    let result = slots.before(&mut source, (module, unit.span));
    // Then
    let error = match result {
        Ok(()) => panic!("overlap was accepted"),
        Err(error) => error,
    };
    assert!(error.contains(&place), "{error}");
    assert!(
        error.contains("consumer read overlaps selected initialization"),
        "{error}"
    );
    assert!(error.contains("Fix:"), "{error}");
    assert_eq!(source.captures.len(), 0);
    assert_eq!(source.identities.len(), 0);
    Ok(())
}
