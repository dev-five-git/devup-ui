use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::BTreeMap;

use super::super::{
    analysis::Index,
    compiled::{Accounting, Origin},
    retention,
};

#[test]
fn type_specifier_is_omitted_when_a_runtime_declaration_shares_its_symbol() -> Result<(), String> {
    // Given
    let source = "import {type Ghost} from './types';export const Ghost=Symbol('live');export const help=(value:Ghost)=>Ghost;";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let index = Index::new(&parsed.program, &built.semantic, source);
    let origin = Origin {
        filename: "/merged.css.ts",
        package: "@devup-ui/react",
        resolver: None,
    };
    let mut dispositions = FxHashMap::default();
    let mut retained = FxHashSet::default();
    let mut pieces = BTreeMap::new();
    for (name, export) in &index.exports {
        let symbol = export.target.ok_or("local runtime export")?;
        retention::binding(
            (symbol, name, &index),
            (&mut dispositions, &mut retained),
            &mut pieces,
        );
    }
    let accounting = Accounting::new(
        (&parsed.program, &built.semantic),
        &origin,
        &FxHashSet::default(),
    );
    // When
    super::retain(
        (&parsed.program, &index, &origin),
        &mut dispositions,
        (&mut pieces, &accounting),
    );
    // Then
    let output: Vec<_> = pieces.into_values().map(|piece| piece.text).collect();
    assert_eq!(
        output,
        vec![
            "export const Ghost=Symbol('live');",
            "export const help=(value:Ghost)=>Ghost;"
        ]
    );
    assert_eq!(retained.len(), 2);
    Ok(())
}
