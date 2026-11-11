use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("({0:[first(),second()],1:[third(),fourth()]})")]
#[case("[[first(),second()],[third(),fourth()]]")]
#[serial]
fn responsive_selection_preserves_missing_keys_without_capture_arrows(
    #[case] table: &str,
    #[values("0", "1", "2", "undefined", "null", "Symbol('missing')")] key: &str,
) {
    let source = format!(
        "{BOX}export const a=(index,first,second,third,fourth,later)=><Box p={{{table}[index]}} id={{later()}}/>;"
    );
    let actual = evaluate(
        &source,
        &format!(
            "a({key},()=>{{trace.push('first');return '1px';}},()=>{{trace.push('second');return '2px';}},()=>{{trace.push('third');return '3px';}},()=>{{trace.push('fourth');return '4px';}},()=>{{trace.push('later');return 'id';}})"
        ),
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["first", "second", "third", "fourth", "later"])
    );
    assert_eq!(actual.element["props"]["id"], "id");
    let Some(style) = actual.element["props"]["style"].as_object() else {
        panic!("style object required")
    };
    let expected = match key {
        "0" => vec![serde_json::json!("1px"), serde_json::json!("2px")],
        "1" => vec![serde_json::json!("3px"), serde_json::json!("4px")],
        _ => {
            assert_eq!(actual.element["props"]["className"], "");
            Vec::new()
        }
    };
    assert_eq!(style.values().cloned().collect::<Vec<_>>(), expected);
}

#[rstest]
#[case("({0:[first(),second()],1:[third(),fourth()]})")]
#[case("[[first(),second()],[third(),fourth()]]")]
#[serial]
fn responsive_selection_coerces_the_key_once_after_eager_values(
    #[case] table: &str,
    #[values("return '0'", "return '1'", "return 'missing'", "throw 'key-failed'")]
    resolution: &str,
) {
    let source = format!(
        "{BOX}export const a=(index,first,second,third,fourth,later)=><Box p={{{table}[index]}} id={{later()}}/>;"
    );
    let actual = evaluate(
        &source,
        &format!(
            "(()=>{{try{{return a({{[Symbol.toPrimitive](){{trace.push('key');{resolution};}}}},()=>{{trace.push('first');return '1px';}},()=>{{trace.push('second');return '2px';}},()=>{{trace.push('third');return '3px';}},()=>{{trace.push('fourth');return '4px';}},()=>{{trace.push('later');return 'id';}});}}catch(error){{return {{error:String(error)}};}}}})()"
        ),
    );
    if resolution.starts_with("throw") {
        assert_eq!(actual.element, serde_json::json!({"error":"key-failed"}));
        assert_eq!(
            actual.trace,
            serde_json::json!(["first", "second", "third", "fourth", "key"])
        );
    } else {
        assert_eq!(actual.element["props"]["id"], "id");
        assert_eq!(
            actual.trace,
            serde_json::json!(["first", "second", "third", "fourth", "key", "later"])
        );
    }
}

#[rstest]
#[case("({0:[first(),'2px'],1:['3px',second()]})")]
#[case("[[first(),'2px'],['3px',second()]]")]
#[serial]
fn responsive_selection_with_static_and_dynamic_values_keeps_missing_keys_safe(
    #[case] table: &str,
    #[values("0", "1", "2")] key: &str,
) {
    let source = format!("{BOX}export const a=(index,first,second)=><Box p={{{table}[index]}}/>;");
    let actual = evaluate(
        &source,
        &format!(
            "a({key},()=>{{trace.push('first');return '1px';}},()=>{{trace.push('second');return '4px';}})"
        ),
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "second"]));
    let Some(style) = actual.element["props"]["style"].as_object() else {
        panic!("style object required")
    };
    let Some(class) = actual.element["props"]["className"].as_str() else {
        panic!("className string required")
    };
    match key {
        "0" => {
            assert_eq!(style["--padding-0-"], "1px");
            assert!(class.contains("padding-1-2px"));
        }
        "1" => {
            assert_eq!(style["--padding-1-"], "4px");
            assert!(class.contains("padding-0-3px"));
        }
        _ => {
            assert!(style.is_empty());
            assert_eq!(class, "");
        }
    }
}

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
