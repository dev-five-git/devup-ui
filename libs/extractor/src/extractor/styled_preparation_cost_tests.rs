use super::*;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[rstest::rstest]
#[case("({p:4,_hover:{color:'blue'},m:[1,null,2]})")]
#[case("on?{ p: 4 }:{ p: 2 }")]
#[case("'external'")]
fn original_ast_when_no_property_callback_exists_uses_no_arena_storage(#[case] source: &str) {
    // Given
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let mut style = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let original = readable_code(&style);
    let mut evaluations = Evaluations::new(&builder, original.clone());
    let before = allocator.used_bytes();
    // When
    let plan =
        lower(&mut evaluations, &mut style, None).unwrap_or_else(|error| panic!("{error:?}"));
    // Then
    assert!(plan.is_none());
    assert_eq!(allocator.used_bytes(), before);
    assert_eq!(readable_code(&style), original);
    assert_eq!(evaluations.bindings.len(), 0);
}

#[test]
fn composition_admission_when_parts_are_written_uses_no_arena_storage() {
    // Given
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let expression = Parser::new(
        &allocator,
        "parts([{p:4,_hover:{ color: 'blue' }},on?{ p: 2 }:null],false)",
        SourceType::tsx(),
    )
    .parse_expression()
    .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::CallExpression(call) = &expression else {
        panic!("call fixture required")
    };
    let before = allocator.used_bytes();
    // When
    let admitted = super::super::rule_parts::admitted(&builder, &call.arguments, None);
    // Then
    assert!(admitted);
    assert_eq!(allocator.used_bytes(), before);
}

#[rstest::rstest]
#[case("parts({ p: 4 })", true)]
#[case("parts(false,null,undefined)", true)]
#[case("parts(on?{ p: 4 }:null)", true)]
#[case("parts('opaque',`text`)", true)]
#[case("parts(external,{ p: 2 })", true)]
#[case("parts([{ p: 4 },...[null,{ p: 2 }]])", true)]
#[case("parts(...[{ p: 4 },on?{ p: 2 }:false])", true)]
#[case("parts(on&&{ p: 2 })", true)]
#[case("parts(on||{ p: 2 })", true)]
#[case("parts(on??{ p: 2 })", true)]
#[case("parts(unknown())", false)]
#[case("parts(...unknown)", false)]
#[case("parts([...unknown])", false)]
#[case("parts([,{ p: 2 }])", false)]
#[case("parts(on?{ p: 2 }:unknown())", false)]
#[serial_test::serial]
fn composition_admission_when_shape_varies_matches_extraction_without_rewriting_source(
    #[case] source: &str,
    #[case] expected: bool,
) {
    // Given
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let expression = Parser::new(&allocator, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::CallExpression(call) = &expression else {
        panic!("call fixture required")
    };
    let original = readable_code(&expression);
    // When
    let admitted = super::super::rule_parts::admitted(&builder, &call.arguments, None);
    // Then
    assert_eq!(admitted, expected);
    assert_eq!(
        admitted,
        super::super::rule_parts::compose_bound(&builder, &call.arguments, (None, "cost.tsx"))
            .is_some()
    );
    assert_eq!(readable_code(&expression), original);
}
