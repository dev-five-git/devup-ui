use super::*;

#[test]
#[serial]
fn capture_contract_when_jsx_rest_spread_has_getters_uses_saved_values_once() {
    let source = concat!(
        "const rest={get id(){trace.push('id');return 'a'},get className(){trace.push('class');return 'external'},get style(){trace.push('style');return {",
        "opacity:.5}}};const a=<div {...rest} css={{color:'red'}} title={(trace.push('title'),'b')}/>;"
    );
    let compiled = emotion(source).required("native Emotion rest spread must compile");
    let actual = whole::evaluate_code(&compiled.code, "a.props");
    assert_eq!(
        actual.trace,
        serde_json::json!(["id", "class", "style", "title"]),
        "{}",
        compiled.code
    );
    assert!(
        actual.element["className"]
            .as_str()
            .required("saved className must remain a string")
            .contains("external")
    );
    assert_eq!(actual.element["style"]["opacity"], 0.5);
}

#[test]
#[serial]
fn capture_contract_when_runtime_props_spread_has_getters_uses_saved_values_once() {
    let source = concat!(
        "import {jsx} from 'react/jsx-runtime';const props={get id(){trace.push('id');return 'a'},get className(){trace.push('class');return 'external'},get style(){trace.push('style');return {",
        "opacity:.5}}};const a=jsx('div',{...props,css:{color:'red'},title:(trace.push('title'),'b')});"
    );
    let compiled = emotion(source).required("native Emotion runtime props spread must compile");
    let actual = whole::evaluate_code(&compiled.code, "a.props");
    assert_eq!(
        actual.trace,
        serde_json::json!(["id", "class", "style", "title"]),
        "{}",
        compiled.code
    );
    assert!(
        actual.element["className"]
            .as_str()
            .required("saved className must remain a string")
            .contains("external")
    );
    assert_eq!(actual.element["style"]["opacity"], 0.5);
}

#[test]
#[serial]
fn capture_contract_when_styled_spread_has_getters_keeps_saved_class_and_style() {
    let source = concat!(
        "import styled from '@emotion/styled';const __devupForwardRef=render=>render;const Button=styled.button({color:'blue'});const rest={get className(){trace.push('class');return 'external'},get style(){trace.push('style');return {",
        "opacity:.5}}};const a=<Button {...rest} css={{margin:2}}/>;"
    );
    let compiled = emotion(source).required("styled Emotion spread must compile");
    let actual = whole::evaluate_code(&compiled.code, "a.props");
    assert_eq!(
        actual.trace,
        serde_json::json!(["class", "style"]),
        "{}",
        compiled.code
    );
}
