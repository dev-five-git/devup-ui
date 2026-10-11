use super::Dependencies;
use css::Naming;
use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_semantic::SemanticBuilder;
use rstest::rstest;

#[rstest]
#[case("require('module')", Naming::Risky)]
#[case("import('module')", Naming::Risky)]
#[case("require('module').value", Naming::Risky)]
#[case("({require:x=>x}).require('module')", Naming::Own)]
#[case("ordinary('module')", Naming::Own)]
fn raw_dependency_syntax_keeps_its_naming_distinction(
    #[case] source: &str,
    #[case] expected: Naming,
) {
    // Given: Dependencies classifies raw syntax, not resolver execution or shadow policy.
    let allocator = Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let namings = rustc_hash::FxHashMap::default();
    let mut dependencies = Dependencies {
        scoping: &scoping,
        namings: &namings,
        naming: Naming::Own,
    };
    // When
    dependencies.visit_program(&parsed.program);
    // Then
    assert_eq!(dependencies.naming, expected);
}
