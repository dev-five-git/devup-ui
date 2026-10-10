use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate, extracted};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box _hover={{p:state.pad}} id={state.id}>{state.child}</Box>")]
#[case("<Box selectors={{'& > span':{p:state.pad}}} id={state.id}>{state.child}</Box>")]
#[case("jsx(Box,{_hover:{p:state.pad},id:state.id,children:state.child})")]
#[case("jsx(Box,{selectors:{'& > span':{p:state.pad}},id:state.id,children:state.child})")]
#[serial]
fn selector_container_assigns_leaf_when_adjacent_reads_are_captured(#[case] expression: &str) {
    // Given: an observable scalar leaf inside a selector object and later operands.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{get pad(){{trace.push('pad');return '16px'}},get id(){{trace.push('id');return 'target'}},get child(){{trace.push('child');return 'text'}}}};const jsx=(tag,props)=>props;const node=render(state);"
    );
    // When: actual emitted classes are joined to their actual inline variables.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node),node.id]);");
    // Then: padding receives the leaf, never the containing object, in source order.
    assert_eq!(
        actual,
        "[[\"pad\",\"id\",\"child\"],[\"padding:16px:0\"],\"target\"]",
        "{}",
        compiled_jsx(&source)
    );
}

#[rstest]
#[case("<Box _hover={{['p']:state.pad,color:state.color}} id={state.id}>{state.child}</Box>")]
#[case(
    "<Box selectors={{'& > span':{['p']:state.pad,color:state.color},'& > b':{p:state.other}}} id={state.id}>{state.child}</Box>"
)]
#[case("jsx(Box,{_hover:{['p']:state.pad,color:state.color},id:state.id,children:state.child})")]
#[serial]
fn selector_container_keeps_variable_associations_when_properties_are_distinct(
    #[case] expression: &str,
) {
    // Given: different property values, computed literal keys and adjacent getters.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{}};for(const[key,value]of Object.entries({{pad:'16px',color:'red',other:'24px',id:'target',child:'text'}}))Object.defineProperty(state,key,{{get(){{trace.push(key);return value}}}});const jsx=(tag,props)=>props;const node=render(state);"
    );
    let descendant = expression.contains("selectors");
    // When: selected classes consume the variable assigned by their authored leaf.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: different leaves neither alias nor receive an entire object.
    let expected = if descendant {
        "[[\"pad\",\"color\",\"other\",\"id\",\"child\"],[\"color:red:0\",\"padding:16px:0\",\"padding:24px:0\"]]"
    } else {
        "[[\"pad\",\"color\",\"id\",\"child\"],[\"color:red:0\",\"padding:16px:0\"]]"
    };
    assert_eq!(actual, expected);
}

#[rstest]
#[case("0", "[\"padding:0:0\"]")]
#[case("-0", "[\"padding:0:0\"]")]
#[case("false", "[]")]
#[case("null", "[]")]
#[case("undefined", "[]")]
#[case("''", "[]")]
#[case("NaN", "[]")]
#[serial]
fn and_literal_value_is_selected_when_controller_is_falsy(
    #[case] controller: &str,
    #[case] expected: &str,
) {
    // Given: an AND value with observable left and right getters.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box p={{state.a&&state.b}} id={{state.id}}/>}}let trace=[];const state={{get a(){{trace.push('a');return {controller}}},get b(){{trace.push('b');return '16px'}},get id(){{trace.push('id');return 'target'}}}};const node=render(state);"
    );
    // When: the compiled value selects the controller's own literal style.
    let actual = selected(
        &source,
        "JSON.stringify([trace,assigned(node),Object.values(node.style??{})]);",
    );
    // Then: zero applies padding zero, absent values apply nothing, and b is unread.
    assert_eq!(actual, format!("[[\"a\",\"id\"],{expected},[]]"));
}

#[test]
#[serial]
fn literal_selectors_keep_compiling_when_children_are_conditional_arrays() {
    // Given: literal selectors/keys and lazy conditional array children.
    let source = "import {Box,css}from '@devup-ui/react';function render(state){return <Box selectors={{'& > span':{['p']:state.pad}}} id={state.id}>{state.flag?[state.child]:[]}</Box>}let trace=[];const state={pad:'16px',id:'target',flag:false,get child(){trace.push('child');return 'text'}};const node=render(state);";
    // When: the actual output executes the false child branch.
    let actual = selected(
        source,
        "JSON.stringify([trace,assigned(node),node.children]);",
    );
    // Then: selector classes remain selected and the unused child is not read.
    assert_eq!(actual, "[[],[\"padding:16px:0\"],[[]]]");
}

