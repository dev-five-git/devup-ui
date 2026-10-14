use super::{As, rename, resolve};
use crate::coverage_tests::expression;
use oxc_allocator::Allocator;
use oxc_ast::{ast::Expression, builder::AstBuilder};
use rstest::rstest;

#[rstest]
#[case("motion.button", "motion.button")]
#[case("motion.controls.button", "motion.controls.button")]
#[case("(motion.controls).button", "motion.controls.button")]
fn residual_member_name_retains_unresolved_receiver_chain(
    #[case] source: &str,
    #[case] expected: &str,
) {
    // Given: no declared object exists for the constant evaluator to fold.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let Expression::JSXElement(mut element) = expression(&allocator, "<div></div>") else {
        panic!("JSX fixture")
    };
    let value = expression(&allocator, source);
    // When
    let resolved = resolve(&ast, &element, value, "div");
    // Then: observe both JSX names, not just successful resolution.
    let As::Name(name) = resolved else {
        panic!("static member must remain a JSX binding")
    };
    rename(&ast, &mut element, name);
    assert_eq!(element.opening_element.name.to_string(), expected);
    let Some(closing) = &element.closing_element else {
        panic!("paired JSX fixture")
    };
    assert_eq!(closing.name.to_string(), expected);
}
