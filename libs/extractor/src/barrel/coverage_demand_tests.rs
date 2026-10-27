use std::error::Error;

use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPattern, Statement};
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

use super::super::demand::Demand;

#[rstest]
#[case("const A=B; const B=A;", false)]
#[case("const A=()=>0; type T=typeof A;", true)]
#[case("let A; A=0;", false)]
#[case("const A=()=>0; const B=A; const view=<B/>;", true)]
#[case("const A=()=>0; consume(A);", false)]
fn render_only_when_semantic_references_cycle_are_type_only_or_write(
    #[case] code: &str,
    #[case] expected: bool,
) -> Result<(), Box<dyn Error>> {
    // Given: real TypeScript/JSX reference flags and an alias graph for A.
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.errors().count(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.errors().count(), 0);
    let Statement::VariableDeclaration(declaration) = &parsed.program.body[0] else {
        return Err("variable declaration node".into());
    };
    let BindingPattern::BindingIdentifier(binding) = &declaration.declarations[0].id else {
        return Err("A binding node".into());
    };
    let demand = Demand::new(&built.semantic, &parsed.program);
    // When: the same helper used for opaque render demand follows A's references.
    let result = demand.render_only(binding);
    // Then: only type-only and actual render demand qualify, never cycles or writes.
    assert_eq!(result, expected);
    Ok(())
}
