use super::{code, dynamic, expression};
use crate::{ExtractStyleProp, ExtractStyleValue, assignment_test_support::evaluate};
use oxc_allocator::Allocator;
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::{GetSpan, SPAN};
use rstest::rstest;
use serial_test::serial;

#[test]
fn composition_capture_preserves_getter_order_when_arguments_choose_classes() {
    // Given: class arguments have observable reads and a conditional choice.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut source = expression(
        &allocator,
        "compose(state.first,state.flag?'yes':'no',state.last)",
    );
    let Expression::CallExpression(call) = &mut source else {
        panic!("composition call fixture")
    };
    // When: captured arguments execute together with the composed call.
    let captures = crate::assignment_composition::capture(&ast, &mut call.arguments);
    let generated = crate::utils::call_with_values(&ast, captures, source);
    let actual = evaluate(&format!(
        "let trace=[];const state={{get first(){{trace.push('first');return 'a'}},get flag(){{trace.push('flag');return false}},get last(){{trace.push('last');return 'b'}}}};function compose(...values){{return values.join(' ')}}const result={};JSON.stringify([result,trace]);",
        code(&generated)
    ));
    // Then: each controller and class read stays once and in source order.
    assert_eq!(actual, "[\"a no b\",[\"first\",\"flag\",\"last\"]]");
}

#[test]
fn class_template_capture_coerces_once_when_two_orders_share_an_interpolation() {
    // Given: the same authored interpolation feeds primary, raw and alternate classes.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "`raw ${state.part}`");
    let mut prepared = [
        Some(expression(&allocator, "`first ${state.part}`")),
        Some(expression(&allocator, "`last ${state.part}`")),
    ];
    // When: joint class capture is evaluated with observable string coercion.
    let (_, generated) = crate::class_evaluation::capture(&ast, &source, &mut prepared)
        .unwrap_or_else(|| panic!("joint template capture"));
    let actual = evaluate(&format!(
        "let trace=[];const state={{get part(){{trace.push('get');return {{toString(){{trace.push('string');return 'x'}}}}}}}};const result={};JSON.stringify([result[0],result[2],result[3],trace]);",
        code(&generated)
    ));
    // Then: changing class order never repeats the interpolation or coercion.
    assert_eq!(
        actual,
        "[\"first x\",\"raw x\",\"last x\",[\"get\",\"string\"]]"
    );
}

#[rstest]
#[case("state.value", "state.value")]
#[case("`${state.value} !important`", "`${state.value} !important`")]
#[serial]
fn scalar_capture_uses_normalized_input_when_a_fixed_important_tail_was_removed(
    #[case] authored: &str,
    #[case] identifier: &str,
) {
    // Given: scalar consumers share a getter, with or without normalization metadata.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, authored);
    let mut styles = [ExtractStyleProp::Static(dynamic("color", identifier))];
    // When: element capture prepares the value before inline style generation.
    let (binding, captured) = crate::element_evaluation::scalar(&ast, &mut styles, &source, &mut 0)
        .unwrap_or_else(|| panic!("scalar capture"));
    let inline = crate::gen_style::gen_styles(&ast, &styles, None)
        .unwrap_or_else(|| panic!("inline scalar style"));
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{reads++;return 'red'}}}};const {binding}={};const inline={};JSON.stringify([reads,Object.values(inline)]);",
        code(&captured),
        code(&inline)
    ));
    // Then: CSS gets a clean value from one read in both cases.
    assert_eq!(actual, "[1,[\"red\"]]");
}

#[test]
#[serial]
fn styled_variable_capture_converts_once_when_generated_render_uses_the_variable_key() {
    use crate::extract_style::{
        extract_dynamic_style::ExtractDynamicStyle,
        numeric_conversion::{NumericConversion, NumericUnit},
    };
    // Given: generated inline styles refer to the variable owned by a literal creation field.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let text = "({p:state.value})";
    let source = expression(&allocator, text);
    let Expression::ObjectExpression(object) = &source else {
        panic!("creation object")
    };
    let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        panic!("creation field")
    };
    let _scope = crate::sparse_sites::SiteScope::enter("capture.tsx", text, &[]);
    let value = ExtractDynamicStyle::new("padding", 0, "state.value", None)
        .at(property.value.span().start)
        .with_conversion(NumericConversion::Unknown(NumericUnit::Length));
    let variable = value.variable_name();
    let mut styles = [ExtractStyleProp::Static(ExtractStyleValue::Dynamic(value))];
    let render = format!("()=>({{style:{{'{variable}':state.value}}}})");
    let mut component = expression(&allocator, &render);
    let Expression::ArrowFunctionExpression(arrow) = &mut component else {
        panic!("generated render")
    };
    arrow.span = SPAN;
    // When: creation binds the raw field and generated variable uses its conversion.
    crate::assignment_capture::styled_creation(
        &ast,
        &mut component,
        crate::assignment_capture::StyledCreation {
            source: &source,
            styles: &mut styles,
        },
    );
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{reads++;return 4}}}};const render={};const first=render(),second=render();JSON.stringify([reads,Object.values(first.style),Object.values(second.style)]);",
        code(&component)
    ));
    // Then: both renders reuse the one converted creation value.
    assert_eq!(actual, "[1,[\"16px\"],[\"16px\"]]");
}
