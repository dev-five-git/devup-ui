use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[path = "assignment_mixed_container_support.rs"]
mod support;

#[rstest]
#[case("<Box _hover={{p:state.pad,color:'red'}} id={state.id}>{state.child}</Box>")]
#[case("<Box selectors={{'& > span':{p:state.pad,color:'red'}}} id={state.id}>{state.child}</Box>")]
#[case("jsx(Box,{_hover:{p:state.pad,color:'red'},id:state.id,children:state.child})")]
#[case(
    "jsx(Box,{selectors:{'& > span':{p:state.pad,color:'red'}},id:state.id,children:state.child})"
)]
#[serial]
fn mixed_container_reads_leaf_once_when_static_class_is_present(#[case] expression: &str) {
    // Given: literal color and observable padding before adjacent operands.
    let source = fixture(expression, "true", "return '16px'");
    let original = support::source_trace(&source);
    assert_eq!(original, "[\"pad\",\"id\",\"child\"]");
    // When: the native compiled JSX executes its actual selected classes/variables.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: the literal class survives and padding is captured before id/child.
    assert_eq!(
        actual,
        "[[\"pad\",\"id\",\"child\"],[\"color:red:0\",\"padding:16px:0\"]]"
    );
}

#[rstest]
#[case(
    "<Box _hover={{color:state.flag?'red':'blue',p:state.pad}} id={state.id}>{state.child}</Box>",
    "true",
    "red"
)]
#[case(
    "<Box _hover={{color:state.flag?'red':'blue',p:state.pad}} id={state.id}>{state.child}</Box>",
    "false",
    "blue"
)]
#[case(
    "jsx(Box,{_hover:{color:state.flag?'red':'blue',p:state.pad},id:state.id,children:state.child})",
    "true",
    "red"
)]
#[case(
    "jsx(Box,{_hover:{color:state.flag?'red':'blue',p:state.pad},id:state.id,children:state.child})",
    "false",
    "blue"
)]
#[case(
    "<Box selectors={{'& > span':{color:state.flag?'red':'blue',p:state.pad}}} id={state.id}>{state.child}</Box>",
    "false",
    "blue"
)]
#[serial]
fn mixed_container_selects_only_authored_branch_when_color_is_conditional(
    #[case] expression: &str,
    #[case] flag: &str,
    #[case] color: &str,
) {
    // Given: static color alternatives controlled by an observable getter.
    let source = fixture(expression, flag, "return '16px'");
    let original = support::source_trace(&source);
    assert_eq!(original, "[\"flag\",\"pad\",\"id\",\"child\"]");
    // When: the selected branch and dynamic sibling execute together.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: there is exactly one color class, with one ordered predicate read.
    assert_eq!(
        actual,
        format!("[[\"flag\",\"pad\",\"id\",\"child\"],[\"color:{color}:0\",\"padding:16px:0\"]]")
    );
}

#[rstest]
#[case("<Box _hover={{p:state.pad,color:'red'}} id={state.id}>{state.child}</Box>")]
#[case("jsx(Box,{_hover:{p:state.pad,color:'red'},id:state.id,children:state.child})")]
#[serial]
fn mixed_container_stops_adjacent_operands_when_first_leaf_throws(#[case] expression: &str) {
    // Given: the first authored leaf throws before adjacent operands.
    let source = fixture(expression, "true", "throw Error('pad failed')").replace(
        "const node=render(state);",
        "let error;try{render(state)}catch(value){error=value.message}",
    );
    assert_eq!(support::source_trace(&source), "[\"pad\"]");
    // When: compiled source encounters the failing leaf.
    let actual = evaluate(&format!(
        "{}JSON.stringify([trace,error]);",
        compiled_jsx(&source)
    ));
    // Then: neither id nor child was evaluated.
    assert_eq!(actual, "[[\"pad\"],\"pad failed\"]");
}

#[rstest]
#[case(
    "true",
    "[[\"flag\",\"pad\",\"id\",\"child\"],[\"color:red:0\",\"padding:16px:0\"]]"
)]
#[case("false", "[[\"flag\",\"pad\",\"id\",\"child\"],[\"padding:16px:0\"]]")]
#[serial]
fn mixed_container_keeps_no_style_branch_when_static_color_is_guarded(
    #[case] flag: &str,
    #[case] expected: &str,
) {
    // Given: a no-style logical color branch and a dynamic sibling.
    let source = fixture(
        "<Box _hover={{color:state.flag&&'red',p:state.pad}} id={state.id}>{state.child}</Box>",
        flag,
        "return '16px'",
    );
    // When: the guard is selected.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: the false branch contributes no class, without skipping padding.
    assert_eq!(actual, expected);
}

