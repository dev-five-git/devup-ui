use super::literal_w38k_support::{GETTERS, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>{return {color:'red'};}", "", vec!["built", "get", "get"])]
#[case("function(p){return {color:'red'};}", "", vec!["built", "get", "get"])]
#[case("p=>{return;}", "color:red", vec!["built", "get", "get"])]
#[case("function(p){return;}", "color:red", vec!["built", "get", "get"])]
#[case(
    "p=>{trace.push('generic');return;}",
    "color:red",
    vec!["built", "get", "generic", "get", "generic"]
)]
#[serial]
fn generic_callback_when_it_is_not_metadata_keeps_the_existing_block_route(
    #[case] callback: &str,
    #[case] declarations_after: &str,
    #[case] expected_trace: Vec<&str>,
) {
    // Given: real nonorder callbacks exercise existing valued and bare behavior.
    let source = fixture(&format!("${{{callback}}};{declarations_after}"), GETTERS);
    // When: public extraction and actual generated generic callback renders execute.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    // Then: generic acceptance, retained invocation and full red declarations persist.
    assert_eq!(trace, expected_trace);
    assert_eq!(tokens(&classes), vec![vec!["color-0-red--255-a"]; 2]);
    assert_eq!(declarations(&output), red(&[None]));
}
