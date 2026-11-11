use super::whole::execute;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("[1,.5,...rest]", "[.7,1]", "0", Some("opacity-0-1--255-a"))]
#[case("[1,.5,...rest]", "[.7,1]", "1", Some("opacity-0-_d_5--255-a"))]
#[case("[1,.5,...rest]", "[.7,1]", "2", None)]
#[case("[1,.5,...rest]", "[.7,1]", "3", Some("opacity-0-1--255-a"))]
#[case("[1,.5,...rest]", "[.7,1]", "100", None)]
#[case("[1,.5,...rest]", "[]", "undefined", None)]
#[case("[1,.5,...rest]", "[]", "null", None)]
#[case("[1,.5,...rest]", "[]", "Symbol('missing')", None)]
#[case("[1,.5,...rest]", "['1']", "2", None)]
#[case("[1,.5,...rest]", "[null]", "2", None)]
#[case("[1,.5,...rest]", "[false]", "2", None)]
#[case("[...rest]", "[.7]", "0", None)]
#[case("[...rest,1]", "[.7,.5]", "2", Some("opacity-0-1--255-a"))]
#[case("[...rest,1]", "[.7,.5]", "1", None)]
#[case("[,,1,...rest]", "[.7]", "0", None)]
#[case("[,,1,...rest]", "[.7]", "2", Some("opacity-0-1--255-a"))]
#[case("[1,...rest,.5,...rest]", "[.7]", "2", Some("opacity-0-_d_5--255-a"))]
#[serial]
fn spread_selections_reuse_static_classes_and_keep_other_values_dynamic(
    #[case] table: &str,
    #[case] rest: &str,
    #[case] key: &str,
    #[case] expected_class: Option<&str>,
) {
    let source = format!("{BOX}export const a=(rest,key)=><Box opacity={{{table}[key]}}/>;");
    let compiled = code(&source);
    let probe = format!("a({rest},{key})");
    let original = execute(&source.replace("<Box", "<div"), &probe);
    let actual = execute(&compiled, &probe);
    assert_eq!(actual.trace, original.trace);
    if let Some(class) = expected_class {
        assert_eq!(actual.element["props"]["className"], class, "{compiled}");
    } else if let Some(value) = original.element["props"].get("opacity") {
        assert!(
            actual.element["props"]["style"]
                .as_object()
                .is_some_and(|style| style.values().any(|selected| selected == value)),
            "{compiled}: {}",
            actual.element
        );
    }
}

#[rstest]
#[case("[1,.5,...rest]", "'0'", false)]
#[case("[1,.5,...rest]", "'2'", false)]
#[case("[1,.5,...rest]", "'0'", true)]
#[serial]
fn spread_iteration_and_key_coercion_run_once_before_later_props(
    #[case] table: &str,
    #[case] key: &str,
    #[case] throws: bool,
) {
    let source = format!(
        "{BOX}export const a=(rest,key,later,child)=><Box opacity={{{table}[key]}} id={{later()}}>{{child()}}</Box>;"
    );
    let compiled = code(&source);
    let fail = if throws { "throw 'key-failed';" } else { "" };
    let probe = format!(
        "(()=>{{try{{return a({{*[Symbol.iterator](){{trace.push('iterate');yield .7;trace.push('done');}}}},{{[Symbol.toPrimitive](){{trace.push('key');{fail}return {key};}}}},()=>{{trace.push('later');return 'id';}},()=>{{trace.push('child');return 'text';}});}}catch(error){{return {{error:String(error)}};}}}})()"
    );
    let original = execute(
        &source.replace("<Box", "<div").replace("</Box>", "</div>"),
        &probe,
    );
    let actual = execute(&compiled, &probe);
    assert_eq!(actual.trace, original.trace);
    assert_eq!(actual.element.get("error"), original.element.get("error"));
    assert_eq!(
        actual.trace,
        if throws {
            serde_json::json!(["iterate", "done", "key"])
        } else {
            serde_json::json!(["iterate", "done", "key", "later", "child"])
        }
    );
}

#[test]
#[serial]
fn unselected_calls_still_run_and_spread_errors_prevent_the_key_and_later_props() {
    let source = format!(
        "{BOX}export const a=(first,rest,key,later)=><Box opacity={{[first(),1,...rest][key]}} id={{later()}}/>;"
    );
    let probe = "(()=>{try{return a(()=>{trace.push('first');return .2;},{*[Symbol.iterator](){trace.push('iterate');throw 'spread-failed';}},{toString(){trace.push('key');return '1';}},()=>{trace.push('later');return 'id';});}catch(error){return {error:String(error)};}})()";
    let original = execute(&source.replace("<Box", "<div"), probe);
    let actual = execute(&code(&source), probe);
    assert_eq!(actual.trace, original.trace);
    assert_eq!(actual.trace, serde_json::json!(["first", "iterate"]));
    assert_eq!(actual.element, original.element);
}

