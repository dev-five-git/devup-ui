use super::*;
use rstest::rstest;
use serial_test::serial;
const CREATE_ELEMENT: &str = "import {createElement} from 'react';\n";

#[rstest]
#[case("jsx(Box,{color:'red',...rest})", "color", "red")]
#[case("jsx(Box,{...rest,color:'red'})", "color", "red")]
#[case("jsx(Box,{color:'red',...rest,color:'blue'})", "color", "blue")]
#[case("jsx(Box,{bg:'red',background:'blue',...rest})", "background", "blue")]
#[case("jsx(Box,{...{color:'red'},...rest})", "color", "red")]
#[case("createElement(Box,{color:'red',...rest})", "color", "red")]
#[serial]
fn calls_when_unknown_spreads_interleave_keep_authored_styles(
    #[case] element: &str,
    #[case] property: &str,
    #[case] value: &str,
) {
    let source = format!("{BOX}{JSX_RUNTIME}{CREATE_ELEMENT}export const a=(rest)=>{element};");
    assert_eq!(
        static_styles(&output(&source)),
        vec![(property.into(), value.into())]
    );
    assert_eq!(
        whole::evaluate(&source, "a({color:'raw',bg:'raw'}).props.color").element,
        "raw"
    );
}

#[rstest]
#[case("jsx(Box,{color:'red',...first,color:'blue',...last})")]
#[case("createElement(Box,{color:'red',...first,color:'blue',...last},'child')")]
#[serial]
fn calls_when_repeated_spreads_own_keys_preserve_dom_precedence(#[case] element: &str) {
    let source = format!(
        "{BOX}{JSX_RUNTIME}{CREATE_ELEMENT}export const a=(first,last)=>{element};
    "
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("color".into(), "blue".into())]
    );
    for (first, last, expected) in [
        ("{color:'x'}", "{}", "x"),
        ("{}", "{ color: undefined }", "undefined"),
        ("{}", "{color:'y'}", "y"),
    ] {
        assert_eq!(
            whole::evaluate(&source, &format!("String(a({first},{last}).props.color)")).element,
            expected
        );
    }
    if element.starts_with("createElement") {
        assert!(output(&source).code.contains("\"child\""));
    }
}

#[test]
#[serial]
fn calls_when_runtime_style_is_explicit_ignore_raw_absence() {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(rest,x)=>jsx(Box,{{p:x,...rest}});");
    for rest in ["{ p: undefined }", "{}"] {
        assert_eq!(
            whole::evaluate(&source, &format!("Object.values(a({rest},5).props.style)")).element,
            serde_json::json!([5])
        );
    }
}

#[test]
#[serial]
fn calls_when_spread_sources_are_effectful_read_them_once() {
    let source = format!("{BOX}{JSX_RUNTIME}export const a=(f)=>jsx(Box,{{color:'red',...f()}});");
    let actual = whole::evaluate(
        &source,
        "a(()=>{trace.push('spread');return {color:'blue'};})",
    );
    assert_eq!(actual.trace, serde_json::json!(["spread"]));
    assert_eq!(actual.element["props"]["color"], "blue");
}

#[test]
#[serial]
fn calls_when_selectors_precede_spreads_keep_the_selector() {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=(rest)=>jsx(Box,{{_hover:{{bg:'red'}},...rest}});"
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("background".into(), "red".into())]
    );
    assert_eq!(
        whole::evaluate(&source, "a({_hover:{bg:'raw'}}).props._hover.bg").element,
        "raw"
    );
}

#[test]
#[serial]
fn calls_when_dom_props_are_explicit_keep_them() {
    let rendered = code(&format!(
        "{BOX}{JSX_RUNTIME}export const a=(rest)=>jsx(Box,{{as:'a',id:'x',styleOrder:3,color:'red',...rest}});"
    ));
    assert!(rendered.contains("jsx(\"a\""), "{rendered}");
    assert!(rendered.contains("id: \"x\""), "{rendered}");
}

#[test]
#[serial]
fn calls_when_style_keys_are_not_identifiers_keep_raw_keys() {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=(rest)=>jsx(Box,{{'background-color':'red',...rest}});"
    );
    assert_eq!(
        whole::evaluate(
            &source,
            "a({'background-color':'blue'}).props['background-color']"
        )
        .element,
        "blue"
    );
    assert_eq!(
        static_styles(&output(&source)),
        vec![("background-color".into(), "red".into())]
    );
}

#[test]
#[serial]
fn calls_when_props_suspend_keep_suspension_in_the_caller() {
    let rendered = code(&format!(
        "{BOX}{JSX_RUNTIME}export const a=async(k,f,g)=>jsx(Box,{{color:'red',[k]:await g(),...f()}});"
    ));
    assert_eq!(rendered.matches("f()").count(), 1, "{rendered}");
    assert!(rendered.contains("await g()"), "{rendered}");
}

#[test]
#[serial]
fn create_element_when_props_are_opaque_keeps_native_merging() {
    let rendered = code(&format!(
        "{CREATE_ELEMENT}{BOX}export const a=(p)=>createElement(Box,p,'x');"
    ));
    assert!(rendered.contains("...p"), "{rendered}");
    assert!(rendered.contains("?.className"), "{rendered}");
}
