use super::may_hold;
use crate::coverage_tests::expression;
use oxc_allocator::Allocator;
use rstest::rstest;

#[rstest]
#[case("({title:'text'})", false)]
#[case("({style:{color:'red'}})", true)]
#[case("({['title']:'text'})", true)]
#[case("({...rest})", true)]
fn literal_spread_keys_control_possible_inline_overrides(
    #[case] source: &str,
    #[case] expected: bool,
) {
    // Given
    let allocator = Allocator::default();
    let spread = expression(&allocator, source);
    // When
    let actual = may_hold(&spread, "style");
    // Then
    assert_eq!(actual, expected);
}

#[test]
fn unknown_property_keys_survive_object_prop_modification() {
    // Given
    let allocator = Allocator::default();
    let ast = oxc_ast::builder::AstBuilder::new(&allocator);
    let mut source = expression(&allocator, "({[state.key]:state.value})");
    let expected = crate::coverage_tests::code(&source);
    // When
    let oxc_ast::ast::Expression::ObjectExpression(object) = &mut source else {
        panic!("object fixture")
    };
    let _result = super::modify_prop_object(
        &ast,
        &mut object.properties,
        &mut [],
        None,
        None,
        None,
        None,
        None,
    );
    // Then
    assert_eq!(crate::coverage_tests::code(&source), expected);
}
