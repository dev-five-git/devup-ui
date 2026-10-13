use super::literal_w38k_support::{GETTERS, declarations, fixture, red, tokens};
use super::{compile, evaluate};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "style-order:2;style-order:${p=>{return;}};color:red",
    ("color:red", None),
    vec!["built", "get", "get"]
)]
#[case(
    "style-order:${p=>{trace.push('earlier');return 2;}};style-order:${p=>{return;}};color:red",
    ("color:red", None),
    vec!["built", "get", "earlier", "get", "earlier"]
)]
#[case(
    "style-order:${p=>{return;}};style-order:7;color:red",
    ("style-order:7;color:red", Some(7)),
    vec!["built", "get", "get"]
)]
#[case(
    "style-order:2;style-order:${p=>{return;}};style-order:7;color:red",
    ("style-order:7;color:red", Some(7)),
    vec!["built", "get", "get"]
)]
#[case(
    "style-order:${p=>{trace.push('first');return;}};style-order:${p=>{trace.push('second');return;}};color:red",
    ("color:red", None),
    vec!["built", "get", "first", "second", "get", "first", "second"]
)]
#[serial]
fn absent_boundary_when_metadata_is_repeated_erases_only_earlier_orders(
    #[case] body: &str,
    #[case] expected: (&str, Option<u8>),
    #[case] expected_trace: Vec<&str>,
) {
    // Given: earlier orders, later orders and distinguishable callback effects.
    let source = fixture(body, GETTERS);
    let control_source = fixture(expected.0, GETTERS);
    // When: public extraction normalizes the source and actual generated renders run.
    let output = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let control = compile(&control_source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, trace) = evaluate(&output.code);
    let (control_classes, control_trace) = evaluate(&control.code);
    // Then: no earlier order resurrects, later 7 survives and all effects remain.
    assert_eq!(trace, expected_trace);
    assert_eq!(control_trace, vec!["built", "get", "get"]);
    assert_eq!(tokens(&classes), tokens(&control_classes));
    assert_eq!(declarations(&output), red(&[expected.1]));
    assert_eq!(declarations(&output), declarations(&control));
}
