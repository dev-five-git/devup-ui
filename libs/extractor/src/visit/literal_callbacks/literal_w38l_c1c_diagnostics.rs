use super::literal_w38l_c1c_source::produced;
use crate::css_prop::{NESTED_MIXIN, UNPLACED, template_parts};
use crate::utils::readable_code;
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_span::GetSpan;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("&:hover{${base};}", NESTED_MIXIN)]
#[case(concat!("$", "{", "base", "}", ":hover", "{", "color:red", "}"), UNPLACED)]
#[serial]
fn styled_supplier_when_default_rejects_retains_exact_hole_and_requirement(
    #[case] body: &str,
    #[case] expected: &str,
) {
    // Given: real external holes have no producer provenance.
    let source = format!("import {{css}} from '@devup-ui/react';unknown`{body}`;");
    let allocator = Allocator::default();
    let (visitor, mut expression) = produced(&allocator, &source);
    let Expression::TaggedTemplateExpression(tag) = &expression else {
        panic!("retained source template")
    };
    let original = (expression.span(), readable_code(&expression));
    let hole = &tag.quasi.expressions[0];
    assert_eq!(visitor.style_values.finite(hole), None);
    let expected_hole = (hole.span(), readable_code(hole));
    let Err((rejected, requirement)) = template_parts(&visitor.ast, &tag.quasi, true) else {
        panic!("expected default rejection")
    };
    // When: the existing styled preparer encounters this default rejection.
    visitor.prepare_styled_template(&mut expression);
    // Then: it keeps the source AST, rejected span and precise diagnostic contract.
    assert_eq!(requirement, expected);
    assert_eq!((rejected.span(), readable_code(&rejected)), expected_hole);
    assert_eq!((expression.span(), readable_code(&expression)), original);
    assert_eq!(visitor.errors, vec![]);
}
