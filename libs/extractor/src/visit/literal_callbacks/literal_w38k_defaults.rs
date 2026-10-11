use super::literal_w38k_support::{PLAIN, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("(p,q=(trace.push('default'),2))=>{trace.push('body');return;}")]
#[case("function(p,q=(trace.push('default'),2)){trace.push('body');return;}")]
#[serial]
fn second_default_when_only_props_are_supplied_executes_before_the_body(#[case] callback: &str) {
    // Given: the omitted second argument has an observable authored default.
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), PLAIN);
    let control_source = fixture("color:red", PLAIN);
    // When: the actual generated component invokes the callback for both renders.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let control = compile(&control_source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    let (control_classes, control_trace) = evaluate(&control.code);
    // Then: defaults and bodies execute exactly twice without changing styling.
    assert_eq!(trace, vec!["built", "default", "body", "default", "body"]);
    assert_eq!(control_trace, vec!["built"]);
    assert_eq!(tokens(&classes), vec![vec!["color-0-red--255-a"]; 2]);
    assert_eq!(classes, control_classes);
    assert_eq!(declarations(&output), red(&[None]));
    assert_eq!(declarations(&output), declarations(&control));
}

#[rstest]
#[case("(p=(trace.push('first'),{}))=>{return;}")]
#[case("function(p=(trace.push('first'),{})){return;}")]
#[serial]
fn first_default_when_props_are_supplied_stays_unexecuted(#[case] callback: &str) {
    // Given: a first-parameter default differs observably from the supplied object.
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), PLAIN);
    // When: real generated renders supply their props to the retained callback.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    // Then: neither first default runs, and every selected declaration is unlayered.
    assert_eq!(trace, vec!["built"]);
    assert_eq!(tokens(&classes), vec![vec!["color-0-red--255-a"]; 2]);
    assert_eq!(declarations(&output), red(&[None]));
}