#[test]
#[serial]
fn css_and_literal_keeps_compiling_when_background_is_guarded() {
    // Given: the previously valid main/#751 literal CSS guard.
    let source = "import {css}from '@devup-ui/react';const LIGHT='white';function render(dark){return css({bg:dark&&LIGHT})}const node=render(false);";
    // When: extraction compiles the guard without rejecting valid CSS.
    let output = extracted(source);
    // Then: its no-style branch executes without a generated class.
    assert_eq!(
        evaluate(&format!(
            "{}JSON.stringify(node);",
            crate::assignment_test_support::jsx_js(&output.code)
        )),
        "\"\""
    );
}

#[rstest]
#[case(
    "<Box selectors={{'& > span':{p:state.pad},'& > b':{p:state.pad}}} id={state.id}>{state.child}</Box>"
)]
#[case(
    "jsx(Box,{selectors:{'& > span':{p:state.pad},'& > b':{p:state.pad}},id:state.id,children:state.child})"
)]
#[serial]
fn selector_container_keeps_source_sites_when_repeated_getters_return_different_values(
    #[case] expression: &str,
) {
    // Given: BOM/CRLF/multibyte source and identical getter syntax at different sites.
    let source = format!(
        "\u{feff}import {{Box}}from '@devup-ui/react';\r\nimport {{jsx}}from 'react/jsx-runtime';\r\nconst label='한';function render(state){{return {expression}}}let trace=[];let reads=0;const state={{get pad(){{trace.push('pad');return ++reads===1?'16px':'24px'}},get id(){{trace.push('id');return 'target'}},get child(){{trace.push('child');return 'text'}}}};const jsx=(tag,props)=>props;const node=render(state);"
    );
    // When: each selected class consumes its own source assignment.
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    // Then: repeated syntax remains two ordered reads with different variable values.
    assert_eq!(
        actual,
        "[[\"pad\",\"pad\",\"id\",\"child\"],[\"padding:16px:0\",\"padding:24px:0\"]]"
    );
}

#[rstest]
#[case("0", "[\"padding:0:0\"]")]
#[case("-0", "[\"padding:0:0\"]")]
#[case("false", "[]")]
#[case("null", "[]")]
#[case("undefined", "[]")]
#[case("''", "[]")]
#[case("NaN", "[]")]
#[serial]
fn css_and_literal_value_is_selected_when_controller_is_falsy(
    #[case] controller: &str,
    #[case] expected: &str,
) {
    // Given: the same authored logical value on the css() caller path.
    let source = format!(
        "import {{css}}from '@devup-ui/react';function render(state){{return css({{p:state.a&&'16px'}})}}let reads=0;const state={{get a(){{reads++;return {controller}}}}};const node={{className:render(state)}};"
    );
    // When: the logical value is compiled into its static class selection.
    let actual = selected(&source, "JSON.stringify([reads,assigned(node)]);");
    // Then: zero selects padding zero and other absent literals select no class.
    assert_eq!(actual, format!("[1,{expected}]"));
}

#[rstest]
#[case("<Box p={state.a&&state.b} id={state.id}/>")]
#[case("jsx(Box,{p:state.a&&state.b,id:state.id})")]
#[case("<Box _hover={{p:[state.a&&state.b,null,'8px']}} id={state.id}/>")]
#[case("jsx(Box,{_hover:{p:[state.a&&state.b,null,'8px']},id:state.id})")]
#[serial]
fn zero_controller_keeps_source_order_when_jsx_runtime_and_selectors_are_used(
    #[case] expression: &str,
) {
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{get a(){{trace.push('a');return 0}},get b(){{trace.push('b');return '16px'}},get id(){{trace.push('id');return 'target'}}}};const jsx=(tag,props)=>props;const node=render(state);"
    );
    let actual = selected(&source, "JSON.stringify([trace,assigned(node)]);");
    let styles = if expression.contains("_hover") {
        "[\"padding:0:0\",\"padding:8px:2\"]"
    } else {
        "[\"padding:0:0\"]"
    };
    assert_eq!(actual, format!("[[\"a\",\"id\"],{styles}]"));
}
