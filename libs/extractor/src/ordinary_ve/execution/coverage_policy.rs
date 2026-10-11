use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

use super::{Stylesheet, policy};

#[rstest]
#[case("declare const padding:number;", "style({padding})", "/erased.ts:2:15")]
#[case(
    "declare function padding():number;",
    "style({padding:padding()})",
    "/erased.ts:3:26"
)]
fn erased_native_input_reports_its_declaration_or_helper_call(
    #[case] declaration: &str,
    #[case] expression: &str,
    #[case] place: &str,
) {
    let code = format!(
        "import {{style}} from '@vanilla-extract/css';\n{declaration}\nconst box={expression};"
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let stylesheet = Stylesheet {
        filename: "/erased.ts",
        code: &code,
        source: &code,
        edits: &[],
    };

    let result = policy::check(stylesheet, &selection);

    assert_eq!(
        result,
        Err(format!(
            "{place}: a required native input is erased by TypeScript. Fix: supply an initialized runtime value"
        ))
    );
}

#[rstest]
#[case("const alias=module.exports;", None)]
#[case("const alias=exports;", None)]
#[case("const alias=module.exports;\nalias.space='12px';", Some(("module.exports", "/commonjs.ts:2:1")))]
#[case("const alias=exports;\nalias.space='12px';", Some(("exports", "/commonjs.ts:2:1")))]
#[case("let alias=exports;", Some(("exports", "/commonjs.ts:1:11")))]
#[case("const box={ value:module.exports };", Some(("module.exports", "/commonjs.ts:1:19")))]
#[case("module.exports.copy=exports;", Some(("module.exports", "/commonjs.ts:1:1")))]
fn commonjs_observations_distinguish_export_readonly_mutable_and_other_escapes(
    #[case] code: &str,
    #[case] expected: Option<(&str, &str)>,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let selection = crate::ordinary_ve::selection::select(&parsed.program, &built.semantic);
    let stylesheet = Stylesheet {
        filename: "/commonjs.ts",
        code,
        source: code,
        edits: &[],
    };

    let checks =
        policy::commonjs_observations(stylesheet, &selection, (&parsed.program, &built.semantic));

    let actual: Vec<_> = checks
        .iter()
        .map(|check| {
            (
                check.read.as_str(),
                check.path.clone(),
                check.place.as_str(),
            )
        })
        .collect();
    let expected: Vec<_> = expected
        .into_iter()
        .map(|(read, place)| (read, vec![], place))
        .collect();
    assert_eq!(actual, expected);
}
