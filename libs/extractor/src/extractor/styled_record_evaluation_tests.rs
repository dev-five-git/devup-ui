use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::*;
use crate::utils::readable_code;

#[test]
fn repair_745_empty_captures_keep_the_callers_arguments() {
    // Given: an expression whose arguments belong to its enclosing function.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let body = Parser::new(&allocator, "arguments[0]", SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));

    // When: bind no captures, then execute the emitted expression in its authored scope.
    let render = bind_render(&builder, Vec::new(), body);
    let script = format!("(function(){{return {};}})(7)", readable_code(&render));
    let result = Context::default()
        .eval(Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}"));

    // Then: no zero-argument wrapper steals the caller's arguments object.
    assert_eq!(
        result.as_number().map(f64::to_bits),
        Some(7.0_f64.to_bits())
    );
}

#[rstest]
#[case::equal_true(true, true)]
#[case::equal_false(false, false)]
#[case::different_true(true, false)]
#[case::different_false(false, true)]
fn boolean_choice_when_test_is_captured(
    #[case] yes: bool,
    #[case] no: bool,
    #[values(false, true)] on: bool,
) {
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = "(trace.push('test'), on);";
    let parsed = Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("test fixture must be an expression");
    };
    let mut reads = Evaluations::new(&builder, source.to_string());
    let test = reads.capture(&statement.expression, &reads.boolean(true));
    let choice = reads.choose(&test, &reads.boolean(yes), &reads.boolean(no));
    match &choice {
        Expression::BooleanLiteral(value) => {
            assert_eq!(yes, no);
            assert_eq!(value.value, yes);
        }
        Expression::ConditionalExpression(_) => assert_ne!(yes, no),
        _ => panic!("Boolean choice must be constant or conditional"),
    }
    let render = bind_render(&builder, reads.bindings, choice);
    let script = format!(
        "const trace = []; const on = {on}; const result = {}; JSON.stringify([result, trace]);",
        readable_code(&render)
    );
    let mut context = Context::default();
    let result = match context.eval(Source::from_bytes(script.as_bytes())) {
        Ok(result) => result,
        Err(error) => panic!("Boolean choice evaluation failed: {error}"),
    };
    let result = match result.as_string() {
        Some(result) => result.to_std_string_escaped(),
        None => panic!("Boolean choice must return a JSON string"),
    };
    assert_eq!(
        result,
        format!(r#"[{},["test"]]"#, if on { yes } else { no })
    );
}
