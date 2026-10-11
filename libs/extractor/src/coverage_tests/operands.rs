use super::{code, dynamic, expression};
use crate::{
    ExtractStyleProp, ExtractStyleValue,
    assignment_test_support::{compiled_jsx, evaluate},
};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("motion.button", "button")]
#[case("motion.controls.button", "button")]
#[serial]
fn static_member_types_keep_the_receiver_chain(#[case] tag: &str, #[case] expected: &str) {
    // Given
    let source = format!(
        "import {{Box}}from '@devup-ui/react';const motion={{button:'button',controls:{{button:'button'}}}};const node=<Box as={{{tag}}} p={{1}}/>;JSON.stringify(node.tag);"
    );
    // When
    let actual = evaluate(&compiled_jsx(&source));
    // Then
    assert_eq!(actual, format!("\"{expected}\""));
}

#[test]
#[serial]
fn array_shape_cannot_collapse_to_one_scalar_assignment() {
    // Given
    let allocator = Allocator::default();
    let source = expression(&allocator, "[state.value]");
    let styles = [ExtractStyleProp::Static(dynamic("color", "state.value"))];
    // When
    let scalar = crate::assignment_owner::scalar(&source, &styles);
    // Then: responsive arrays require the structured consumer, not a scalar variable.
    assert_eq!(scalar, None);
}

#[test]
fn object_spread_is_not_a_literal_evaluation_proof() {
    // Given
    let allocator = Allocator::default();
    let source = expression(&allocator, "({...{color:'red'}})");
    // When
    let literal = crate::static_assignment::literal_source(&source);
    // Then
    assert!(!literal);
}

#[test]
fn dynamic_records_require_inline_assignment_even_inside_class_containers() {
    // Given
    let style =
        ExtractStyleProp::StaticArray(vec![ExtractStyleProp::Static(dynamic("color", "input"))]);
    // When
    let class_only = crate::static_assignment::class_only(&style);
    // Then
    assert!(!class_only);
}

#[rstest]
#[case("state", true)]
#[case("read()", false)]
fn private_member_purity_follows_the_receiver(#[case] receiver: &str, #[case] expected: bool) {
    use oxc_ast_visit::Visit;

    struct PrivateReads(Vec<bool>);
    impl<'a> oxc_ast_visit::Visit<'a> for PrivateReads {
        fn visit_expression(&mut self, expression: &oxc_ast::ast::Expression<'a>) {
            if matches!(
                expression,
                oxc_ast::ast::Expression::PrivateFieldExpression(_)
            ) {
                self.0.push(crate::utils::is_pure(expression));
            }
            oxc_ast_visit::walk::walk_expression(self, expression);
        }
    }
    // Given
    let allocator = Allocator::default();
    let source = format!("class Receiver{{#value;method(state,read){{return {receiver}.#value}}}}");
    let parsed = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut reads = PrivateReads(vec![]);
    // When
    reads.visit_program(&parsed.program);
    // Then
    assert_eq!(reads.0, vec![expected]);
}

#[test]
#[serial]
fn static_selector_identity_distinguishes_layered_declarations() {
    // Given
    let plain = crate::extract_style::extract_static_style::ExtractStaticStyle::new(
        "color", "red", 0, None,
    );
    let mut layered = plain.clone();
    layered.layer = Some("base".into());
    // When
    let identity = (
        plain.class_selector().map(std::borrow::Cow::into_owned),
        layered.class_selector().map(std::borrow::Cow::into_owned),
    );
    // Then
    assert_eq!(identity, (None, Some("@layer base".into())));
}

#[rstest]
#[case("({color:state.color})", "[\"red\",[\"color\"]]")]
#[case("[state.color]", "[\"red\",[\"color\"]]")]
#[serial]
fn lowered_container_keeps_raw_values_when_special_records_are_not_consumers(
    #[case] source: &str,
    #[case] expected: &str,
) {
    // Given: typed IR with a dynamic consumer and an unrelated special record.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, source);
    let mut styles = [
        ExtractStyleProp::Static(dynamic("color", "state.color")),
        ExtractStyleProp::Static(ExtractStyleValue::Typography("heading".into())),
    ];
    let lowering = crate::assignment_lowering::Lowering {
        ast: &ast,
        order: None,
        filename: None,
        alternate_order: None,
    };
    // When
    let generated = lowering.lower(&source, &mut styles);
    let actual = evaluate(&format!(
        "let trace=[];const state={{get color(){{trace.push('color');return 'red'}}}};const result={};JSON.stringify([Array.isArray(result[2])?result[2][0]:result[2].color,trace]);",
        code(&generated)
    ));
    // Then
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn missing_member_consumers_preserve_all_literal_field_reads_before_selection() {
    // Given
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "({a:state.first,b:state.second})[state.key]");
    let lowering = crate::assignment_lowering::Lowering {
        ast: &ast,
        order: None,
        filename: None,
        alternate_order: None,
    };
    // When
    let generated = lowering.lower(&source, &mut []);
    let actual = evaluate(&format!(
        "let trace=[];const state={{get first(){{trace.push('first');return 'red'}},get second(){{trace.push('second');return 'blue'}},get key(){{trace.push('key');return 'b'}}}};const result={};JSON.stringify([trace,result[2]]);",
        code(&generated)
    ));
    // Then
    assert_eq!(actual, "[[\"first\",\"second\",\"key\"],\"blue\"]");
}
