use super::own_scalar;
use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("{ width:'13px',child:{ n:1 }}", Some("\"13px\""))]
#[case("{ width:12,width:13 }", Some("13"))]
#[case("{ width:13,width:{ n:1 }}", None)]
#[case("{ width:{ n:1 },width:13 }", Some("13"))]
#[case("{ width:true }", Some("true"))]
#[case("{ width:13,child:{ flag:true }}", Some("13"))]
#[case("{ width:null }", Some("null"))]
#[case("{ width:1e999 }", None)]
#[case("{ width:13,child:1e999 }", Some("13"))]
#[case("{ width:13,child:{ '':1e999 }}", Some("13"))]
#[case("{ child:{ n:1 }}", None)]
#[case("{ width:13,...{ child:1 }}", None)]
#[case("{ width:13,['child']:1 }", None)]
#[case("{ width:13,get child(){ return 1 }}", None)]
#[case("{ width:13,set child(x){} }", None)]
#[case("{ width:13,child(){} }", None)]
#[case("{ width:13,toJSON:1 }", None)]
#[case("{ width:13,toString:1 }", None)]
#[case("{ width:13,valueOf:1 }", None)]
#[case("{ width:13,__proto__:null }", None)]
#[case("{ width:13,child:{ ...{ n:1 }}}", None)]
#[case("{ width:13,child:[1] }", None)]
#[case("{ width:13,child:unknown }", None)]
fn literal_slot_when_grammar_and_last_own_property_determine_exactness(
    #[case] object: &str,
    #[case] expected: Option<&str>,
) {
    // Given
    let allocator = Allocator::default();
    let source = format!("({object});");
    let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("fixture expression")
    };
    let Expression::ObjectExpression(object) =
        crate::utils::unwrap_syntax_only(&statement.expression)
    else {
        panic!("fixture object")
    };
    // When
    let value = own_scalar(object, "width");
    // Then
    assert_eq!(
        value.and_then(|value| value.js_literal()).as_deref(),
        expected
    );
}
