use crate::assignment_test_support::{compiled_jsx, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "<Box id={state.id} p={state.pad} className={state.cls} style={state.style} {...state.rest}>{state.child}</Box>",
    "[\"id\",\"pad\",\"cls\",\"style\",\"rest\",\"child\"]"
)]
#[case(
    "<Box style={state.style} className={state.cls} p={state.pad} id={state.id}>{state.child}</Box>",
    "[\"style\",\"cls\",\"pad\",\"id\",\"child\"]"
)]
#[case(
    "jsx(Box,{id:state.id,p:state.pad,className:state.cls,style:state.style,...state.rest,children:state.child})",
    "[\"id\",\"pad\",\"cls\",\"style\",\"rest\",\"child\"]"
)]
#[case(
    "jsxs(Box,{style:state.style,className:state.cls,p:state.pad,id:state.id,children:state.child})",
    "[\"style\",\"cls\",\"pad\",\"id\",\"child\"]"
)]
#[case(
    "jsxDEV(Box,{id:state.id,p:state.pad,className:state.cls,style:state.style,...state.rest,children:state.child})",
    "[\"id\",\"pad\",\"cls\",\"style\",\"rest\",\"child\"]"
)]
#[serial]
fn operands_keep_source_order_when_dynamic_value_is_absent(
    #[case] expression: &str,
    #[case] expected: &str,
) {
    // Given: neighboring operands expose reordering or duplicate style reads.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx,jsxs,jsx as jsxDEV}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const input={{}};for(const[key,value]of Object.entries({{id:'target',pad:null,cls:'external',style:{{color:'red'}},rest:{{title:'title'}},child:'text'}}))Object.defineProperty(input,key,{{get(){{trace.push(key);return value}}}});const jsx=(tag,props)=>props;const jsxs=jsx;const jsxDEV=jsx;const node=render(input);JSON.stringify([trace,node.className.includes('external'),node.style.color]);"
    );
    // When: both JSX and real runtime-construction extraction paths execute.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: original order, user class and user style survive.
    assert_eq!(actual, format!("[{expected},true,\"red\"]"));
}

#[rstest]
#[case("void read()")]
#[case("test()?null:null")]
#[case("test()?false:true")]
#[serial]
fn empty_effects_remain_ordered_when_no_declaration_is_admitted(
    #[case] expression: &str,
    #[values(false, true)] runtime: bool,
) {
    // Given: an absent result still has an authored call before the next operand.
    let node = if runtime {
        format!("jsx(Box,{{p:{expression},id:after()}})")
    } else {
        format!("<Box p={{{expression}}} id={{after()}}/>")
    };
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';let trace=[];function read(){{trace.push('read');return 4}}function test(){{trace.push('read');return true}}function after(){{trace.push('after');return 'id'}}const jsx=(tag,props)=>props;const node={node};JSON.stringify([trace,String(node.className??'').trim()]);"
    );
    // When: empty-effect lowering executes the original source.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: the read remains once and no declaration class appears.
    assert_eq!(actual, "[[\"read\",\"after\"],\"\"]");
}

#[test]
#[serial]
fn absent_throw_propagates_when_a_later_attribute_would_read() {
    // Given: void must not erase a throwing call.
    let source = "import {Box}from '@devup-ui/react';let trace=[];function read(){trace.push('read');throw Error('stop')}function after(){trace.push('after');return 'id'}let error;try{const node=<Box p={void read()} id={after()}/>}catch(e){error=e.message}JSON.stringify([trace,error]);";
    // When: generated props run through the original throw boundary.
    let actual = evaluate(&compiled_jsx(source));
    // Then: subsequent operands remain unexecuted.
    assert_eq!(actual, "[[\"read\"],\"stop\"]");
}

#[rstest]
#[case("null", 0)]
#[case("4", 1)]
#[serial]
fn alternate_class_order_uses_capture_when_order_follows_value(
    #[case] value: &str,
    #[case] count: usize,
) {
    // Given: a later style order prepares two class alternatives.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';let trace=[];const state={{get value(){{trace.push('value');return {value}}},get order(){{trace.push('order');return true}}}};const node=<Box p={{state.value}} styleOrder={{state.order?1:2}}/>;JSON.stringify([trace,String(node.className??'').split(/\\s+/).filter(Boolean).length]);"
    );
    // When: the selected alternate executes its shared tuple.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: neither class slot rereads the authored value.
    assert_eq!(actual, format!("[[\"value\",\"order\"],{count}]"));
}
