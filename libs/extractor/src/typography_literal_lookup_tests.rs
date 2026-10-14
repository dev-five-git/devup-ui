use crate::{
    ExtractStyleValue,
    assignment_test_support::{evaluate, extracted, jsx_js},
};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case((true, true))]
#[case((true, false))]
#[case((false, true))]
#[case((false, false))]
#[serial]
fn literal_base_lookup_keeps_scalar_output_when_adjacent_reads_are_observable(
    #[case] selection: (bool, bool),
    #[values((false, false), (false, true), (true, false), (true, true))] topology: (bool, bool),
    #[values(false, true)] conditional: bool,
) {
    // Given: inert table entries, independently observable selection and later fields.
    let _theme = super::super::super::TypographyTheme::register();
    let (choice, heading) = selection;
    let (runtime, class_first) = topology;
    let operand = if conditional {
        "state.flag?({z:'body',a:'heading'} as const)[state.key]:undefined"
    } else {
        "({z:'body',a:'heading'} as const)[state.key]"
    };
    let fields = if class_first {
        vec![
            ("className", "state.name"),
            ("typography", operand),
            ("title", "state.later"),
        ]
    } else {
        vec![
            ("typography", operand),
            ("title", "state.later"),
            ("className", "state.name"),
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
    let output = extracted(&format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {view}}}"
    ));
    let records = output
        .styles
        .iter()
        .map(|value| match value {
            ExtractStyleValue::Typography(name) => (format!("typo-{name}"), name.clone()),
            value => panic!("base-only lookup emitted a CSS atom: {value:?}"),
        })
        .collect::<Vec<_>>();
    let records =
        serde_json::to_string(&records).unwrap_or_else(|error| panic!("records: {error}"));
    // When: generated classes join the actual base-preset records at the native boundary.
    let actual = evaluate(&format!(
        r"function jsx(tag,props){{return props}}{}const trace=[];let changed=false;const state={{get flag(){{trace.push('flag');return {choice}}},get key(){{trace.push('key');return changed?'absent':{} }},get name(){{trace.push('name');return 'external'}},get later(){{trace.push('later');changed=true;return 'later'}}}};const node=render(state);const records={records};JSON.stringify([trace,String(node.className).split(/\s+/).filter(name=>name&&name!=='external').map(name=>{{const record=records.find(([key])=>key===name);if(!record)throw Error('missing '+name);return record[1]}}),Object.hasOwn(node,'style')]);",
        jsx_js(&output.code),
        if heading { "'a'" } else { "'z'" }
    ));
    // Then: no empty inline style is introduced, and the key/condition is read once in place.
    let mut trace = if class_first { vec!["name"] } else { vec![] };
    if conditional {
        trace.push("flag");
    }
    let selected = !conditional || choice;
    if selected {
        trace.push("key");
    }
    trace.push("later");
    if !class_first {
        trace.push("name");
    }
    let presets = if selected {
        vec![if heading { "heading" } else { "body" }]
    } else {
        vec![]
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("observation: {error}")),
        serde_json::json!([trace, presets, false]),
        "{}",
        output.code
    );
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn dynamic_lookup_fields_finish_in_authored_order_when_a_later_field_mutates_values(
    #[case] heading: bool,
    #[values(false, true)] runtime: bool,
) {
    // Given: nonliteral table fields must all run in authored order, not sorted map order.
    let _theme = super::super::super::TypographyTheme::register();
    let operand = "({z:values.body,a:values.heading()})[state.key]";
    let view = if runtime {
        format!("jsx(Box,{{typography:{operand},title:state.later,className:state.name}})")
    } else {
        format!("<Box typography={{{operand}}} title={{state.later}} className={{state.name}}/>")
    };
    let output = extracted(&format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {view}}}"
    ));
    // When: a getter and call produce different values if deferred past the later field.
    let actual = evaluate(&format!(
        r"function jsx(tag,props){{return props}}{}const trace=[];let changed=false;const values={{get body(){{trace.push('body');return changed?'heading':'body'}},heading(){{trace.push('heading');return changed?'body':'heading'}}}};const state={{get key(){{trace.push('key');return {} }},get later(){{trace.push('later');changed=true;return 'later'}},get name(){{trace.push('name');return 'external'}}}};const node=render(state);JSON.stringify([trace,String(node.className).split(/\s+/).filter(name=>name&&name!=='external'),Object.entries(node.style??{{}})]);",
        jsx_js(&output.code),
        if heading { "'a'" } else { "'z'" }
    ));
    // Then: both fields run once before the key and mutation, including the unselected entry.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("observation: {error}")),
        serde_json::json!([
            ["body", "heading", "key", "later", "name"],
            [if heading { "typo-heading" } else { "typo-body" }],
            []
        ]),
        "{}",
        output.code
    );
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn folded_nested_lookup_keeps_its_controller_when_class_tables_are_identical(
    #[case] inner: bool,
    #[values(false, true)] runtime: bool,
) {
    // Given: different authored field orders generate identical inert class tables.
    let _theme = super::super::super::TypographyTheme::register();
    let operand = "state.outer?(state.inner?({a:'heading',z:'body'})[state.key]:({z:'body',a:'heading'})[state.key]):'body'";
    let view = if runtime {
        format!("jsx(Box,{{typography:{operand},title:state.later}})")
    } else {
        format!("<Box typography={{{operand}}} title={{state.later}}/>")
    };
    let output = extracted(&format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {view}}}"
    ));
    // When: the inner controller is observable even though its class result is invariant.
    let actual = evaluate(&format!(
        "function jsx(tag,props){{return props}}{}const trace=[];const state={{get outer(){{trace.push('outer');return true}},get inner(){{trace.push('inner');return {inner}}},get key(){{trace.push('key');return 'a'}},get later(){{trace.push('later');return 'later'}}}};const node=render(state);JSON.stringify([trace,node.className,Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: a scalar projection cannot silently erase a nested controller read.
    assert_eq!(
        actual, "[[\"outer\",\"inner\",\"key\",\"later\"],\"typo-heading\",[]]",
        "{}",
        output.code
    );
}

#[rstest]
#[case(true)]
#[case(false)]
#[serial]
fn folded_nested_literal_keeps_its_controller_when_an_outer_selection_remains(
    #[case] runtime: bool,
) {
    // Given: only the inner equal-class selection folds, leaving a nonliteral outer class.
    let _theme = super::super::super::TypographyTheme::register();
    let operand = "state.outer?(state.inner?'heading':('heading' as const)):'body'";
    let view = if runtime {
        format!("jsx(Box,{{typography:{operand},title:state.later}})")
    } else {
        format!("<Box typography={{{operand}}} title={{state.later}}/>")
    };
    let output = extracted(&format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(state){{return {view}}}"
    ));
    // When: the generated expression selects the branch with the folded controller.
    let actual = evaluate(&format!(
        "function jsx(tag,props){{return props}}{}const trace=[];const state={{get outer(){{trace.push('outer');return true}},get inner(){{trace.push('inner');return false}},get later(){{trace.push('later');return 'later'}}}};const node=render(state);JSON.stringify([trace,node.className,Object.entries(node.style??{{}})]);",
        jsx_js(&output.code)
    ));
    // Then: the observable inner read still occurs once before the later field.
    assert_eq!(
        actual, "[[\"outer\",\"inner\",\"later\"],\"typo-heading\",[]]",
        "{}",
        output.code
    );
}
