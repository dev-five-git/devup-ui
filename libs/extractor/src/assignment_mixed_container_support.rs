use crate::assignment_test_support::{evaluate, extracted, jsx_js};
use crate::{ExtractStyleValue, extract_style::style_property::StyleProperty};

#[test]
#[serial_test::serial]
fn unrelated_typography_record_does_not_change_selected_selector_association() {
    // Given
    let source = "import {Box,css}from '@devup-ui/react';const unused=()=>css({typography:'heading'});let trace=[];const node=<Box _hover={{color:'red'}}/>;";
    assert!(
        extracted(source)
            .styles
            .iter()
            .any(|value| matches!(value, ExtractStyleValue::Typography(name) if name == "heading"))
    );
    // When
    let actual = associated(source);
    // Then
    assert_eq!(actual, "[\"&:hover|color:red:0|\"]");
}

pub(super) fn associated(source: &str) -> String {
    let output = extracted(source);
    let records = output
        .styles
        .iter()
        .filter_map(|value| {
            let (class, property, value, level, selector, layer) =
                match (value, value.extract(None)?) {
                    (ExtractStyleValue::Static(value), StyleProperty::ClassName(class)) => (
                        class,
                        value.property.clone(),
                        value.value.clone(),
                        value.level,
                        value.selector.as_ref(),
                        value.layer.as_deref(),
                    ),
                    (
                        ExtractStyleValue::Dynamic(value),
                        StyleProperty::Variable {
                            class_name,
                            variable_name,
                            ..
                        },
                    ) => (
                        class_name,
                        value.property().to_string(),
                        format!("var({variable_name})"),
                        value.level(),
                        value.selector(),
                        value.layer(),
                    ),
                    _ => return None,
                };
            Some((
                class,
                format!(
                    "{}|{property}:{value}:{level}|{}",
                    selector.map(ToString::to_string).unwrap_or_default(),
                    layer.unwrap_or_default()
                ),
            ))
        })
        .collect::<Vec<_>>();
    let records = serde_json::to_string(&records).unwrap_or_else(|error| panic!("{error}"));
    let original = source_trace(source);
    evaluate(&format!(
        "{}const records={records};const classes=String(node.className??'').split(/\\s+/).filter(Boolean);const assigned=classes.map(name=>{{const record=records.find(([key])=>key===name);if(!record)throw Error('unbound class '+name);return record[1].replace(/var\\(([^)]+)\\)/g,(_,variable)=>String(node.style[variable]))}}).sort();if(JSON.stringify(trace)!=={})throw Error('source trace mismatch '+JSON.stringify(trace));JSON.stringify(assigned);",
        jsx_js(&output.code),
        serde_json::to_string(&original).unwrap_or_else(|error| panic!("{error}"))
    ))
}

pub(super) fn source_trace(source: &str) -> String {
    evaluate(&format!(
        "const Box='div';{}JSON.stringify(trace);",
        jsx_js(source)
    ))
}
