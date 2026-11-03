use super::*;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::operator::LogicalOperator;
use rstest::rstest;

#[rstest]
#[case("``", [true, false, true])]
#[case("`nonempty`", [false, true, true])]
#[case("null", [true, false, false])]
#[case("false", [true, false, true])]
#[case("0", [true, false, true])]
fn literal_selection_distinguishes_truthiness_from_nullishness(
    #[case] source: &str,
    #[case] left: [bool; 3],
) {
    let arena = Allocator::default();
    let visitor = DevupVisitor::new(&arena, "literal.tsx", "@devup-ui/react", vec![], None);
    let value = Parser::new(&arena, source, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    for (operator, expected) in [
        LogicalOperator::And,
        LogicalOperator::Or,
        LogicalOperator::Coalesce,
    ]
    .into_iter()
    .zip(left)
    {
        assert_eq!(
            literal_choice(&visitor.bindings, &value, operator),
            Some(expected),
            "{source}"
        );
    }
}

#[test]
fn non_scalar_cooked_template_tail_is_not_rewritten_as_utf8() {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "tail.tsx", "@devup-ui/react", vec![], None);
    let mut value = Parser::new(&arena, r"`${read()}\uD800`", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let Expression::TemplateLiteral(template) = &value else {
        panic!("expected template")
    };
    assert!(
        template.quasis[1]
            .value
            .cooked
            .as_ref()
            .is_some_and(|cooked| cooked.as_str().is_none())
    );
    let before = crate::utils::readable_code(&value);
    let mut captured = Vec::new();
    visitor.capture_leaf(&mut value, &mut captured);
    assert_eq!(captured.len(), 1);
    assert_eq!(crate::utils::readable_code(&captured[0].1), before);
    assert!(
        matches!(&value, Expression::Identifier(read) if read.name.as_str() == captured[0].0.as_str())
    );
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
}

#[test]
fn external_tag_keeps_its_own_interpolation_and_capture_boundary() {
    let arena = Allocator::default();
    let mut visitor = DevupVisitor::new(&arena, "tag.tsx", "@devup-ui/react", vec![], None);
    let mut value = Parser::new(&arena, "externalTag`${read()}`", SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let before = crate::utils::readable_code(&value);
    let mut captured = Vec::new();
    assert!(visitor.capture_css_boundary(&mut value, &mut captured));
    assert!(captured.is_empty());
    assert_eq!(crate::utils::readable_code(&value), before);
    assert_eq!(visitor.errors, [] as [(u32, String); 0]);
}
