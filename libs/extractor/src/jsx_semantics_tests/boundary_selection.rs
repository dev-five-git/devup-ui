use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("0")]
#[case("index")]
#[serial]
fn selection_boundary_when_unselected_branches_run_keeps_eager_source_order(#[case] key: &str) {
    let source = format!(
        "{BOX}export const a=(index,first,second,later)=><Box p={{[[first()],[second()]][{key}]}} id={{later()}}/>;"
    );
    let actual = evaluate(
        &source,
        "a(0,()=>{trace.push('first');return '4px';},()=>{trace.push('second');return '8px';},()=>{trace.push('later');return 'id';})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["first", "second", "later"])
    );
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .is_some_and(|style| { style.values().any(|value| value == "4px") })
    );
}

#[rstest]
#[case("0")]
#[case("index")]
#[serial]
fn selection_boundary_when_an_unselected_branch_throws_prevents_later_attributes(
    #[case] key: &str,
) {
    let source = format!(
        "{BOX}export const a=(index,first,second,later)=><Box p={{[[first()],[second()]][{key}]}} id={{later()}}/>;"
    );
    let actual = evaluate(
        &source,
        "(()=>{try{return a(0,()=>{trace.push('first');return '4px';},()=>{trace.push('second');throw 'boom';},()=>{trace.push('later');return 'id';});}catch(error){return error;}})()",
    );
    assert_eq!(actual.element, "boom");
    assert_eq!(actual.trace, serde_json::json!(["first", "second"]));
}

#[test]
#[serial]
fn selection_boundary_when_selected_array_is_nested_keeps_existing_flat_array_error() {
    let source = format!("{BOX}export const a=<Box p={{[[[1,2]], [3,4]][0]}}/>;");
    let message = error(&source);
    assert!(
        message.contains("responsive arrays must be flat"),
        "{message}"
    );
    assert!(message.contains("[1, 2]"), "{message}");
}
