use super::literal_w38k_support::{declarations, fixture, red, tokens};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("", vec!["built", "default", "default"])]
#[case("stop:undefined", vec!["built", "default", "default"])]
#[case("stop:false", vec!["built"])]
#[case("stop:true", vec!["built"])]
#[serial]
fn destructured_default_when_property_is_absent_or_present_obeys_javascript(
    #[values(
        "({stop=(trace.push('default'),false)})=>{return;}",
        "function({stop=(trace.push('default'),false)}){return;}"
    )]
    callback: &str,
    #[case] property: &str,
    #[case] expected_trace: Vec<&str>,
) {
    // Given: missing, explicit undefined and present values reach actual bindings.
    let renders = format!(
        "trace.push('built');const a=Card({{{property}}},null);const b=Card({{{property}}},null);"
    );
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), &renders);
    let control_source = fixture("color:red", &renders);
    // When: public extraction and both generated callback invocations execute.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let control = compile(&control_source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    let (control_classes, control_trace) = evaluate(&control.code);
    // Then: only absent/undefined bindings evaluate defaults, with omission styling.
    assert_eq!(trace, expected_trace);
    assert_eq!(control_trace, vec!["built"]);
    assert_eq!(tokens(&classes), vec![vec!["color-0-red--255-a"]; 2]);
    assert_eq!(classes, control_classes);
    assert_eq!(declarations(&output), red(&[None]));
    assert_eq!(declarations(&output), declarations(&control));
}
