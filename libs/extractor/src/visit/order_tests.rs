use super::order::{Reach, reach};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("input", Reach::Reads)]
#[case("mark(input)", Reach::Runs)]
#[case("3", Reach::Constant)]
fn generated_length_guard_preserves_argument_reach(#[case] source: &str, #[case] expected: Reach) {
    let arena = Allocator::default();
    let ast = oxc_ast::builder::AstBuilder::new(&arena);
    let input = Parser::new(&arena, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let generated = crate::imported_constants::local_rule_shapes::number_length(&ast, input);
    assert!(reach(&crate::scope::Bindings::default(), &generated) == expected);
}

#[test]
fn authored_length_guard_remains_a_user_call() {
    let arena = Allocator::default();
    let source = "((__devupValue)=>typeof __devupValue==='number'&&__devupValue!==0?__devupValue+'px':__devupValue)(input)";
    let value = Parser::new(&arena, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert!(reach(&crate::scope::Bindings::default(), &value) == Reach::Runs);
}

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
#[case("`${{toString(){throw 0;}}}`", Reach::Runs)]
#[case("`${[{}]}`", Reach::Runs)]
#[case("`${3}`", Reach::Constant)]
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

#[test]
fn compiled_call_when_its_arguments_are_spread_can_run_iterator_code() {
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        "import {css} from '@devup-ui/react'; css(...parts);",
        SourceType::ts(),
    )
    .parse();
    let mut bindings = crate::scope::Bindings::default();
    let scoping = SemanticBuilder::new()
        .build(&parsed.program)
        .semantic
        .into_scoping();
    let symbol = scoping.get_root_binding("css".into());
    bindings.scope(std::rc::Rc::new(scoping));
    assert!(bindings.export(symbol, "css"));
    let Statement::ExpressionStatement(statement) = &parsed.program.body[1] else {
        panic!("call required")
    };
    assert!(reach(&bindings, &statement.expression) == Reach::Runs);
}
