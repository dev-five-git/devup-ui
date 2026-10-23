use super::*;
use rstest::rstest;
use serial_test::serial;

fn source(props: &str) -> String {
    format!(
        "{BOX}import {{createElement}} from 'react';export const a=(first,read)=>createElement(Box,{props},'child');"
    )
}

#[rstest]
#[serial]
fn explicit_styles_when_create_element_props_are_wrapped_remain_independent(
    #[values("(BODY)", "((BODY))", "(BODY as const)", "(BODY satisfies object)")] wrapper: &str,
    #[values(true, false)] before: bool,
    #[values(true, false)] single_css: bool,
) {
    // Given an explicit declaration and an unknown spread in either order.
    let body = if before {
        "{...read(),color:'red',p:1}"
    } else {
        "{color:'red',p:1,...read()}"
    };
    let source = source(&wrapper.replace("BODY", body));

    // When the complete factory source is extracted and executed.
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("the factory does not compile: {error}"));
    let actual = whole::execute(
        &output.code,
        "(()=>{const raw={};const e=a(null,()=>{trace.push('spread');return {get color(){trace.push('getter');return raw;},p:'blue',id:'kept',className:'external',style:{opacity:0.5}};});return [e.type,e.props.color===raw,e.props.p,e.props.id,e.props.className,e.props.style,e.children];})()",
    );

    // Then authored CSS/classes coexist with raw identity and existing merges.
    assert_eq!(
        static_styles(&output),
        vec![
            ("color".into(), "red".into()),
            ("padding".into(), "4px".into())
        ]
    );
    assert_eq!(actual.trace, serde_json::json!(["spread", "getter"]));
    let suffix = if single_css { "" } else { "-a" };
    assert_eq!(
        actual.element,
        serde_json::json!(["div", true, "blue", "kept", format!("external color-0-red--255{suffix} padding-0-4px--255{suffix}"), {"opacity":0.5}, ["child"]])
    );
}

#[rstest]
#[serial]
fn authored_effects_when_create_element_props_are_wrapped_run_in_source_order(
    #[values("(BODY)", "((BODY))", "(BODY as const)", "(BODY satisfies object)")] wrapper: &str,
    #[values(true, false)] before: bool,
    #[values(true, false)] single_css: bool,
) {
    // Given a dynamic explicit style adjacent to an effectful unknown spread.
    let body = if before {
        "{...read(),color:first()}"
    } else {
        "{color:first(),...read()}"
    };
    let source = source(&wrapper.replace("BODY", body));

    // When the emitted factory runs with a conflicting raw color.
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("the factory does not compile: {error}"));
    let actual = whole::execute(
        &output.code,
        "(()=>{const e=a(()=>{trace.push('first');return 'red';},()=>{trace.push('spread');return {get color(){trace.push('getter');return 'green';}};});return [e.props.color,(e.props.className||'').trim().length>0,Object.values(e.props.style||{}),e.children];})()",
    );

    // Then the explicit dynamic class/value attaches, with each effect once.
    assert_eq!(output.styles.len(), 1);
    assert!(
        matches!(output.styles.iter().next(), Some(ExtractStyleValue::Dynamic(value)) if value.property() == "color")
    );
    assert_eq!(
        actual.element,
        serde_json::json!(["green", true, ["red"], ["child"]])
    );
    let expected = if before {
        vec!["spread", "getter", "first"]
    } else {
        vec!["first", "spread", "getter"]
    };
    assert_eq!(actual.trace, serde_json::json!(expected));
}

#[rstest]
#[serial]
fn explicit_styles_when_wrapped_create_element_has_no_spread_compile(
    #[values("(BODY)", "((BODY))", "(BODY as const)", "(BODY satisfies object)")] wrapper: &str,
    #[values(true, false)] single_css: bool,
) {
    // Given the wrapped literal without an unknown spread.
    let source = source(&wrapper.replace("BODY", "{color:'red'}"));

    // When the control compiles and runs.
    let output = compile_with(
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("the factory does not compile: {error}"));
    let actual = whole::execute(&output.code, "a()");

    // Then absolute red CSS and its class remain attached.
    assert_eq!(static_styles(&output), vec![("color".into(), "red".into())]);
    let suffix = if single_css { "" } else { "-a" };
    assert_eq!(
        actual.element["props"]["className"],
        format!("color-0-red--255{suffix}")
    );
    assert_eq!(actual.trace, serde_json::json!([]));
}
