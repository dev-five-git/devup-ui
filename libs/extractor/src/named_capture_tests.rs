use crate::assignment_test_support::{compiled_jsx, jsx_js};
use crate::named_capture_support::evaluate;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "<Box boxSizing='border-box' className={clsx(variants[variant],isError&&variant==='default'&&errorClassNames,className)} typography={state.typo} {...state.rest}/>"
)]
#[case(
    "jsx(Box,{boxSizing:'border-box',className:clsx(variants[variant],isError&&variant==='default'&&errorClassNames,className),typography:state.typo,...state.rest})"
)]
#[case(
    "<Box className={state.combine('default',state.argument)} typography={state.typo} {...state.rest}/>"
)]
#[case(
    "jsx(Box,{className:state.combine('default',state.argument),typography:state.typo,...state.rest})"
)]
#[serial]
fn opaque_call_runs_at_authored_position_when_typography_and_spread_follow(
    #[case] expression: &str,
    #[values(false, true)] throws: bool,
) {
    // Given: the rejected clsx shape and a receiver-sensitive opaque call.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state,clsx,variants,variant,isError,errorClassNames,className){{return {expression}}}let trace=[];const jsx=(tag,props)=>props;const variants={{get default(){{trace.push('variant');return 'p-4'}}}};let state={{get argument(){{trace.push('argument');return 'arg'}},combine(a,b){{if(this!==state)throw Error('receiver');trace.push('call:'+a+':'+b);if({throws})throw Error('stop');return 'external'}},get typo(){{trace.push('typo');return 'heading'}},get rest(){{trace.push('rest');return {{id:1}}}}}};const clsx=(...args)=>{{trace.push('clsx:'+args.join('|'));if({throws})throw Error('stop');return 'external'}};let node,error;try{{node=render(state,clsx,variants,'default',true,'error','custom')}}catch(e){{error=e.message}}JSON.stringify([trace,error??null,node?.className?.includes('external')??false]);"
    );
    // When: source and compiled element expressions execute natively.
    let expected = evaluate(&format!("const Box='div';{}", jsx_js(&source)));
    let actual = evaluate(&compiled_jsx(&source));
    // Then: one intact call runs before later reads, or cuts them off on throw.
    assert_eq!(actual, expected);
}

#[rstest]
#[case("<Box typography={typography}/>")]
#[case("jsx(Box,{typography})")]
#[case("<Box title={state.before} typography={typography} color={state.color}>{state.child}</Box>")]
#[case("jsx(Box,{title:state.before,typography,color:state.color,children:state.child})")]
#[serial]
fn raw_typography_is_read_once_when_global_getter_throws_on_second_read(
    #[case] expression: &str,
    #[values("'heading'", "''", "false", "0", "null")] value: &str,
) {
    // Given: even a solitary identifier is an observable authored operand.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[],reads=0;Object.defineProperty(globalThis,'typography',{{get(){{trace.push('typo');if(++reads>1)throw Error('second');return {value}}}}});const state={{get before(){{trace.push('before');return 'title'}},get color(){{trace.push('color');return 'red'}},get child(){{trace.push('child');return 'child'}}}};const jsx=(tag,props)=>props;let node,error;try{{node=render(state)}}catch(e){{error=e.message}}"
    );
    let expected = evaluate(&format!(
        "const Box='div';{}",
        jsx_js(&format!("{source}JSON.stringify([trace,error??null]);"))
    ));
    // When: the generated truthiness/interpolation executes.
    let compiled = compiled_jsx(&source);
    let actual = evaluate(&format!("{compiled}JSON.stringify([trace,error??null]);"));
    // Then: the raw read is singular and siblings keep source order.
    assert_eq!(actual, expected, "{compiled}");
    let classes = evaluate(&format!(
        "{compiled}JSON.stringify(String(node?.className??'').split(/\\s+/).filter(v=>v.startsWith('typo-')));"
    ));
    assert_eq!(
        classes,
        if value == "'heading'" {
            "[\"typo-heading\"]"
        } else {
            "[]"
        }
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn namespace_value_keeps_its_position_when_later_values_are_captured(#[case] throws: bool) {
    // Given: xlink:href precedes handler/title/styles and children.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box xlink:href={{state.href}} onClick={{state.handler}} title={{state.title}} color={{state.color}}>{{state.child}}</Box>}}let trace=[];const state={{get href(){{trace.push('href');if({throws})throw Error('stop');return '#icon'}},get handler(){{trace.push('handler');return 1}},get title(){{trace.push('title');return 'title'}},get color(){{trace.push('color');return 'red'}},get child(){{trace.push('child');return 'child'}}}};let node,error;try{{node=render(state)}}catch(e){{error=e.message}}JSON.stringify([trace,error??null,node?.['xlink:href']??null]);"
    );
    // When: quoted namespace keys are observed without changing their values.
    let expected = evaluate(&format!("const Box='div';{}", jsx_js(&source)));
    let actual = evaluate(&compiled_jsx(&source));
    // Then: an early namespace throw prevents every subsequent operand.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn namespace_read_precedes_await_when_async_element_resumes() {
    // Given: the namespace read precedes a later awaited style and child.
    let source = "import {Box}from '@devup-ui/react';async function render(state){return <Box xlink:href={state.href} onClick={state.handler} title={state.title} color={await state.color}>{await state.child}</Box>}let trace=[];const state={get href(){trace.push('href');return '#icon'},get handler(){trace.push('handler');return 1},get title(){trace.push('title');return 'title'},get color(){trace.push('color');return Promise.resolve('red')},get child(){trace.push('child');return Promise.resolve('child')}};render(state).then(node=>JSON.stringify([trace,node['xlink:href'],node.children]),error=>JSON.stringify(['error',String(error)]));";
    // When: native promise jobs resume both source and emitted expressions.
    let expected = evaluate(&format!("const Box='div';{}", jsx_js(source)));
    let actual = evaluate(&compiled_jsx(source));
    // Then: namespace timing survives suspension at the actual invocation argument.
    assert_eq!(actual, expected);
}
