use super::*;
use serial_test::serial;

#[test]
#[serial]
fn global_statement_when_nested_preserves_wrapper_order() {
    // Given: grouping wrappers around a statement and an ordered sibling.
    let source = "import {globalCss} from '@devup-ui/react';globalCss`@media print{@supports(display:grid){@import 'inside.css';body{style-order:2;color:red}}}`;";
    // When: the public global route compiles.
    let actual = output(source);
    // Then: statement wrappers retain their original nesting, metadata is not raw CSS.
    let raw: String = actual
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Css(css) => Some(css.css.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        raw.contains("@media print{@supports(display:grid){@import 'inside.css';}}"),
        "{raw}"
    );
    assert!(!raw.contains("style-order"));
    assert!(actual.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.value()=="red" && style.style_order()==Some(2))));
}

#[test]
#[serial]
fn opaque_global_when_selector_is_named_style_order_preserves_descriptors() {
    // Given: selector spelling is not declaration metadata.
    let source = concat!(
        "import {globalCss} from '@devup-ui/react';",
        "globalCss`@media print{@page{size:A4;styleOrder{color:red}}@font-face{font-family:test;src:url(test.woff)}}body{style-order:2;color:blue}`;"
    );
    // When: public extraction separates opaque blocks and global atoms.
    let actual = output(source);
    // Then: descriptor and selector records are kept alongside the ordered atom.
    let raw: String = actual
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Css(css) => Some(css.css.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        raw.contains("size:A4;")
            && raw.contains("styleOrder{color:red;}")
            && raw.contains("font-family:test;"),
        "{raw}"
    );
    assert!(actual.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.value()=="blue" && style.style_order()==Some(2))));
}

#[test]
#[serial]
fn global_when_layer_body_is_scalar_extracts_order_and_layer() {
    // Given: recursion receives literal text, rather than a prelowered top-level tag.
    let source = "import {globalCss} from '@devup-ui/react';globalCss({'@layer named':'body{style-order:2;color:red}'});";
    // When: public extraction recurses into the scalar body.
    let actual = output(source);
    // Then: the atom belongs to the named layer at authored order 2.
    assert!(actual.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property()=="color" && style.value()=="red" && style.style_order()==Some(2) && style.layer()==Some("named"))));
}

#[test]
#[serial]
fn global_when_root_order_is_runtime_rejects_authored_controller() {
    // Given: valid but distinct orders have no runtime global class selector.
    let source = "import {globalCss} from '@devup-ui/react';\nconst a=(on)=>globalCss`style-order:${on?2:3};body{color:red}`;";
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .rfind("on?")
        .required("controller")
        + 1;
    // When: global extraction cannot statically select the root order.
    let actual = error(source);
    // Then: the original controller owns the error.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
    assert!(
        actual.contains("global styles require a static order"),
        "{actual}"
    );
}

#[test]
#[serial]
fn global_when_value_is_runtime_rejects_original_template() {
    // Given: fixed-global text cannot host an element CSS variable.
    let source = "import {globalCss} from '@devup-ui/react';\nconst a=(value)=>globalCss`body{style-order:2;color:${value}}`;";
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .find('`')
        .required("template")
        + 1;
    // When: public extraction rejects the runtime declaration.
    let actual = error(source);
    // Then: the template expression, not a guessed hole coordinate, owns it.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}
