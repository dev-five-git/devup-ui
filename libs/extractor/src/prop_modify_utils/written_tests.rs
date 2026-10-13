use super::*;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("({className:'blue'})", "", "blue")]
#[case("({className:'blue',id:'x'})", "", "blue")]
#[case("({id:'x'})", "", "base")]
#[case(
    "({className:'red',...rest})",
    "const rest={className:'blue'};",
    "blue"
)]
#[case("({className:'red',...rest})", "const rest={};", "red")]
#[case("({...rest})", "const rest={className:'blue'};", "blue")]
#[case("({...rest})", "const rest={};", "base")]
#[case("({[key]:'blue'})", "const key='className';", "blue")]
#[case("({[key]:'blue'})", "const key='id';", "base")]
#[case("({className:undefined})", "", "undefined")]
fn written_class_when_a_literal_spread_follows_an_explicit_value_obeys_assignment(
    #[case] source: &str,
    #[case] setup: &str,
    #[case] expected: &str,
) {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    let written = [
        Written::Prop(Expression::new_string_literal(SPAN, "base", None, &ast)),
        Written::Spread(statement.expression.clone_in(ast.allocator())),
    ];
    let expression = last_written(&ast, &written, "className")
        .unwrap_or_else(|| panic!("the explicit class is present"));
    let script = format!(
        "(()=>{{{setup}return ({});}})()",
        crate::utils::readable_code(&expression)
    );
    let mut context = boa_engine::Context::default();
    let actual = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    assert_eq!(
        actual
            .to_string(&mut context)
            .unwrap_or_else(|error| panic!("{error}"))
            .to_std_string_escaped(),
        expected
    );
}
