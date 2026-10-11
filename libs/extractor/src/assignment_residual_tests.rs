use crate::assignment_blocker_support::selected;
use crate::assignment_test_support::{compiled_jsx, evaluate, extracted};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box className={state.flag?'p-4':'p-8'} color={state.color}/>")]
#[case("jsx(Box,{className:state.flag?'p-4':'p-8',color:state.color})")]
#[serial]
fn literal_class_rules_are_retained_when_adjacent_assignments_are_captured(
    #[case] expression: &str,
) {
    // Given: a literal-bearing Tailwind class expression beside a dynamic style.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const input={{get flag(){{trace.push('flag');return true}},get color(){{trace.push('color');return 'red'}}}};const jsx=(tag,props)=>props;const node=render(input);"
    );
    // When: actual selected classes are associated with emitted declarations.
    let actual = selected(
        &source,
        "JSON.stringify([trace,selected(node).filter(value=>value.startsWith('padding:'))]);",
    );
    // Then: capture does not hide the literal class from compile-time extraction.
    assert_eq!(actual, "[[\"flag\",\"color\"],[\"padding:1rem:0\"]]");
}

#[rstest]
#[case(
    "css({color:'red',p:1},state.cond&&{color:'blue',_hover:{color:'green'}})",
    "[\"cond\"]"
)]
#[case(
    "css({color:'red'},state.flag?state.other:{color:'blue'})",
    "[\"flag\",\"other\"]"
)]
#[case(
    "css({color:'red'},state.cond&&state.other,state.flag?'x':null,[undefined,false])",
    "[\"cond\",\"other\",\"flag\"]"
)]
#[serial]
fn composition_reads_controls_and_classes_once_when_parts_are_conditional(
    #[case] expression: &str,
    #[case] expected: &str,
) {
    // Given: controllers and class values expose the source argument order.
    let source = format!(
        "import {{css}}from '@devup-ui/react';function render(state){{return {expression}}}let trace=[];const input={{get cond(){{trace.push('cond');return true}},get flag(){{trace.push('flag');return true}},get other(){{trace.push('other');return 'external'}}}};const node=render(input);JSON.stringify(trace);"
    );
    // When: the composed class expression is evaluated.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: a controller shared by class and rule selection is read only once.
    assert_eq!(actual, expected);
}

#[test]
#[serial]
fn selector_values_remain_distinct_when_multiple_properties_share_a_condition() {
    // Given: two dynamic properties and one logical selector clause.
    let source = "import {Box}from '@devup-ui/react';function render(state){return <Box _hover={state.flag&&{color:state.color,bg:state.background}}/>}const node=render({flag:true,color:'red',background:'blue'});";
    // When: the shared condition selects both property assignments.
    let actual = selected(source, "JSON.stringify(assigned(node));");
    // Then: the values use distinct source roles, never one overwritten variable.
    assert_eq!(actual, "[\"background:blue:0\",\"color:red:0\"]");
}

#[rstest]
#[case("({a:'red',b:'blue'})[state.key]", "missing")]
#[case("({a:'red',b:'blue'})[state.key]", "toString")]
#[case("({a:'red',b:'blue'})[state.key]", "constructor")]
#[case("({a:'red',__proto__:'blue'})[state.key]", "__proto__")]
#[case("[1,2][state.key]", "9")]
#[serial]
fn missing_and_prototype_selections_are_empty_when_no_authored_entry_applies(
    #[case] expression: &str,
    #[case] key: &str,
) {
    // Given: a static class alongside a lookup which selects no literal-defined CSS value.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box p={{1}} color={{{expression}}}/>}}const node=render({{key:'{key}'}});"
    );
    // When: selected class fragments are rendered in a template.
    let actual = selected(&source, "JSON.stringify(selected(node));");
    // Then: no undefined/prototype fragment is rendered as a class.
    assert_eq!(actual, "[\"padding:4px:0\"]");
}

#[rstest]
#[case("state.flag && state.color", false, "[0,[]]")]
#[case("state.flag ? state.color : null", false, "[0,[]]")]
#[case("state.flag ? false : state.color", true, "[0,[]]")]
#[case("state.flag && state.color", true, "[1,[\"red\"]]")]
#[case("state.flag ? state.color : null", true, "[1,[\"red\"]]")]
#[case("state.flag ? false : state.color", false, "[1,[\"red\"]]")]
#[case("(state.flag ? false : state.color) ?? 'blue'", true, "[0,[]]")]
#[case("(state.flag ? null : state.color) ?? 'blue'", true, "[1,[]]")]
#[case("(state.flag ? false : state.color) || 'blue'", true, "[1,[]]")]
#[serial]
fn no_style_branches_stay_absent_when_dynamic_alternatives_exist(
    #[case] expression: &str,
    #[case] flag: bool,
    #[case] expected: &str,
) {
    // Given: a logical/conditional branch which deliberately supplies no styles.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box color={{{expression}}}/>}}const node=render({{flag:{flag},color:'red'}});JSON.stringify([String(node.className??'').split(/\\s+/).filter(Boolean).length,Object.values(node.style??{{}})]);"
    );
    // When: class and inline value selection execute together.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: only the selected value branch carries its inline assignment.
    assert_eq!(actual, expected);
}

