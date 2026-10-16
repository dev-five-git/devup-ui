use super::*;
use serial_test::serial;

#[test]
#[serial]
fn create_element_compiles_by_binding_whatever_it_is_named() {
    let rendered = code(&format!(
        "import React, {{ createElement as make }} from 'react';\n{BOX}export const named = make(Box, {{ color: 'red' }}, 'child');\nexport const namespace = React.createElement(Box, {{ bg: 'blue' }}, 'a', 'b');"
    ));

    assert!(
        rendered.contains("make(\"div\", { className: \"color-0-red--255-a\" }, \"child\")"),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "React.createElement(\"div\", { className: \"background-0-blue--255-a\" }, \"a\", \"b\")"
        ),
        "{rendered}"
    );
    assert!(!rendered.contains("Box"), "{rendered}");
}

#[test]
#[serial]
fn create_element_through_a_namespace_import_and_require_compiles() {
    let rendered = code(&format!(
        "import * as R from 'react';\n{BOX}const React = require('react');\nconst {{ createElement: ce }} = require('react');\nexport const a = R.createElement(Box, {{ bg: 'red' }});\nexport const b = React.createElement(Box, {{ bg: 'blue' }});\nexport const c = ce(Box, {{ bg: 'green' }});"
    ));

    assert_eq!(rendered.matches("(\"div\"").count(), 3, "{rendered}");
}

#[test]
#[serial]
fn create_element_keeps_children_key_and_ref() {
    let rendered = code(&format!(
        "import {{ createElement }} from 'react';\n{BOX}export const a = (r) => createElement(Box, {{ bg: 'blue', key: 1, ref: r }}, 'a', createElement(Box));"
    ));

    assert!(rendered.contains("key: 1"), "{rendered}");
    assert!(rendered.contains("ref: r"), "{rendered}");
    assert!(
        rendered.contains("\"a\", createElement(\"div\", {})"),
        "{rendered}"
    );
}

#[test]
#[serial]
fn create_element_without_props_builds_the_element() {
    let rendered = code(&format!(
        "import {{ createElement }} from 'react';\n{BOX}export const a = createElement(Box);\nexport const b = createElement(Box, null, 'x');\nexport const c = createElement(Box, undefined);"
    ));

    assert_eq!(
        rendered.matches("createElement(\"div\", {}").count(),
        3,
        "{rendered}"
    );
}

#[test]
#[serial]
fn create_element_with_props_held_in_a_value_spreads_them() {
    let rendered = code(&format!(
        "import {{ createElement }} from 'react';\n{BOX}export const a = (p) => createElement(Box, p, 'x');\nexport const b = (f) => createElement(Box, f(), 'y');\nexport const c = createElement(Box, {{ p: 1 }} as {{ p: number }});"
    ));

    assert!(rendered.contains("...p"), "{rendered}");
    assert!(
        rendered.contains("className: __devupSpread0?.className || \"\""),
        "{rendered}"
    );
    assert!(
        rendered.contains("createElement(\"div\", ((__devupSpread"),
        "{rendered}"
    );
    assert!(
        rendered.contains("className: \"padding-0-4px--255-a\""),
        "{rendered}"
    );
}

#[test]
#[serial]
fn a_shadowed_create_element_is_not_react() {
    let message = error(&format!(
        "import {{ createElement }} from 'react';\n{BOX}export const a = (createElement) => createElement(Box, {{ color: 'red' }});"
    ));

    assert!(message.contains("`Box` is read at runtime"), "{message}");
}

#[test]
#[serial]
fn a_shadowed_react_is_not_react() {
    let message = error(&format!(
        "import React from 'react';\n{BOX}export const a = (React) => React.createElement(Box, {{ color: 'red' }});"
    ));

    assert!(message.contains("`Box` is read at runtime"), "{message}");
}

#[test]
#[serial]
fn create_element_of_anything_else_is_left_alone() {
    let rendered = code(&format!(
        "import {{ createElement }} from 'react';\n{BOX}export const a = createElement('div', {{ color: 'red' }});\nexport const b = createElement(Foo, {{ color: 'red' }});\nexport const c = <Box color=\"blue\" />;"
    ));

    assert!(
        rendered.contains("createElement(\"div\", { color: \"red\" })"),
        "{rendered}"
    );
    assert!(
        rendered.contains("createElement(Foo, { color: \"red\" })"),
        "{rendered}"
    );
}

#[test]
#[serial]
fn create_element_given_a_spread_argument_is_read_at_runtime() {
    let message = error(&format!(
        "import {{ createElement }} from 'react';\n{BOX}export const a = (args) => createElement(Box, ...args);"
    ));

    assert!(message.contains("`Box` is read at runtime"), "{message}");
}

#[test]
#[serial]
fn create_element_props_that_hold_nothing_build_an_empty_object() {
    let rendered = code(&format!(
        "import React, {{ createElement, useState }} from 'react';\n{BOX}export const a = createElement(Box, void 0);\nexport const b = React.createElement(Box, null);\nexport const c = useState(0);"
    ));

    assert_eq!(
        rendered.matches("createElement(\"div\", {})").count(),
        2,
        "{rendered}"
    );
    assert!(rendered.contains("= useState(0)"), "{rendered}");
}
