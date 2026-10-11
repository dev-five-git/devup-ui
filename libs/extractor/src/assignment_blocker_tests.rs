use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    false,
    "[\"color:red:0\",\"padding:4px:0\"]",
    "[\"background:red:0\",\"margin:12px:0\"]"
)]
#[case(
    true,
    "[\"color:blue:0\",\"color:green:0\",\"padding:4px:0\"]",
    "[\"margin:8px:0\"]"
)]
#[serial]
fn composition_selects_source_rules_when_conditions_change(
    #[case] flag: bool,
    #[case] a: &str,
    #[case] b: &str,
) {
    // Given: the exact rejected compositions with distinct overridden values.
    let source = format!(
        "import {{css}} from '@devup-ui/react';function render(cond,flag){{return [css({{color:'red',p:1}},cond&&{{color:'blue',_hover:{{color:'green'}}}}),css([{{m:1}},flag?{{m:2}}:{{m:3,bg:'red'}}])]}}const nodes=render({flag},{flag});"
    );
    // When: both class strings are evaluated and matched to emitted declarations.
    let actual = selected(
        &source,
        "JSON.stringify(nodes.map(className=>selected({className})));",
    );
    // Then: each condition chooses only its authored rules.
    assert_eq!(actual, format!("[{a},{b}]"));
}

#[rstest]
#[case(true, "[\"box-shadow:none:0\",\"box-shadow:none:2\"]")]
#[case(false, "[]")]
#[serial]
fn nested_selector_keeps_branch_bindings_when_responsive_styles_are_selected(
    #[case] enabled: bool,
    #[case] expected: &str,
) {
    // Given: an AND style object with a nested responsive assignment.
    let source = format!(
        "import {{Box}} from '@devup-ui/react';function render(enabled){{return <Box _hover={{enabled&&{{boxShadow:['none',null,'none']}}}}/>}}const node=render({enabled});"
    );
    // When: the actual emitted branch executes.
    let actual = selected(&source, "JSON.stringify(selected(node));");
    // Then: the enabled branch is bound and the disabled branch is empty.
    assert_eq!(actual, expected);
}

#[rstest]
#[case("sm", false, "12px")]
#[case("md", true, "28px")]
#[case("lg", false, "20px")]
#[serial]
fn nested_shorthand_members_keep_classes_when_consumers_coalesce(
    #[case] size: &str,
    #[case] icon: bool,
    #[case] value: &str,
) {
    // Given: px's two consumers and the exact nested icon/size choices.
    let source = format!(
        "import {{Box}} from '@devup-ui/react';function render(size,icon){{return <Box px={{{{false:{{sm:'12px',md:'16px',lg:'20px'}}[size],true:{{sm:'24px',md:'28px',lg:'32px'}}[size]}}[(!!icon).toString()]}}/>}}const node=render('{size}',{icon});"
    );
    // When: the selected classes execute through both maps.
    let actual = selected(&source, "JSON.stringify(selected(node));");
    // Then: both padding sides use the source's selected value.
    assert_eq!(
        actual,
        format!("[\"padding-left:{value}:0\",\"padding-right:{value}:0\"]"),
        "{}",
        compiled_jsx(&source)
    );
}

#[rstest]
#[case("[1,0.5,...some][a]", "some:[0.25],a:2")]
#[case("[...arr,1][idx]", "arr:[0.25],idx:0")]
#[serial]
fn spread_array_selects_raw_value_when_index_is_dynamic(
    #[case] expression: &str,
    #[case] input: &str,
) {
    // Given: each distinct rejected spread-array shape selects the spread's numeric value.
    let source = format!(
        "import {{Box}} from '@devup-ui/react';function render({{some,arr,a,idx}}){{return <Box opacity={{{expression}}}/>}}const node=render({{{input}}});JSON.stringify([String(node.className??'').split(/\\s+/).filter(Boolean).length,Object.values(node.style??{{}})]);"
    );
    // When: the actual class and inline assignment are evaluated jointly.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: a dynamic class exists and receives 0.25 rather than tuple projections of a number.
    assert_eq!(actual, "[1,[0.25]]");
}

#[rstest]
#[case(
    "<Box {...f()} title={<Box as={state.tag||'b'}/>}>{state.child}</Box>",
    "[\"f\",\"tag\",\"child\"]"
)]
#[case(
    "jsx(Box,{...f(),[state.key]:state.value})",
    "[\"f\",\"key\",\"value\"]"
)]
#[case(
    "jsx(Box,{className:'z',onClick:state.click,[state.key]:2,...f(),[state.key]:1,title:'x'})",
    "[\"click\",\"key\",\"f\",\"key\"]"
)]
#[case("<Box {...{...state.rest}}/>", "[\"rest\"]")]
#[serial]
fn spread_reads_remain_once_and_ordered_when_adjacent_fields_execute(
    #[case] expression: &str,
    #[case] expected: &str,
) {
    // Given: getters expose computed keys, element type and children around spreads.
    let source = format!(
        "import {{Box}} from '@devup-ui/react';import {{jsx}} from 'react/jsx-runtime';function render(state,f){{return {expression}}}let trace=[];const state={{}};for(const [key,value]of Object.entries({{tag:'b',child:'child',key:'x',value:3,click:'handler',rest:{{id:1}}}}))Object.defineProperty(state,key,{{get(){{trace.push(key);return value}}}});const f=()=>{{trace.push('f');return {{id:2}}}};const jsx=(tag,props)=>props;const node=render(state,f);JSON.stringify(trace);"
    );
    // When: emitted full props execute in Boa.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: source order and count, including both authored key reads, remain exact.
    assert_eq!(actual, expected);
}

#[rstest]
#[case("({lg:'buttonLg',md:'button',sm:'buttonSm',tag:'tag'}as const)[state.size]")]
#[case("({lg:'buttonLg',md:'button',sm:'buttonSm',tag:'tag'})[state.size]")]
#[serial]
fn typography_reads_stay_before_children_when_literal_maps_are_selected(#[case] expression: &str) {
    // Given: both rejected typography forms and observable key/child accessors.
    let source = format!(
        "import {{Text}} from '@devup-ui/react';function render(state){{return <Text typography={{{expression}}}>{{state.child}}</Text>}}let trace=[];const state={{get size(){{trace.push('size');return 'sm'}},get child(){{trace.push('child');return 'text'}}}};const node=render(state);JSON.stringify(trace);"
    );
    // When: emitted attributes and children execute.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: the source's lookup precedes its child.
    assert_eq!(actual, "[\"size\",\"child\"]");
}

#[test]
#[serial]
fn jsx_call_reads_order_before_background_when_controller_changes() {
    // Given: consecutive controller reads differ, so reversing them changes both outputs.
    let source = "import {Box}from '@devup-ui/react';import {jsx}from 'react/jsx-runtime';function render(state){return jsx(Box,{styleOrder:state.active?5:10,bg:state.active?'red':'blue'})}let count=0;const state={get active(){return ++count===1}};const jsx=(tag,props)=>props;const node=render(state);";
    // When: actual emitted classes select an order and a value.
    let actual = selected(source, "JSON.stringify([count,selected(node)]);");
    // Then: the second read selects blue while the first selects order 5 (order asserted separately).
    assert_eq!(actual, "[2,[\"background:blue:0\"]]");
}
