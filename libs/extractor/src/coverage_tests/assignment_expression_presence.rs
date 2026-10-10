use crate::{
    ExtractStyleProp, ExtractStyleValue,
    assignment_test_support::evaluate,
    coverage_tests::{code, expression},
    extract_style::{
        extract_dynamic_style::ExtractDynamicStyle,
        numeric_conversion::{NumericConversion, NumericUnit},
    },
    gen_class_name::gen_class_names,
    gen_style::gen_styles,
};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(false, ("'4'", "[1,true,[\"16px\"],\"4\"]"))]
#[case(false, ("null", "[1,false,[null],null]"))]
#[case(false, ("''", "[1,false,[\"\"],\"\"]"))]
#[case(true, ("0", "[1,true,[\"0px\"],0]"))]
#[case(true, ("false", "[1,false,[false],false]"))]
#[serial]
fn expression_payload_presence_uses_captured_raw_value_when_assignment_is_rewritten(
    #[case] responsive: bool,
    #[case] fixture: (&str, &str),
) {
    // Given: the Expression IR payload carries an ordinary, absence-sensitive
    // declaration. Its raw assignment is captured by the caller, not reread.
    let (raw, expected) = fixture;
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let identifier = if responsive {
        "state.value?.[2]"
    } else {
        "state.value"
    };
    let original = ExtractStyleValue::Dynamic(
        ExtractDynamicStyle::new("gap", if responsive { 2 } else { 0 }, identifier, None)
            .with_presence()
            .with_conversion(NumericConversion::Unknown(NumericUnit::Length)),
    );
    let mut props = [ExtractStyleProp::Expression {
        expression: expression(&allocator, "('external-class')"),
        styles: vec![original.clone()],
    }];

    // When: presence rewrites the matching Expression payload to the captured
    // scalar or responsive slot; consume that payload through real generators.
    super::rewrite(&mut props, &[original], responsive);
    let mut declarations = props[0]
        .extract()
        .into_iter()
        .map(ExtractStyleProp::Static)
        .collect::<Vec<_>>();
    let class = gen_class_names(&ast, &mut declarations, None, None)
        .unwrap_or_else(|| panic!("presence class fixture"));
    let inline = gen_styles(&ast, &declarations, None)
        .unwrap_or_else(|| panic!("inline declaration fixture"));
    let input = if responsive {
        format!("[null,null,{raw}]")
    } else {
        raw.to_string()
    };
    let slot = if responsive {
        "result.raw?.[2]"
    } else {
        "result.raw"
    };
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{reads++;return reads===1?{input}:'changed'}}}};const result=((__devupValue)=>({{className:{},style:{},raw:__devupValue}}))(state.value);JSON.stringify([reads,Boolean(result.className),Object.values(result.style),{slot}]);",
        code(&class),
        code(&inline),
    ));

    // Then: presence and numeric conversion share the first raw read, including
    // absent values and active zero, without changing the caller's raw value.
    assert_eq!(actual, expected);
}
