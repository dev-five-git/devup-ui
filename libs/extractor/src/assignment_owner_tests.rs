use crate::{
    ExtractStyleProp, ExtractStyleValue,
    assignment_test_support::evaluate,
    coverage_tests::{code, expression},
    extractor::extract_style_from_expression::{LiteralHandling, extract_style_from_expression},
    gen_class_name::gen_class_names,
    gen_style::gen_styles,
};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::GetSpan;
use rstest::rstest;
use serial_test::serial;
use std::fmt::Write;

#[path = "assignment_owner_css_text_tests.rs"]
mod css_text;

fn generated<'a>(ast: &AstBuilder<'a>, styles: &mut [ExtractStyleProp<'a>]) -> String {
    let raw = match styles {
        [ExtractStyleProp::Evaluated { binding, .. }] => format!("{binding}[2]"),
        _ => "undefined".to_string(),
    };
    let class =
        gen_class_names(ast, styles, None, None).unwrap_or_else(|| panic!("generated class"));
    let inline = gen_styles(ast, styles, None).unwrap_or_else(|| panic!("generated inline style"));
    let mut declarations = String::new();
    for (binding, value) in crate::assignment_lowering::take_evaluations(styles) {
        write!(declarations, "const {binding}={};", code(&value))
            .unwrap_or_else(|_| panic!("generated declarations"));
    }
    format!(
        "{declarations}const result={{className:{},style:{},raw:{raw}}};",
        code(&class),
        code(&inline)
    )
}

fn assert_evaluated(styles: &[ExtractStyleProp<'_>], original: &Expression<'_>) {
    let [
        ExtractStyleProp::Evaluated {
            binding,
            source,
            evaluation,
            alternate_order,
            alternate_class,
            ..
        },
    ] = styles
    else {
        panic!("non-scalar assignment must retain its evaluated owner")
    };
    assert_eq!(
        binding,
        &crate::sparse_sites::binding_name(original.span().start)
    );
    assert_eq!(source.span(), original.span());
    assert_eq!(code(source), code(original));
    assert!(evaluation.is_none());
    assert!(alternate_order.is_none());
    assert!(!alternate_class);
}

#[rstest]
#[case("4", "[1,true,[\"16px\"],4]")]
#[case("'4'", "[1,true,[\"16px\"],\"4\"]")]
#[case("null", "[1,false,[null],null]")]
#[case("false", "[1,false,[false],false]")]
#[case("''", "[1,false,[\"\"],\"\"]")]
#[serial]
fn ordinary_scalar_keeps_presence_and_raw_capture_when_api_builds_an_owner(
    #[case] raw: &str,
    #[case] expected: &str,
) {
    // Given: ordinary property extraction produces a genuinely presence-sensitive value.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut source = expression(&allocator, "state.value");
    let original = source.clone_in(&allocator);
    // When: the public expression API routes through the capture seam.
    let mut styles = extract_style_from_expression(
        &ast,
        Some("p"),
        &mut source,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    )
    .styles;
    // Then: it retains deferred evaluation, absence selection and the unconverted raw slot.
    assert_evaluated(&styles, &original);
    let values = styles[0].extract();
    let [ExtractStyleValue::Dynamic(value)] = values.as_slice() else {
        panic!("ordinary parser-produced dynamic value")
    };
    assert!(value.presence());
    assert_eq!(super::scalar(&original, &styles), None);
    let output = generated(&ast, &mut styles);
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{reads++;return reads===1?{raw}:'wrong'}}}};{output}JSON.stringify([reads,Boolean(result.className),Object.values(result.style),result.raw]);"
    ));
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn ordinary_array_keeps_read_order_and_absence_when_api_builds_responsive_consumers() {
    // Given: two actual responsive getters, one numeric and one absent.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut source = expression(&allocator, "[state.first,state.second]");
    let original = source.clone_in(&allocator);
    // When: ordinary array extraction goes through the same seam.
    let mut styles = extract_style_from_expression(
        &ast,
        Some("p"),
        &mut source,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    )
    .styles;
    // Then: metadata, raw values, absence and authored getter order remain intact.
    assert_evaluated(&styles, &original);
    let values = styles[0].extract();
    let variables: Vec<_> = values
        .iter()
        .map(|value| match value {
            ExtractStyleValue::Dynamic(value) => {
                assert!(value.presence());
                value.variable_name()
            }
            _ => panic!("responsive dynamic descriptor"),
        })
        .collect();
    assert_eq!(variables.len(), 2);
    let output = generated(&ast, &mut styles);
    let actual = evaluate(&format!(
        "let trace=[];const state={{get first(){{trace.push('first');return trace.length===1?4:'wrong'}},get second(){{trace.push('second');return trace.length===2?null:'wrong'}}}};{output}JSON.stringify([trace,result.className.trim().split(/\\s+/).length,[result.style[{:?}],result.style[{:?}]],result.raw]);",
        variables[0], variables[1]
    ));
    assert_eq!(
        actual,
        "[[\"first\",\"second\"],1,[\"16px\",null],[4,null]]"
    );
}
