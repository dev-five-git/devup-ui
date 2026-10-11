use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

use crate::ordinary_ve::{edits::Replacement, execution::SelectedModule, selection};
use crate::vanilla_extract::Stylesheet;

#[rstest]
#[case(
    false,
    "native binding `style` escapes preserved source. Fix: use the API only in exact initializers and retain computed values"
)]
#[case(
    true,
    "native `style` call cannot remain in preserved source. Fix: report this extraction error with the original module"
)]
fn rejects_uncovered_native_source_at_its_authored_location(
    #[case] covers_binding: bool,
    #[case] diagnostic: &str,
) {
    let source = "import {style} from '@vanilla-extract/css';\nconst box=style({ padding:8 });";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selected = selection::select(&parsed.program, &built.semantic);
    let replacements = if covers_binding {
        vec![Replacement {
            span: selected.native_calls[0].callee,
            text: "'callee'".into(),
        }]
    } else {
        vec![]
    };
    let module = SelectedModule {
        stylesheet: Stylesheet {
            filename: "/preserved.ts",
            code: source,
            source,
            edits: &[],
        },
        selection: &selected,
    };
    let error = match super::check(module, &built.semantic, &replacements) {
        Ok(()) => panic!("uncovered native source passed preservation check"),
        Err(error) => error,
    };
    assert_eq!(error, format!("/preserved.ts:2:11: {diagnostic}"));
}
