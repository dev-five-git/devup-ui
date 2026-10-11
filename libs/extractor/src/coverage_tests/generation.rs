use super::{code, dynamic, expression};
use crate::{ExtractStyleProp, assignment_test_support::evaluate, gen_style::gen_styles};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use serial_test::serial;
use std::collections::BTreeMap;

#[test]
#[serial]
fn inline_value_is_absent_when_one_sided_condition_is_false() {
    // Given
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let styles = [ExtractStyleProp::Conditional {
        condition: expression(&allocator, "enabled"),
        consequent: Some(Box::new(ExtractStyleProp::Static(dynamic(
            "color", "chosen",
        )))),
        alternate: None,
    }];
    // When
    let generated =
        code(&gen_styles(&ast, &styles, None).unwrap_or_else(|| panic!("inline value")));
    let actual = evaluate(&format!(
        "let enabled=false,chosen='red';const absent={generated};enabled=true;const present={generated};JSON.stringify([Object.values(absent),Object.values(present)]);"
    ));
    // Then
    assert_eq!(actual, "[[null],[\"red\"]]");
}

#[test]
#[serial]
fn shared_variable_selects_only_the_requested_branch() {
    // Given: identical property identity and distinct observable inputs.
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let styles = [ExtractStyleProp::Conditional {
        condition: expression(&allocator, "enabled"),
        consequent: Some(Box::new(ExtractStyleProp::Static(dynamic(
            "color",
            "state.first",
        )))),
        alternate: Some(Box::new(ExtractStyleProp::Static(dynamic(
            "color",
            "state.second",
        )))),
    }];
    // When
    let generated =
        code(&gen_styles(&ast, &styles, None).unwrap_or_else(|| panic!("inline value")));
    let actual = evaluate(&format!(
        "let enabled=true,trace=[];const state={{get first(){{trace.push('first');return 'red'}},get second(){{trace.push('second');return 'blue'}}}};const first={generated};enabled=false;const second={generated};JSON.stringify([trace,Object.keys(first).length,Object.keys(second).length,Object.values(first),Object.values(second)]);"
    ));
    // Then
    assert_eq!(actual, "[[\"first\",\"second\"],1,1,[\"red\"],[\"blue\"]]");
}

#[test]
#[serial]
fn single_member_variable_reads_its_value_without_reading_the_selector() {
    // Given
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let styles = [ExtractStyleProp::MemberExpression {
        map: BTreeMap::from([(
            "one".into(),
            Box::new(ExtractStyleProp::Static(dynamic("color", "state.value"))),
        )]),
        expression: expression(&allocator, "state.key"),
    }];
    // When
    let generated =
        code(&gen_styles(&ast, &styles, None).unwrap_or_else(|| panic!("inline value")));
    let actual = evaluate(&format!(
        "let reads=0;const state={{get value(){{reads++;return 'red'}},get key(){{throw Error('unused selector')}}}};const result={generated};JSON.stringify([reads,Object.values(result)]);"
    ));
    // Then: this IR seam has already selected the sole variable consumer.
    assert_eq!(actual, "[1,[\"red\"]]");
}
