use super::{code, dynamic, expression};
use crate::{ExtractStyleProp, ExtractStyleValue, assignment_test_support::evaluate};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::builder::AstBuilder;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn styled_creation_avoids_shadowing_when_authored_code_uses_the_capture_name() {
    use oxc_span::GetSpan;
    // Given: the render reads an outer binding with the would-be generated capture name.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "({color:state.value})");
    let oxc_ast::ast::Expression::ObjectExpression(object) = &source else {
        panic!("literal creation fixture")
    };
    let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = &object.properties[0] else {
        panic!("literal field fixture")
    };
    let name = format!("__devupStyled{}", property.value.span().start);
    let render = format!("()=>{name}");
    let mut component = expression(&allocator, &render);
    // When: creation captures its field without changing the authored render binding.
    crate::assignment_capture::styled_creation(
        &ast,
        &mut component,
        crate::assignment_capture::StyledCreation {
            source: &source,
            styles: &mut [],
        },
    );
    let actual = evaluate(&format!(
        "let reads=0;const {name}=11;const state={{get value(){{reads++;return 'red'}}}};const render={};JSON.stringify([reads,render(),render()]);",
        code(&component)
    ));
    // Then: one creation read does not hijack either render's lexical binding.
    assert_eq!(actual, "[1,11,11]");
}

#[rstest]
#[case("read()", true)]
#[case("flag ? read() : other()", true)]
#[case("flag && read()", true)]
#[case("({a:read(),b:other()})[key]", true)]
#[case("[read(),other()][key]", true)]
#[case("state[key]", true)]
#[case("({a:read(),...state})[key]", false)]
#[case("({a:[read()]})[key]", false)]
#[case("flag ? [read()] : other()", false)]
#[case("flag && [read()]", false)]
#[case("`${read()}`", false)]
#[serial]
fn scalar_owner_keeps_authored_selection_when_every_consumer_is_always_present(
    #[case] source: &str,
    #[case] eligible: bool,
) {
    // Given: same-property consumers with no absence guard or responsive structure.
    let allocator = Allocator::default();
    let source = expression(&allocator, source);
    let styles = [ExtractStyleProp::Static(dynamic("color", "read()"))];
    // When: the owner attempts a single scalar assignment.
    let selected = crate::assignment_owner::scalar(&source, &styles);
    // Then: scalar syntax preserves its authored selection; structured syntax cannot collapse.
    let actual = selected.map(|value| match value {
        ExtractStyleValue::Dynamic(value) => value.identifier().to_string(),
        _ => panic!("scalar owner must retain a dynamic declaration"),
    });
    assert_eq!(actual, eligible.then(|| code(&source)));
}

#[rstest]
#[case(
    "({a:state.first,b:state.second})[state.key]",
    "b",
    "[\"blue\",[\"first\",\"second\",\"key\"]]"
)]
#[case(
    "[state.first,state.second][state.key]",
    "1",
    "[\"blue\",[\"first\",\"second\",\"key\"]]"
)]
#[case("state.values[state.key]", "b", "[\"blue\",[\"key\"]]")]
#[serial]
fn unmapped_member_fields_keep_raw_reads_when_only_one_member_has_a_consumer(
    #[case] source: &str,
    #[case] key: &str,
    #[case] expected: &str,
) {
    // Given: selection can reach a field with no declaration in the consumer map.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, source);
    let mut styles = [ExtractStyleProp::MemberExpression {
        expression: source.clone_in(&allocator),
        map: [(
            "a".into(),
            Box::new(ExtractStyleProp::Static(dynamic("color", "state.first"))),
        )]
        .into(),
    }];
    let lowering = crate::assignment_lowering::Lowering {
        ast: &ast,
        order: None,
        filename: None,
        alternate_order: None,
    };
    // When: raw selection executes after lowering the partial consumer map.
    let generated = lowering.lower(&source, &mut styles);
    let actual = evaluate(&format!(
        "let trace=[];const state={{values:{{b:'blue'}},get first(){{trace.push('first');return 'red'}},get second(){{trace.push('second');return 'blue'}},get key(){{trace.push('key');return '{key}'}}}};const result={};JSON.stringify([result[2],trace]);",
        code(&generated)
    ));
    // Then: unmapped branches still supply raw values, in authored evaluation order.
    assert_eq!(actual, expected);
}

#[rstest]
#[case("'a'", "[1,true,\"a\"]")]
#[case("null", "[1,false,null]")]
#[serial]
fn presence_rewrites_member_consumers_when_raw_value_was_captured(
    #[case] raw: &str,
    #[case] expected: &str,
) {
    // Given: an absence-sensitive declaration nested in a member selection.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = expression(&allocator, "state[key]");
    let value = ExtractStyleValue::Dynamic(
        crate::extract_style::extract_dynamic_style::ExtractDynamicStyle::new(
            "color",
            0,
            "state[key]",
            None,
        )
        .with_presence(),
    );
    let mut styles = [ExtractStyleProp::MemberExpression {
        expression: source.clone_in(&allocator),
        map: [("a".into(), Box::new(ExtractStyleProp::Static(value)))].into(),
    }];
    let lowering = crate::assignment_lowering::Lowering {
        ast: &ast,
        order: None,
        filename: None,
        alternate_order: None,
    };
    // When: generated class selection executes with a getter whose second read differs.
    let generated = lowering.lower(&source, &mut styles);
    let actual = evaluate(&format!(
        "let reads=0;const key='value';const state={{get value(){{reads++;return reads===1?{raw}:'a'}}}};const result={};JSON.stringify([reads,Boolean(result[0]),result[2]]);",
        code(&generated)
    ));
    // Then: both absence and raw selection refer to the first read.
    assert_eq!(actual, expected);
}
