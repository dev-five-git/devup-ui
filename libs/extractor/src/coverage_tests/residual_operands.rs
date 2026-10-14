use super::{code, dynamic, expression};
use crate::extract_style::{
    extract_css::ExtractCss, extract_keyframes::ExtractKeyframes,
    extract_static_style::ExtractStaticStyle, style_property::StyleProperty,
};
use crate::{ExtractStyleProp, ExtractStyleValue, assignment_test_support::evaluate};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("({color:state.color,other:state.other})", true)]
#[case("[state.color,state.other]", false)]
#[serial]
fn residual_special_records_do_not_consume_color_fields_or_responsive_slots(
    #[case] source: &str,
    #[case] object: bool,
) {
    // Given: real special records, which are not property/level consumers.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, source);
    let frames = ExtractKeyframes {
        keyframes: [(
            "from".into(),
            vec![ExtractStaticStyle::new("opacity", "0", 0, None)],
        )]
        .into(),
        ..Default::default()
    };
    let Some(StyleProperty::ClassName(frame_class)) =
        ExtractStyleValue::Keyframes(frames.clone()).extract(None)
    else {
        panic!("keyframes class")
    };
    let mut styles = [
        ExtractStyleProp::Static(dynamic("color", "state.color")),
        ExtractStyleProp::Static(ExtractStyleValue::Keyframes(frames)),
        ExtractStyleProp::Static(ExtractStyleValue::Css(ExtractCss {
            css: "body { color: red; }".into(),
            file: "residual.css".into(),
        })),
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
        "let trace=[];const state={{get color(){{trace.push('color');return 'red'}},get other(){{trace.push('other');return 'blue'}}}};const result={};JSON.stringify([Array.isArray(result[2])?result[2]:[result[2].color,result[2].other],trace,result[0].split(' ').filter(x=>x==='{}').length,Object.values(result[1])]);",
        code(&generated),
        frame_class,
    ));
    // Then: object residual class appears once; arrays omit non-level records.
    let expected_count = usize::from(object);
    assert_eq!(
        actual,
        format!("[[\"red\",\"blue\"],[\"color\",\"other\"],{expected_count},[\"red\"]]")
    );
}
