use super::preserves_reads;
use crate::assignment_test_support::evaluate;
use crate::coverage_tests::{code, expression};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use rstest::rstest;

#[rstest]
#[case("('body')", true)]
#[case("undefined", true)]
#[case("state.value", false)]
#[case("state.choose ? 'body' : undefined", true)]
#[case("state.choose ? (state.inner ? 'body' : undefined) : 'heading'", true)]
#[case("state.choose ? 'body' : state.value", false)]
#[case("state.choose ? read() : 'body'", false)]
#[case("({a:'body',b:'heading'})[state.key]", true)]
#[case("({a:read()})[state.key]", false)]
#[case("read()", false)]
fn residual_constant_projection_proves_inert_value_production(
    #[case] source: &str,
    #[case] expected: bool,
) {
    // Given: normative private AST predicate controls, not fabricated production IR.
    // The constant-class capture wrapper retains the complete source evaluation.
    let allocator = Allocator::default();
    let source = expression(&allocator, source);
    let class = expression(&allocator, "('typo-body')");
    // When
    let actual = preserves_reads(&source, &class);
    // Then
    assert_eq!(actual, expected);
}

#[rstest]
#[case(
    "state.choose ? 'body' : 'heading'",
    "state.choose ? 'typo-body' : 'typo-heading'",
    true
)]
#[case(
    "state.choose ? 'body' : 'heading'",
    "other.choose ? 'typo-body' : 'typo-heading'",
    false
)]
#[case(
    "state.outer ? (state.inner ? 'body' : 'body') : 'heading'",
    "state.outer ? 'typo-body' : 'typo-heading'",
    false
)]
#[case(
    "({a:'body',b:'heading'})[state.key]",
    "({a:'typo-body',b:'typo-heading'})[state.key] || ''",
    true
)]
#[case(
    "({a:'body',b:'heading'})[state.key]",
    "({a:'typo-body',b:'typo-heading'})[other.key]",
    false
)]
fn residual_structured_projection_requires_each_authored_controller_read(
    #[case] source: &str,
    #[case] class: &str,
    #[case] expected: bool,
) {
    // Given: valid parsed selections with an independently specified retained-read contract.
    let allocator = Allocator::default();
    let source = expression(&allocator, source);
    let class = expression(&allocator, class);
    // When
    let actual = preserves_reads(&source, &class);
    // Then
    assert_eq!(actual, expected);
}

#[rstest]
#[case(true, "[\"outer\",\"inner\"]")]
#[case(false, "[\"outer\",\"key\"]")]
fn residual_constant_projection_wrapper_retains_lazy_nested_reads(
    #[case] selected: bool,
    #[case] expected_trace: &str,
) {
    // Given: the existing whole-source-once wrapper used for constant scalar classes.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(
        &allocator,
        "state.outer ? (state.inner ? 'body' : undefined) : ({a:'body'})[state.key]",
    );
    let class = expression(&allocator, "('typo-body')");
    assert!(preserves_reads(&source, &class));
    // When
    let wrapped =
        crate::utils::call_with_values(&ast, vec![("__devupTypography".into(), source)], class);
    let actual = evaluate(&format!(
        "let trace=[];const state={{get outer(){{trace.push('outer');return {selected}}},get inner(){{trace.push('inner');return false}},get key(){{trace.push('key');return 'a'}}}};const value={};JSON.stringify([value,trace]);",
        code(&wrapped),
    ));
    // Then: independent branch-specific trace, exactly once, with no unselected read.
    assert_eq!(actual, format!("[\"typo-body\",{expected_trace}]"));
}
