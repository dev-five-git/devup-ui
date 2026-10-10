use crate::{
    ExtractStyleValue,
    assignment_test_support::{evaluate, extracted, jsx_js},
    extract_style::style_property::StyleProperty,
};
use rstest::rstest;
use serial_test::serial;

#[path = "typography_literal_lookup_tests.rs"]
mod literal_lookup;

#[rstest]
#[case((true, true))]
#[case((true, false))]
#[case((false, true))]
#[case((false, false))]
#[serial]
fn typography_operand_finishes_before_adjacent_fields_when_order_classes_are_captured(
    #[case] selection: (bool, bool),
    #[values((false, false, false), (false, false, true), (false, true, false), (false, true, true), (true, false, false), (true, false, true), (true, true, false), (true, true, true))]
    topology: (bool, bool, bool),
    #[values(false, true)] array: bool,
) {
    let (choice, order) = selection;
    let (runtime, class_first, scoped) = topology;
    // Given: distinct real presets, lazy branch getters, and a later mutating getter.
    let _theme = super::super::TypographyTheme::register();
    let operand = if array {
        "[state.flag?values.heading():values.body,values.heading()]"
    } else {
        "state.flag?[values.heading(),values.body]:[values.body,values.heading()]"
    };
    let typography = if scoped {
        format!("{{typography:{operand}}}")
    } else {
        operand.to_string()
    };
    let key = if scoped { "_hover" } else { "typography" };
    let fields = if class_first {
        vec![
            ("className", "state.name"),
            ("styleOrder", "state.order?1:2"),
            (key, typography.as_str()),
            ("title", "state.later"),
        ]
    } else {
        vec![
            (key, typography.as_str()),
            ("title", "state.later"),
            ("className", "state.name"),
            ("styleOrder", "state.order?1:2"),
        ]
    };
    let view = if runtime {
        format!(
            "jsx(Box,{{{}}})",
            fields
                .iter()
                .map(|(key, value)| format!("{key}:{value}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    } else {
        format!(
            "<Box {}/>",
            fields
                .iter()
                .map(|(key, value)| format!("{key}={{{value}}}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {view}}}"
    );
    let output = extracted(&source);
    let records = output
        .styles
        .iter()
        .filter_map(|value| match (value, value.extract(None)?) {
            (ExtractStyleValue::Static(style), StyleProperty::ClassName(class)) => Some((
                class,
                style.value.clone(),
                style.level,
                style.style_order,
                style.selector.as_ref().map(ToString::to_string),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    let records =
        serde_json::to_string(&records).unwrap_or_else(|error| panic!("records: {error}"));
    let setup = format!(
        "const trace=[];let changed=false;const values={{heading(){{trace.push('heading');return changed?'body':'heading'}},get body(){{trace.push('body');return changed?'heading':'body'}}}};const state={{get flag(){{trace.push('flag');return {choice}}},get order(){{trace.push('order');return {order}}},get name(){{trace.push('name');return 'external'}},get later(){{trace.push('later');changed=true;return 'later'}}}};"
    );
    // When: the genuine emitted expression executes and selected classes join actual records.
    let actual = evaluate(&format!(
        r"{setup}function jsx(tag,props){{return props}}{}const node=render(state);const records={records};JSON.stringify([trace,String(node.className).split(/\s+/).filter(name=>name&&name!=='external').map(name=>{{if(name.startsWith('typo-'))return [name.slice(5),0,0,null];const record=records.find(([key])=>key===name);if(!record)throw Error('missing '+name);return record.slice(1)}}).sort((a,b)=>a[1]-b[1]),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: branch calls/getters finish once, before later fields can mutate their values.
    let mut trace = if class_first {
        vec!["name", "order"]
    } else {
        vec![]
    };
    trace.extend(if array {
        vec!["flag", if choice { "heading" } else { "body" }, "heading"]
    } else if choice {
        vec!["flag", "heading", "body"]
    } else {
        vec!["flag", "body", "heading"]
    });
    trace.push("later");
    if !class_first {
        trace.extend(["name", "order"]);
    }
    let selected_order = if order { 1 } else { 2 };
    let selector = scoped.then_some("&:hover");
    let presets = if array {
        [if choice { "heading" } else { "body" }, "heading"]
    } else if choice {
        ["heading", "body"]
    } else {
        ["body", "heading"]
    };
    let expected = serde_json::json!([
        trace,
        [
            [
                presets[0],
                0,
                if scoped { selected_order } else { 0 },
                selector
            ],
            [presets[1], 1, selected_order, selector]
        ],
        []
    ]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("observation: {error}")),
        expected,
        "{}",
        output.code
    );
    assert!(
        output
            .styles
            .iter()
            .all(|value| !matches!(value, ExtractStyleValue::Dynamic(_)))
    );
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn selected_typography_throw_stops_before_later_fields_when_classname_is_adjacent(
    #[case] runtime: bool,
) {
    // Given: a selected throwing call and an unselected missing binding.
    let _theme = super::super::TypographyTheme::register();
    let view = if runtime {
        "jsx(Box,{className:state.name,typography:state.flag?[fail(),'body']:[absent,'heading'],title:state.later,styleOrder:state.order?1:2})"
    } else {
        "<Box className={state.name} typography={state.flag?[fail(),'body']:[absent,'heading']} title={state.later} styleOrder={state.order?1:2}/>"
    };
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {view}}}"
    );
    let output = extracted(&source);
    // When: the selected operand throws through the emitted expression.
    let actual = evaluate(&format!(
        "function jsx(tag,props){{return props}}{}const trace=[];const state={{get name(){{trace.push('name');return 'external'}},get flag(){{trace.push('flag');return true}},get later(){{trace.push('later');return 'later'}},get order(){{trace.push('order');return true}}}};function fail(){{trace.push('fail');throw Error('chosen')}}let error;try{{render(state)}}catch(caught){{error=caught.message}}let absent;JSON.stringify([trace,error]);",
        jsx_js(&output.code)
    ));
    // Then: neither the unselected binding nor any later field was evaluated.
    assert_eq!(actual, "[[\"name\",\"flag\",\"fail\"],\"chosen\"]");
}
