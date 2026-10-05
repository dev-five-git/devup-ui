use crate::{
    ExtractStyleValue,
    assignment_test_support::{extracted, jsx_js},
    extract_style::style_property::StyleProperty,
    named_capture_support::evaluate,
};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(("`p-4 ${state.opaque('default')}`", "1rem", true))]
#[case(("state.flag?`p-4 ${state.opaque('default')}`:`p-8 ${state.opaque('default')}`", "1rem", true))]
#[case(("state.flag?`p-4 ${state.other()}`:`p-8 ${state.opaque('default')}`", "2rem", false))]
#[case(("state.left||`p-4 ${state.opaque('default')}`", "1rem", true))]
#[case(("state.left??`p-4 ${state.opaque('default')}`", "1rem", true))]
#[case(("state.flag&&`p-4 ${state.opaque('default')}`", "1rem", true))]
#[serial]
fn prepared_tailwind_variants_share_source_evaluation_when_order_is_conditional(
    #[case] fixture: (&str, &str, bool),
    #[values(false, true)] caller: bool,
    #[values((false, false), (false, true), (true, false), (true, true))] order: (bool, bool),
) {
    // Given: actual supported Tailwind surrounds an opaque call; order may be later.
    let (class, value, flag) = fixture;
    let (order_first, order_flag) = order;
    let order = "state.order?5:10";
    let attrs = match (caller, order_first) {
        (false, false) => format!(
            "className={{{class}}} typography={{state.typo}} styleOrder={{{order}}} color={{state.color}}"
        ),
        (false, true) => format!(
            "styleOrder={{{order}}} className={{{class}}} typography={{state.typo}} color={{state.color}}"
        ),
        (true, false) => {
            format!("className:{class},typography:state.typo,styleOrder:{order},color:state.color")
        }
        (true, true) => {
            format!("styleOrder:{order},className:{class},typography:state.typo,color:state.color")
        }
    };
    let expression = if caller {
        format!("jsx(Box,{{{attrs}}})")
    } else {
        format!("<Box {attrs}/>")
    };
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {expression}}}let trace=[];const state={{get flag(){{trace.push('flag');return {flag}}},get left(){{trace.push('left');return null}},get order(){{trace.push('order');return {order_flag}}},opaque(arg){{if(this!==state)throw Error('receiver');trace.push('opaque:'+arg);return {{toString(){{trace.push('coerce');return 'external'}}}}}},other(){{throw Error('unselected')}},get typo(){{trace.push('typo');return 'heading'}},get color(){{trace.push('color');return 'red'}}}};const jsx=(tag,props)=>props;const node=render(state);"
    );
    let expected_trace = evaluate(&format!(
        "const Box='div';{}",
        jsx_js(&format!("{source}JSON.stringify(trace);"))
    ));
    let output = extracted(&source);
    let records = output
        .styles
        .iter()
        .filter_map(|style| match (style, style.extract(None)) {
            (ExtractStyleValue::Static(style), Some(StyleProperty::ClassName(class)))
                if style.property == "padding" && style.value == value =>
            {
                Some((class, style.style_order))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let records = serde_json::to_string(&records).unwrap_or_else(|error| panic!("{error}"));
    // When: the generated class selects one of both compile-time prepared orders.
    let actual = evaluate(&format!(
        "{} const records={records};JSON.stringify([trace,String(node.className).split(/\\s+/).flatMap(name=>records.filter(([key])=>key===name).map(([,order])=>order)),node.className.includes('external')]);",
        jsx_js(&output.code)
    ));
    // Then: actual declaration order matches the captured later operand, no early read.
    let selected_order = if order_flag { 5 } else { 10 };
    assert_eq!(
        actual,
        format!("[{expected_trace},[{selected_order}],true]"),
        "{}",
        output.code
    );
}

#[rstest]
#[case("state.flag?state.opaque('default'):'p-8'", true, "null")]
#[case("state.flag?state.opaque('default'):'p-8'", false, "null")]
#[case("state.left&&`p-4 ${state.opaque('default')}`", true, "0")]
#[case("state.left||`p-4 ${state.opaque('default')}`", true, "'external'")]
#[case("state.left??`p-4 ${state.opaque('default')}`", true, "false")]
#[case(
    "`p-4 ${state.left||state.opaque('default')} ${state.last()}`",
    true,
    "({toString(){trace.push('left-coerce');return 'external'}})"
)]
#[case(
    "`p-4 ${state.left&&state.opaque('default')} ${state.last()}`",
    true,
    "({toString(){throw Error('unselected coercion')}})"
)]
#[case(
    "`p-4 ${state.left} ${state.last()}`",
    true,
    "({toString(){trace.push('coerce-throw');throw Error('stop')}})"
)]
#[serial]
fn class_controls_remain_lazy_when_raw_truthiness_and_coercion_differ(
    #[case] class: &str,
    #[case] flag: bool,
    #[case] left: &str,
) {
    // Given: raw left values distinguish truthiness, nullishness and string conversion.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function render(state){{return <Box className={{{class}}} styleOrder={{state.order?5:10}} color={{state.color}}/>}}let trace=[];const state={{get flag(){{trace.push('flag');return {flag}}},get left(){{trace.push('left');return {left}}},opaque(arg){{trace.push('opaque:'+arg);return {{toString(){{trace.push('coerce');return 'external'}}}}}},last(){{trace.push('last');return 'last'}},get order(){{trace.push('order');return false}},get color(){{trace.push('color');return 'red'}}}};let node,error;try{{node=render(state);String(node.className)}}catch(e){{error=e.message}}JSON.stringify([trace,error??null]);"
    );
    // When: the source and compiled supported grammar execute with observable coercion.
    let expected = evaluate(&format!("const Box='div';{}", jsx_js(&source)));
    let actual = evaluate(&crate::assignment_test_support::compiled_jsx(&source));
    // Then: only selected branches coerce, once, before subsequent interpolation/order.
    assert_eq!(actual, expected);
}

