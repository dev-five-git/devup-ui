use super::*;
use serial_test::serial;

const CREATE_ELEMENT: &str = "import { createElement } from 'react';\n";

#[test]
#[serial]
fn a_call_gives_way_to_a_spread_as_an_element_does() {
    let output = output(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (rest) => jsx(Box, {{ color: 'red', ...rest }});\nexport const b = (rest) => jsx(Box, {{ ...rest, color: 'red' }});"
    ));
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fallback, "red");
    assert_eq!(
        read(&found[0].identifier, "const rest = { color: 'blue' };"),
        "\"blue\""
    );
    assert_eq!(read(&found[0].identifier, "const rest = {};"), "undefined");
    assert_eq!(static_styles(&output).len(), 1);
}

#[test]
#[serial]
fn a_call_gives_the_last_own_key_of_repeated_spreads_and_the_final_explicit_style() {
    let output = output(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (first, last) => jsx(Box, {{ color: 'red', ...first, color: 'blue', ...last }});"
    ));
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fallback, "blue");
    let both = |first: &str, last: &str| format!("const first = {first}; const last = {last};");
    assert_eq!(
        read(&found[0].identifier, &both("{ color: 'x' }", "{}")),
        "undefined"
    );
    assert_eq!(
        read(&found[0].identifier, &both("{}", "{ color: undefined }")),
        "undefined"
    );
    assert_eq!(
        read(&found[0].identifier, &both("{}", "{ color: 'y' }")),
        "\"y\""
    );
}

#[test]
#[serial]
fn a_call_keeps_the_explicit_style_written_after_its_spread_without_a_slot() {
    let output = output(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (rest) => jsx(Box, {{ color: 'red', ...rest, color: 'blue' }});"
    ));

    assert_eq!(slots(&output).len(), 0);
    assert_eq!(
        static_styles(&output),
        vec![("color".to_string(), "blue".to_string())]
    );
}

#[test]
#[serial]
fn a_call_replaces_an_alias_written_earlier_in_the_props() {
    let output = output(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (rest) => jsx(Box, {{ bg: 'red', background: 'blue', ...rest }});"
    ));
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fallback, "blue");
}

#[test]
#[serial]
fn a_call_with_a_runtime_value_does_not_bring_it_back_over_a_later_own_undefined() {
    let output = output(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (rest, x) => jsx(Box, {{ p: x, ...rest }});"
    ));
    let identifier = output
        .styles
        .iter()
        .find_map(|style| match style {
            ExtractStyleValue::Dynamic(dynamic) => {
                Some(captures::materialized(dynamic.identifier(), &output.code))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("a dynamic style: {}", output.code));

    assert_eq!(
        read(&identifier, "const x = 5; const rest = { p: undefined };"),
        "undefined"
    );
    assert_eq!(read(&identifier, "const x = 5; const rest = {};"), "5");
}

#[test]
#[serial]
fn a_call_gives_way_with_the_keys_of_a_literal_spread_before_an_unknown_one() {
    let output = output(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (rest) => jsx(Box, {{ ...{{ color: 'red' }}, ...rest }});"
    ));
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fallback, "red");
}

#[test]
#[serial]
fn a_call_reads_its_spread_once() {
    let output = output(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (f) => jsx(Box, {{ color: 'red', ...f() }});"
    ));

    assert_eq!(output.code.matches("f()").count(), 1, "{}", output.code);
    assert!(output.code.contains("__devupSpread0"), "{}", output.code);
}

#[test]
#[serial]
fn a_call_with_a_selector_object_before_a_spread_is_an_error() {
    let message = error(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (rest) => jsx(Box, {{ _hover: {{ bg: 'red' }}, ...rest }});"
    ));

    assert!(message.contains("cannot use `_hover`"), "{message}");
}

#[test]
#[serial]
fn a_call_keeps_the_props_it_does_not_compile() {
    let rendered = code(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (rest) => jsx(Box, {{ as: 'a', id: 'x', styleOrder: 3, color: 'red', ...rest }});"
    ));

    assert!(rendered.contains("jsx(\"a\""), "{rendered}");
    assert!(rendered.contains("id: \"x\""), "{rendered}");
}

#[test]
#[serial]
fn a_key_that_is_not_an_identifier_is_read_by_name_in_a_call() {
    let rendered = code(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (rest) => jsx(Box, {{ 'background-color': 'red', ...rest }});"
    ));

    assert!(rendered.contains("[\"background-color\"]"), "{rendered}");
}

#[test]
#[serial]
fn a_call_with_suspending_props_captures_them_in_the_caller() {
    let rendered = code(&format!(
        "{BOX}{JSX_RUNTIME}export const a = async (k, f, g) => jsx(Box, {{ color: 'red', [k]: await g(), ...f() }});"
    ));

    assert_eq!(rendered.matches("f()").count(), 1, "{rendered}");
    assert!(rendered.contains("await g()"), "{rendered}");
}

#[test]
#[serial]
fn create_element_props_give_way_to_a_spread_after_an_explicit_style() {
    let output = output(&format!(
        "{CREATE_ELEMENT}{BOX}export const a = (rest) => createElement(Box, {{ color: 'red', ...rest }});"
    ));
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(
        read(&found[0].identifier, "const rest = { color: 'blue' };"),
        "\"blue\""
    );
}

#[test]
#[serial]
fn create_element_gives_the_final_explicit_style_over_repeated_spreads() {
    let output = output(&format!(
        "{CREATE_ELEMENT}{BOX}export const a = (first, last) => createElement(Box, {{ color: 'red', ...first, color: 'blue', ...last }}, 'child');"
    ));
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fallback, "blue");
    assert!(output.code.contains("\"child\""), "{}", output.code);
}

#[test]
#[serial]
fn create_element_with_props_held_in_a_value_gives_way_to_them() {
    let output = output(&format!(
        "{CREATE_ELEMENT}{BOX}export const a = (p) => createElement(Box, p, 'x');"
    ));

    assert!(output.code.contains("...p"), "{}", output.code);
    assert!(output.code.contains("?.className"), "{}", output.code);
}
