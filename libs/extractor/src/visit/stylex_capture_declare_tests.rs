use super::*;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case::statement("a = f(), <A />;", "const a = f();\n<A />;\n")]
#[case::return_value(
    "return a = f(), b = h(), <A />;",
    "const a = f();\nreturn b = h(), <A />;\n"
)]
#[case::kept_rest("a = f(), g(), <A />;", "const a = f();\ng(), <A />;\n")]
#[case::other_first("g(), a = f(), <A />;", "let a;\ng(), a = f(), <A />;\n")]
#[case::other_target("x = f(), <A />;", "let a;\nx = f(), <A />;\n")]
#[case::compound("a += f(), <A />;", "let a;\na += f(), <A />;\n")]
#[case::member_target("o.a = f(), <A />;", "let a;\no.a = f(), <A />;\n")]
#[case::plain(
    "if (c) a = f(), <A />;\n<A />;\nreturn;",
    "let a;\nif (c) a = f(), <A />;\n<A />;\nreturn;\n"
)]
fn generated_leading_assignments_become_constants(#[case] source: &str, #[case] expected: &str) {
    // Given a statement list whose generated name is `a`
    let arena = Allocator::default();
    let mut program = Parser::new(&arena, source, SourceType::tsx().with_module(false))
        .with_options(oxc_parser::ParseOptions {
            allow_return_outside_function: true,
            ..oxc_parser::ParseOptions::default()
        })
        .parse()
        .program;
    let ast = AstBuilder::new(&arena);
    // When
    declare(&ast, &mut program.body, vec!["a".to_string()]);
    // Then
    assert_eq!(Codegen::new().build(&program).code, expected);
}

#[test]
fn every_generated_leading_assignment_becomes_a_constant() {
    // Given a return, the nearest statement list, assigning both names first
    let arena = Allocator::default();
    let mut program = Parser::new(
        &arena,
        "return a = f(), b = h(), <A />;",
        SourceType::tsx().with_module(false),
    )
    .with_options(oxc_parser::ParseOptions {
        allow_return_outside_function: true,
        ..oxc_parser::ParseOptions::default()
    })
    .parse()
    .program;
    let ast = AstBuilder::new(&arena);
    // When
    declare(
        &ast,
        &mut program.body,
        vec!["a".to_string(), "b".to_string()],
    );
    // Then
    assert_eq!(
        Codegen::new().build(&program).code,
        "const a = f(), b = h();\nreturn <A />;\n"
    );
}
