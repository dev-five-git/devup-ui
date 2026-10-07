use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "<Box className={state.enabled&&'p-4'}/>",
    true,
    "[[\"enabled\"],[\"padding:1rem:0\"]]"
)]
#[case("<Box className={state.enabled&&'p-4'}/>", false, "[[\"enabled\"],[]]")]
#[case(
    "jsx(Box,{className:state.enabled&&'p-4'})",
    true,
    "[[\"enabled\"],[\"padding:1rem:0\"]]"
)]
#[case(
    "jsx(Box,{className:state.enabled&&'p-4'})",
    false,
    "[[\"enabled\"],[]]"
)]
#[serial]
fn rebuilt_tailwind_logical_class_omits_falsy_values(
    #[case] expression: &str,
    #[case] enabled: bool,
    #[case] expected: &str,
) {
    // Given
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';let trace=[];const state={{get enabled(){{trace.push('enabled');return {enabled}}}}};const node={expression};"
    );
    // When
    let actual = selected(
        &source,
        "function jsx(tag,props){return props}JSON.stringify([trace,assigned(node)]);",
    );
    // Then: the true branch selects its emitted padding; false never becomes a class word.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn conditional_styled_argument_keeps_its_creation_evaluation_when_rendered_twice() {
    // Given: the whole authored rule argument selects a static rule at creation.
    let source = "import {styled}from '@devup-ui/react';let reads=0;Object.defineProperty(globalThis,'flag',{get(){reads++;return true}});const A=styled.div(flag?{color:'red'}:{color:'blue'});const created=reads;const first=A({}),second=A({});JSON.stringify([created,reads,first.className===second.className]);";
    // When
    let actual = evaluate(&compiled_jsx(source));
    // Then: rendering never reevaluates the already selected argument.
    assert_eq!(actual, "[1,1,true]");
}