#[rstest]
#[serial]
fn class_yield_stays_in_lazy_branch_when_generator_resumes(#[values(false, true)] flag: bool) {
    // Given: a later order cannot be used inside the class argument, and yield is lazy.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';function* render(state){{return <Box className={{state.flag?`p-4 ${{yield state.value}}`:'p-8'}} styleOrder={{state.order?5:10}} color={{state.color}}/>}}let trace=[];const state={{get flag(){{trace.push('flag');return {flag}}},get value(){{trace.push('value');return 'pause'}},get order(){{trace.push('order');return false}},get color(){{trace.push('color');return 'red'}}}};const generator=render(state);const first=generator.next();const last=generator.next('external');JSON.stringify([trace,first.done,first.done?null:first.value,last.done]);"
    );
    // When: native generator execution resumes a captured interpolation.
    let expected = evaluate(&format!("const Box='div';{}", jsx_js(&source)));
    let actual = evaluate(&crate::assignment_test_support::compiled_jsx(&source));
    // Then: yield remains in its original scope, before the later order read.
    assert_eq!(actual, expected);
}

#[rstest]
#[serial]
fn class_await_stays_lazy_when_a_later_order_operand_is_captured(
    #[values(false, true)] flag: bool,
) {
    // Given: a selected await and coercion precede a later interpolation and order.
    let source = format!(
        "import {{Box}}from '@devup-ui/react';async function render(state){{return <Box className={{state.flag?`p-4 ${{await state.value}} ${{state.last()}}`:'p-8'}} styleOrder={{state.order?5:10}} color={{state.color}}/>}}let trace=[];const state={{get flag(){{trace.push('flag');return {flag}}},get value(){{trace.push('value');return Promise.resolve({{toString(){{trace.push('coerce');return 'external'}}}})}},last(){{trace.push('last');return 'last'}},get order(){{trace.push('order');return false}},get color(){{trace.push('color');return 'red'}}}};render(state).then(node=>JSON.stringify(trace),error=>JSON.stringify(['error',String(error)]));"
    );
    // When: native promises resume the original lazy branch.
    let expected = evaluate(&format!("const Box='div';{}", jsx_js(&source)));
    let actual = evaluate(&crate::assignment_test_support::compiled_jsx(&source));
    // Then: no await moves into a new synchronous arrow body or crosses the later order.
    assert_eq!(actual, expected);
}
