use super::{assert_evaluated, generated};
use crate::{
    ExtractStyleProp, ExtractStyleValue,
    assignment_test_support::evaluate,
    coverage_tests::{code, expression},
    css_utils::css_to_style_template,
    extract_style::{numeric_conversion::NumericConversion, style_property::StyleProperty},
};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("4", "[1,true,[4]]")]
#[case("'4'", "[1,true,[\"4\"]]")]
#[case("'12px'", "[1,true,[\"12px\"]]")]
#[case("null", "[1,true,[null]]")]
#[case("false", "[1,true,[false]]")]
#[case("''", "[1,true,[\"\"]]")]
#[serial]
fn css_text_scalar_keeps_raw_inline_value_when_interpolation_is_captured(
    #[case] raw: &str,
    #[case] expected: &str,
) {
    // Given: the CSS parser, not a changed absence flag, produces the descriptor.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let text = "`padding: ${state.value}`";
    let _sites = crate::provenance::SiteScope::enter("scalar-seam.tsx", text, &[]);
    let source = expression(&allocator, text);
    let Expression::TemplateLiteral(template) = &source else {
        panic!("CSS template fixture")
    };
    let parsed = css_to_style_template(template, 0, &None);
    assert!(parsed.statements.is_empty() && parsed.unplaced.is_empty());
    let descriptors: Vec<_> = parsed
        .styles
        .into_iter()
        .map(|style| ExtractStyleProp::Static(style.into()))
        .collect();
    let [ExtractStyleProp::Static(ExtractStyleValue::Dynamic(original))] = descriptors.as_slice()
    else {
        panic!("parser-produced padding descriptor")
    };
    assert!(!original.presence());
    assert_eq!(original.conversion(), NumericConversion::Keep);
    let variable = original.variable_name();
    let StyleProperty::Variable {
        class_name,
        variable_name,
        identifier,
    } = descriptors[0].extract()[0]
        .extract(None)
        .unwrap_or_else(|| panic!("variable property"))
    else {
        panic!("dynamic property generator")
    };
    assert_eq!(variable_name, variable);
    assert_eq!(identifier, "state.value");
    // When: the seam receives the actual interpolation AST, not the CSS template.
    let mut captured = crate::assignment_owner::capture_styles(
        template.expressions[0].clone_in(&allocator),
        descriptors,
    );
    // Then: the complete source and CSS declaration retain the parser's identity.
    let [ExtractStyleProp::Static(ExtractStyleValue::Dynamic(selected))] = captured.as_slice()
    else {
        panic!("CSS-text scalar must remain a direct dynamic declaration")
    };
    assert_eq!(selected.identifier(), code(&template.expressions[0]));
    assert_eq!(selected.property(), "padding");
    assert_eq!(selected.effective_value(), format!("var({variable})"));
    let StyleProperty::Variable {
        class_name: selected_class,
        variable_name: selected_variable,
        identifier: selected_identifier,
    } = captured[0].extract()[0]
        .extract(None)
        .unwrap_or_else(|| panic!("captured variable property"))
    else {
        panic!("dynamic property generator")
    };
    assert_eq!(
        (selected_class, selected_variable, selected_identifier),
        (class_name.clone(), variable, "state.value".to_string())
    );
    let output = generated(&ast, &mut captured);
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{reads++;return reads===1?{raw}:'wrong'}}}};{output}JSON.stringify([reads,result.className==={class_name:?},Object.values(result.style)]);"
    ));
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn css_text_scalar_keeps_whole_selection_when_interpolation_has_a_controller() {
    // Given: the CSS parser retains a complete conditional interpolation.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(
        &allocator,
        "`padding: ${state.choose ? state.value : state.other}`",
    );
    let Expression::TemplateLiteral(template) = &source else {
        panic!("conditional CSS template fixture")
    };
    let descriptors = css_to_style_template(template, 0, &None)
        .styles
        .into_iter()
        .map(|style| ExtractStyleProp::Static(style.into()))
        .collect();
    // When: capture owns the actual controller and both authored branches.
    let mut styles = crate::assignment_owner::capture_styles(
        template.expressions[0].clone_in(&allocator),
        descriptors,
    );
    // Then: generated inline evaluation selects only the authored branch, once.
    let [ExtractStyleProp::Static(ExtractStyleValue::Dynamic(value))] = styles.as_slice() else {
        panic!("whole-source scalar descriptor")
    };
    assert_eq!(value.identifier(), "state.choose?state.value:state.other");
    let output = generated(&ast, &mut styles);
    let actual = evaluate(&format!(
        "let trace=[];const state={{get choose(){{trace.push('choose');return false}},get value(){{throw Error('unselected')}},get other(){{trace.push('other');return trace.length===2?'12px':'wrong'}}}};{output}JSON.stringify([trace,Boolean(result.className),Object.values(result.style)]);"
    ));
    assert_eq!(actual, "[[\"choose\",\"other\"],true,[\"12px\"]]");
}

#[rstest]
#[case("4", "[1,true,[4],[4]]")]
#[case("null", "[1,true,[null],[null]]")]
#[serial]
fn css_text_array_keeps_evaluated_shape_when_parser_descriptor_has_no_presence(
    #[case] raw: &str,
    #[case] expected: &str,
) {
    // Given: an actual array interpolation still has a presence-free CSS-text consumer.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "`padding: ${[state.value]}`");
    let Expression::TemplateLiteral(template) = &source else {
        panic!("array CSS template fixture")
    };
    let descriptors = css_to_style_template(template, 0, &None)
        .styles
        .into_iter()
        .map(|style| ExtractStyleProp::Static(style.into()))
        .collect();
    // When: the helper receives the array interpolation itself.
    let mut styles = crate::assignment_owner::capture_styles(
        template.expressions[0].clone_in(&allocator),
        descriptors,
    );
    // Then: the array guard keeps the owner, raw array, and unscaled inline element.
    assert_evaluated(&styles, &template.expressions[0]);
    let output = generated(&ast, &mut styles);
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{reads++;return reads===1?{raw}:'wrong'}}}};{output}JSON.stringify([reads,Boolean(result.className),Object.values(result.style),result.raw]);"
    ));
    assert_eq!(actual, expected);
}