#[rstest]
#[case(
    "<Box className={state.className} color={state.color} style={state.style} as={state.tag} {...state.rest} title={state.title}>{state.child}</Box>"
)]
#[case(
    "jsx(Box,{className:state.className,color:state.color,style:state.style,as:state.tag,...state.rest,title:state.title,children:state.child})"
)]
#[serial]
fn adjacent_reads_remain_authored_when_class_style_type_spread_and_children_mix(
    #[case] expression: &str,
) {
    // Given: getters expose every adjacent source operand, including className and style.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const input={{}};for(const[key,value]of Object.entries({{className:'external',color:'red',style:{{}},tag:'section',rest:{{id:1}},title:'title',child:'child'}}))Object.defineProperty(input,key,{{get(){{trace.push(key);return value}}}});const jsx=(tag,props)=>props;const node=render(input);JSON.stringify(trace);"
    );
    // When: the real emitted expression executes.
    let actual = evaluate(&compiled_jsx(&source));
    // Then: no member read is called pure or moved across another read.
    assert_eq!(
        actual,
        "[\"className\",\"color\",\"style\",\"tag\",\"rest\",\"title\",\"child\"]"
    );
}

#[test]
#[serial]
fn asynchronous_nested_selector_reads_stay_lazy_when_awaits_resume() {
    // Given: a lazy selector branch with awaited responsive values and a later title.
    let source = "import {Box}from '@devup-ui/react';async function render(state){return <Box _hover={state.flag&&{boxShadow:[await state.first,null,await state.last]}} title={await state.title}/>}let trace=[];const state={flag:true};for(const[key,value]of Object.entries({first:'none',last:'none',title:'title'}))Object.defineProperty(state,key,{get(){trace.push(key);return Promise.resolve(value)}});let result;render(state).then(node=>{result=[trace,Object.values(node.style),node.title]},error=>{result=['error',String(error)]});";
    let compiled = compiled_jsx(source);
    // When: Boa's actual promise jobs resume every authored await.
    let mut context = boa_engine::Context::default();
    context
        .eval(boa_engine::Source::from_bytes(&compiled))
        .unwrap_or_else(|error| panic!("{error}\n{compiled}"));
    context.run_jobs().unwrap_or_else(|error| panic!("{error}"));
    let actual = context
        .eval(boa_engine::Source::from_bytes("JSON.stringify(result)"))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("resolved JSON"))
        .to_std_string_escaped();
    // Then: await remains outside synchronous captures and value order is preserved.
    assert_eq!(
        actual, "[[\"first\",\"last\",\"title\"],[\"none\",\"none\"],\"title\"]",
        "{compiled}"
    );
}

#[rstest]
#[case(true, Some(5), "state.flag?5:10")]
#[case(false, Some(10), "state.flag?5:10")]
#[case(true, None, "state.flag?undefined:3")]
#[case(false, Some(3), "state.flag?undefined:3")]
#[serial]
fn call_cascade_order_matches_source_when_values_are_shared(
    #[case] flag: bool,
    #[case] order: Option<u8>,
    #[case] order_expression: &str,
) {
    // Given: a JSX call with conditional orders and a dynamic CSS value.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return jsx(Box,{{styleOrder:{order_expression},color:state.color}})}}const jsx=(tag,props)=>props;const node=render({{flag:{flag},color:'red'}});"
    );
    let output = extracted(&source);
    let classes=output.styles.iter().filter(|value|matches!(value,crate::ExtractStyleValue::Dynamic(style)if style.style_order()==order)).filter_map(|value|match value.extract(None){Some(crate::extract_style::style_property::StyleProperty::Variable{class_name,..})=>Some(class_name),_=>None}).collect::<Vec<_>>();
    assert_eq!(classes.len(), 1);
    // When: the input flag selects one authored order.
    let actual = evaluate(&format!(
        "{}JSON.stringify([node.className,Object.values(node.style)]);",
        compiled_jsx(&source)
    ));
    // Then: the class carrying that CSS order uses the same captured red value.
    assert_eq!(
        actual,
        serde_json::to_string(&(classes[0].clone(), vec!["red"]))
            .unwrap_or_else(|error| panic!("{error}"))
    );
}

#[test]
#[serial]
fn generator_computed_keys_stay_after_spreads_when_yield_resumes() {
    // Given: the rejected JSX-call spread shape with a yielding computed value.
    let source = "import {Box}from '@devup-ui/react';import {jsx}from 'react/jsx-runtime';function* render(f,state){return jsx(Box,{...f(),[state.key]:yield state.value})}let trace=[];const input={get key(){trace.push('key');return 'title'},get value(){trace.push('value');return 'yielded'}};const f=()=>{trace.push('f');return {id:1}};const jsx=(tag,props)=>props;const generator=render(f,input);const first=generator.next();const last=generator.next('resumed');JSON.stringify([trace,first.value,last.value.title]);";
    // When: the actual generator is resumed.
    let actual = evaluate(&compiled_jsx(source));
    // Then: the spread runs once, followed by key/value, with yield in the authored scope.
    assert_eq!(
        actual,
        "[[\"f\",\"key\",\"value\"],\"yielded\",\"resumed\"]"
    );
}

#[test]
#[serial]
fn spread_property_getters_run_once_when_class_and_style_are_composed() {
    // Given: source object enumeration exposes className/style getters before a later title.
    let source = "import {Box}from '@devup-ui/react';function render(f,state){return <Box {...f()} title={state.title}/>}let trace=[];const input={get title(){trace.push('title');return 'later'}};const f=()=>{trace.push('f');return {get className(){trace.push('class');return 'external'},get style(){trace.push('style');return {color:'red'}}}};const node=render(f,input);JSON.stringify([trace,node.className,node.style]);";
    // When: the emitted spread and generated composition execute.
    let actual = evaluate(&compiled_jsx(source));
    // Then: enumeration getters run once in source position, not again for composition.
    assert_eq!(
        actual,
        "[[\"f\",\"class\",\"style\",\"title\"],\"external\",{\"color\":\"red\"}]"
    );
}