#[test]
#[serial]
fn dynamic_objects_are_not_coerced_to_choose_a_static_class() {
    let source = format!("{BOX}export const a=rest=><Box opacity={{[1,.5,...rest][2]}}/>;");
    let probe = "a([{[Symbol.toPrimitive](){trace.push('coerce');throw 'must-not-coerce';}}])";
    let original = execute(&source.replace("<Box", "<div"), probe);
    let actual = execute(&code(&source), probe);
    assert_eq!(actual.trace, original.trace);
    assert_eq!(actual.trace, serde_json::json!([]));
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .is_some_and(|style| style.values().any(serde_json::Value::is_object))
    );
}

#[test]
#[serial]
fn string_values_in_spread_selections_keep_their_static_color_classes() {
    let source =
        format!("{BOX}export const a=(rest,key)=><Box color={{['red','blue',...rest][key]}}/>;");
    let compiled = code(&source);
    assert!(compiled.contains("color-0-red--255-a"), "{compiled}");
    assert!(compiled.contains("color-0-blue--255-a"), "{compiled}");
    let actual = execute(&compiled, "a(['red'],'2')");
    assert_eq!(actual.element["props"]["className"], "color-0-red--255-a");
}

#[test]
#[serial]
fn unselected_dynamic_elements_still_run_once_when_a_literal_is_selected() {
    let source = format!(
        "{BOX}export const a=(first,rest,key)=><Box opacity={{[first(),1,...rest][key]}}/>;"
    );
    let probe = "a(()=>{trace.push('first');return .2;},{*[Symbol.iterator](){trace.push('iterate');yield .7;}},{toString(){trace.push('key');return '1';}})";
    let original = execute(&source.replace("<Box", "<div"), probe);
    let actual = execute(&code(&source), probe);
    assert_eq!(actual.trace, original.trace);
    assert_eq!(actual.trace, serde_json::json!(["first", "iterate", "key"]));
    assert_eq!(actual.element["props"]["className"], "opacity-0-1--255-a");
}

#[test]
#[serial]
fn inherited_selection_is_read_once_before_later_attributes() {
    let source = format!(
        "{BOX}export const a=(rest,later)=><Box opacity={{[1,.5,...rest]['missing']}} id={{later()}}/>;"
    );
    let compiled = code(&source);
    let setup = "Object.defineProperty(Array.prototype,'missing',{get(){trace.push('inherited');return .5;}});";
    let probe = "a([],()=>{trace.push('later');return 'id';})";
    let original = execute(&format!("{setup}{}", source.replace("<Box", "<div")), probe);
    let actual = execute(&format!("{setup}{compiled}"), probe);
    assert_eq!(actual.trace, original.trace);
    assert_eq!(actual.trace, serde_json::json!(["inherited", "later"]));
    assert_eq!(
        actual.element["props"]["className"],
        "opacity-0-_d_5--255-a"
    );
}

#[rstest]
#[case(
    "export const a=(rest,key,el=<Box opacity={[1,.5,...rest][key]}/>)=>el;",
    "a([],0)",
    1
)]
#[case(
    "const rest=[],key=0; class A { el=<Box opacity={[1,.5,...rest][key]}/>; } export const a=new A().el;",
    "a",
    0
)]
#[case(
    "export const a=(rest,key)=>{eval('');return <Box opacity={[1,.5,...rest][key]}/>;};",
    "a([],0)",
    1
)]
#[serial]
fn static_class_optimization_does_not_introduce_functions_in_expression_only_scopes(
    #[case] body: &str,
    #[case] probe: &str,
    #[case] arrows: usize,
) {
    let compiled = code(&format!("{BOX}{body}"));
    assert_eq!(compiled.matches("=>").count(), arrows, "{compiled}");
    assert!(!compiled.contains("=== 1"), "{compiled}");
    let actual = execute(&compiled, probe);
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .is_some_and(|style| style.values().any(|value| value == 1))
    );
}

#[rstest]
#[case(r"['\uD800',...rest]", 0)]
#[case(r"['red','\uD800',...rest]", 1)]
#[serial]
fn strings_that_cannot_be_css_remain_native_dynamic_values(
    #[case] table: &str,
    #[case] index: usize,
) {
    let source = format!("{BOX}export const a=(rest,index)=><Box color={{{table}[index]}}/>;");
    let compiled = code(&source);
    let probe = format!(
        "(()=>{{const element=a([],{index});return Object.values(element.props.style)[0].charCodeAt(0);}})()"
    );
    assert_eq!(
        execute(&compiled, &probe).element,
        serde_json::json!(0xd800)
    );
}