fn fixture(expression: &str, flag: &str, pad: &str) -> String {
    format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{get flag(){{trace.push('flag');return {flag}}},get pad(){{trace.push('pad');{pad}}},get id(){{trace.push('id');return 'target'}},get child(){{trace.push('child');return 'text'}}}};const jsx=(tag,props)=>props;const node=render(state);"
    )
}

#[rstest]
#[case(
    "<Box selectors={{'& > span':{p:state.pad,color:state.flag?'red':'blue'},'& > b':{p:state.pad,color:'green'}}} id={state.id}>{state.child}</Box>"
)]
#[case(
    "jsx(Box,{selectors:{'& > span':{p:state.pad,color:state.flag?'red':'blue'},'& > b':{p:state.pad,color:'green'}},id:state.id,children:state.child})"
)]
#[serial]
fn nested_mixed_selectors_keep_associations_when_identical_getters_return_distinct_values(
    #[case] expression: &str,
) {
    // Given: identical property/getter syntax under different authored selectors.
    let source = fixture(expression, "false", "return ++reads===1?'16px':'24px'")
        .replace("let trace=[];", "let trace=[];let reads=0;");
    // When: emitted class identities are joined to selector/property/variable records.
    let actual = support::associated(&source);
    // Then: each selector receives its own value, while native source trace is unchanged.
    assert_eq!(
        actual,
        "[\"& > b|color:green:0|\",\"& > b|padding:24px:0|\",\"& > span|color:blue:0|\",\"& > span|padding:16px:0|\"]"
    );
}

#[rstest]
#[case(
    "<Box _hover={{p:['4px',state.pad],color:state.flag?'red':'blue'}} id={state.id}>{state.child}</Box>"
)]
#[case(
    "jsx(Box,{_hover:{p:['4px',state.pad],color:state.flag?'red':'blue'},id:state.id,children:state.child})"
)]
#[serial]
fn mixed_array_field_keeps_levels_when_literal_and_dynamic_values_coexist(
    #[case] expression: &str,
) {
    // Given: a responsive field with a literal role and a dynamic role.
    let source = fixture(expression, "false", "return '16px'");
    // When: extraction preserves the field's semantic IR before flattening.
    let actual = support::associated(&source);
    // Then: literal and dynamic padding retain their distinct levels and selector.
    assert_eq!(
        actual,
        "[\"&:hover|color:blue:0|\",\"&:hover|padding:16px:1|\",\"&:hover|padding:4px:0|\"]"
    );
}

#[rstest]
#[case(
    "true",
    "[\"&:hover:focus|color:red:0|\",\"&:hover:focus|padding:16px:0|\",\"&:hover|margin:8px:0|\"]"
)]
#[case("false", "[\"&:hover|margin:8px:0|\"]")]
#[serial]
fn mixed_nested_container_is_lazy_when_logical_branch_has_no_styles(
    #[case] flag: &str,
    #[case] expected: &str,
) {
    // Given: an entire guarded selector object next to an unconditional literal.
    let source = fixture(
        "<Box _hover={{_focus:state.flag&&{color:'red',p:state.pad},m:'8px'}} id={state.id}>{state.child}</Box>",
        flag,
        "return '16px'",
    );
    // When: the compiled object is compared with authored native source execution.
    let actual = support::associated(&source);
    // Then: the false branch never reads padding or emits either guarded class.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn mixed_layer_container_keeps_layer_when_properties_are_repeated() {
    // Given: repeated padding under distinct source-authored layers.
    let source = fixture("<Box _hover={{'@layer':{first:{p:state.pad,color:'red'},second:{p:state.pad,color:state.flag?'blue':'green'}}}} id={state.id}>{state.child}</Box>", "true", "return ++reads===1?'16px':'24px'")
        .replace("let trace=[];", "let trace=[];let reads=0;");
    // When: actual emitted classes and inline assignments execute.
    let actual = support::associated(&source);
    // Then: layer, selector, property and selected value remain paired.
    assert_eq!(
        actual,
        "[\"&:hover|color:blue:0|second\",\"&:hover|color:red:0|first\",\"&:hover|padding:16px:0|first\",\"&:hover|padding:24px:0|second\"]"
    );
}

#[test]
#[serial]
fn literal_only_container_has_no_capture_when_no_runtime_leaf_exists() {
    // Given: a pure literal selector object and observable adjacent operands.
    let source = fixture(
        "<Box _hover={{p:'16px',color:'red'}} id={state.id}>{state.child}</Box>",
        "true",
        "return 'unused'",
    );
    // When: the compiler extracts the static container.
    let output = compiled_jsx(&source);
    // Then: it introduces no assignment/field/value capture for that container.
    assert!(!output.contains("__devupAssignment"));
    assert!(!output.contains("__devupField"));
    assert!(!output.contains("__devupValue"));
}
