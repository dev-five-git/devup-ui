use super::order::{Reach, reach};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("this", Reach::Reads)]
#[case("delete value.key", Reach::Runs)]
#[case("[...value]", Reach::Runs)]
#[case("<string>value", Reach::Reads)]
#[case("value?.key!", Reach::Runs)]
#[case("({[value]:other})", Reach::Runs)]
#[case("/pattern/", Reach::Constant)]
#[case("null", Reach::Constant)]
#[case("undefined", Reach::Constant)]
fn relocation_classification_when_syntax_has_observable_evaluation(
    #[case] source: &str,
    #[case] expected: Reach,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let mut bindings = crate::scope::Bindings::default();
    bindings.scope(std::rc::Rc::new(
        SemanticBuilder::new()
            .build(&parsed.program)
            .semantic
            .into_scoping(),
    ));
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    assert!(
        reach(&bindings, &statement.expression) == expected,
        "{source}"
    );
}
