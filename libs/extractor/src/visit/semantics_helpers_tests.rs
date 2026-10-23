use super::*;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

#[rstest]
#[case("false ?? 'external'", None)]
#[case("false || 'external'", Some("external"))]
#[case("null ?? 'external'", Some("external"))]
fn composition_when_an_empty_left_value_has_different_logical_meanings(
    #[case] source: &str,
    #[case] expected: Option<&str>,
) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    let visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
    let mut parts = Vec::new();
    assert_eq!(
        visitor.known_parts(&statement.expression, &mut parts, Text::Classes),
        Some(())
    );
    let classes: Vec<_> = parts
        .iter()
        .filter_map(|part| match part {
            KnownPart::Class(Expression::StringLiteral(value)) => Some(value.value.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(classes, expected.into_iter().collect::<Vec<_>>());
}

#[rstest]
#[case("({color:'red',id:'keep',[key]:'computed',...rest})", true)]
#[case("({...{color:'red',id:'keep'}})", false)]
#[case("({get color(){return 'red'}})", true)]
fn spread_shape_when_classified_uses_the_whole_object(#[case] source: &str, #[case] unknown: bool) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression required")
    };
    assert_eq!(
        element_props::is_unknown_spread(&statement.expression),
        unknown
    );
}
