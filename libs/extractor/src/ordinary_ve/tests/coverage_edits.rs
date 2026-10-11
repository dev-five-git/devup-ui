use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

use super::super::{
    edits::{Replacement, apply},
    selection,
};

#[test]
fn rejects_a_native_call_replacement_overlapping_its_owner() {
    let source =
        "import {style} from '@vanilla-extract/css';\nexport const box=style({ padding:8 });";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let plan = selection::select(&parsed.program, &built.semantic);
    let replacements = vec![
        Replacement {
            span: plan.native_calls[0].span,
            text: "'child'".into(),
        },
        Replacement {
            span: plan.roots[0].span,
            text: "box='owner'".into(),
        },
    ];
    let error = match apply(source, replacements) {
        Ok((code, _)) => panic!("overlapping replacements published {code}"),
        Err(error) => {
            crate::located_errors("/overlap.ts", source, &[], vec![(error.at, error.message)])
        }
    };
    assert_eq!(
        error,
        "/overlap.ts:2:18: native source replacements overlap. Fix: report this extraction error with the original module"
    );
}
