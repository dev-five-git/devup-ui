use super::literal_w38k_support::{GETTERS, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>{if(p.stop)return 2;return;}", vec!["built", "get", "get"])]
#[case("function(p){if(p.stop)return 2;return;}", vec!["built", "get", "get"])]
#[case(
    "p=>{if(p.stop){trace.push('valued');return 2;}trace.push('bare');return;}",
    vec!["built", "get", "valued", "get", "bare"]
)]
#[case(
    "function(p){if(p.stop){trace.push('valued');return 2;}trace.push('bare');return;}",
    vec!["built", "get", "valued", "get", "bare"]
)]
#[serial]
fn terminal_bare_when_branch_is_selected_keeps_only_its_order(
    #[case] callback: &str,
    #[case] trace: Vec<&str>,
) {
    // Given: the four immutable terminal callback fixtures and their getter renders.
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), GETTERS);
    // When: public extraction and the actual generated renders execute.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, actual_trace) = evaluate(&output.code);
    // Then: valued selects 2, bare selects absence, and neither invocation replays.
    assert_eq!(actual_trace, trace);
    assert_eq!(
        tokens(&classes),
        vec![vec!["color-0-red--2-a"], vec!["color-0-red--255-a"]]
    );
    assert_eq!(declarations(&output), red(&[None, Some(2)]));
}

#[rstest]
#[case("p=>{return;}")]
#[case("function(p){return;}")]
#[serial]
fn all_bare_when_callback_is_pure_matches_the_omission_control(#[case] callback: &str) {
    // Given: the exact all-bare fixtures and the same literal with metadata omitted.
    let source = fixture(&format!("style-order:${{{callback}}};color:red"), GETTERS);
    let control_source = fixture("color:red", GETTERS);
    // When: both public outputs execute their actual generated components.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let control = compile(&control_source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    let (control_classes, control_trace) = evaluate(&control.code);
    // Then: no order remains, with unchanged selected classes and component reads.
    assert_eq!(trace, vec!["built", "get", "get"]);
    assert_eq!(trace, control_trace);
    assert_eq!(classes, control_classes);
    assert_eq!(tokens(&classes), vec![vec!["color-0-red--255-a"]; 2]);
    assert_eq!(declarations(&output), red(&[None]));
    assert_eq!(declarations(&output), declarations(&control));
}
