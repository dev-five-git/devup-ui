use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("4", "[\"16px\"]")]
#[case("'4'", "[\"16px\"]")]
#[case("'10px'", "[\"10px\"]")]
#[serial]
fn numeric_creation_values_remain_converted_when_the_component_renders_twice(
    #[case] value: &str,
    #[case] expected: &str,
) {
    let source = "import {styled}from '@devup-ui/react';const A=styled.div({p:read()});";
    let actual = evaluate(&format!(
        "let reads=0;function read(){{reads++;return {value}}}{}const created=reads;const first=A({{}});const second=A({{}});JSON.stringify([created,reads,Object.values(first.style),Object.values(second.style)]);",
        compiled_jsx(source)
    ));
    assert_eq!(actual, format!("[1,1,{expected},{expected}]"));
}
