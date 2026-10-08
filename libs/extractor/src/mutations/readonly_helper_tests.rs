use super::*;
use rstest::rstest;

#[rstest]
#[case(
    "const read=value=>value['child']['p'];read({ child: { p: 1 } });",
    true
)]
#[case("const read=value=>value[key];read({ p: 1 });", false)]
#[case("const read=value=>value?.child.p;read({ child: { p: 1 } });", false)]
#[case("const read=value=>({ p: 1 }).p;read({ p: 1 });", false)]
#[case("const read=value=>value.p;read(...[{ p: 1 }]);", false)]
#[case("const read=value=>value.p;read();", false)]
#[case("const read=(...value)=>1;read({ p: 1 });", false)]
#[case("const read=value=>-value.p;read({ p: 1 });", true)]
#[case("const read=value=>+value.p;read({ p: 1 });", true)]
#[case("const read=value=>!value.p;read({ p: 1 });", true)]
#[case("const read=value=>~value.p;read({ p: 1 });", true)]
#[case("const read=value=>typeof value.p;read({ p: 1 });", true)]
#[case("const read=value=>void value.p;read({ p: 1 });", true)]
#[case("const read=value=>-value.child;read({ child: { p: 1 } });", false)]
#[case("const read=value=>delete value.p;read({ p: 1 });", false)]
#[case("const read=value=>value['child'];read({ child: { p: 1 } });", false)]
fn readonly_summary_when_source_grammar_changes_preserves_scalar_only_contract(
    #[case] source: &str,
    #[case] expected: bool,
) {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = oxc_semantic::SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program)
        .semantic;
    let proof = Proof {
        nodes: semantic.nodes(),
        scoping: semantic.scoping(),
    };
    let oxc_ast::ast::Statement::ExpressionStatement(statement) = parsed
        .program
        .body
        .last()
        .unwrap_or_else(|| panic!("fixture statement"))
    else {
        panic!("call fixture")
    };
    let Expression::CallExpression(call) = &statement.expression else {
        panic!("call fixture")
    };
    // When
    let readonly = reads_arguments(&proof, call);
    // Then
    assert_eq!(readonly, expected, "{source}");
}
