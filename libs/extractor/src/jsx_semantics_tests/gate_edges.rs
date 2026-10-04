use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("<Box {...{color:first()}} color=\"blue\"/>")]
#[case("createElement(Box,{...{color:first()},color:'blue'})")]
#[serial]
fn overwritten_known_spreads_when_compiled_still_evaluate_written_values(#[case] element: &str) {
    let source =
        format!("{BOX}import {{createElement}} from 'react';export const a=(first)=>{element};");
    let actual = evaluate(&source, "a(()=>{trace.push('first');return 'red';})");
    assert_eq!(actual.trace, serde_json::json!(["first"]));
}

#[test]
#[serial]
fn known_spreads_when_style_keys_precede_dom_keys_keep_their_order() {
    let source =
        format!("{BOX}export const a=(take)=><Box {{...{{color:take('color'),id:take('id')}}}}/>;");
    let actual = evaluate(&source, "a(key=>{trace.push(key);return key;})");
    assert_eq!(actual.trace, serde_json::json!(["color", "id"]));
}

#[test]
#[serial]
fn nested_copy_when_a_later_prop_changes_the_source_keeps_the_original_value() {
    let source = format!(
        "{BOX}export const a=(rest,later)=><Box color={{{{...rest}}['color']}} id={{later()}}/>;"
    );
    let actual = evaluate(
        &source,
        "(()=>{let color='red';return a({get color(){trace.push('get');return color;}},()=>{trace.push('later');color='blue';return 'id';});})()",
    );
    assert_eq!(actual.trace, serde_json::json!(["get", "later"]));
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .is_some_and(|style| style.values().any(|value| value == "red")),
        "{}",
        actual.element
    );
}

#[test]
#[serial]
fn literal_selector_when_early_snapshot_would_hide_it_reports_the_original_key() {
    let message = error(&format!(
        "{BOX}export const a=(rest,key)=><Box {{...rest}} {{...{{_hover:{{color:'red'}},[key]:'x'}}}}/>;"
    ));
    assert!(
        message.contains("_hover") && message.starts_with("a.tsx:2:"),
        "{message}"
    );
}

#[test]
#[serial]
fn computed_selection_when_a_key_object_coerces_runs_the_coercion_once() {
    let source = format!(
        "{BOX}export const a=(key,first,second)=><Box color={{{{a:first,b:second}}[key]}}/>;"
    );
    let actual = evaluate(
        &source,
        "a({toString(){trace.push('key');return 'a';}},'red','blue')",
    );
    assert_eq!(actual.trace, serde_json::json!(["key"]));
}

#[test]
#[serial]
fn template_interpolations_when_later_props_execute_keep_coercion_order() {
    let source = format!(
        "{BOX}export const a=(value,later)=><Box className={{`${{value}} ${{later()}}`}}/>;"
    );
    let actual = evaluate(
        &source,
        "a({toString(){trace.push('coerce');return 'external';}},()=>{trace.push('later');return 'other';})",
    );
    assert_eq!(actual.trace, serde_json::json!(["coerce", "later"]));
}

#[rstest]
#[case("2 || load()", "padding", "8px")]
#[case("'$primary' ?? load()", "padding", "$primary")]
#[serial]
fn exact_logical_left_when_right_is_dynamic_keeps_its_static_style(
    #[case] value: &str,
    #[case] property: &str,
    #[case] expected: &str,
) {
    let actual = output(&format!(
        "{BOX}export const a=(load)=><Box p={{{value}}}/>;"
    ));
    assert!(
        static_styles(&actual).contains(&(property.to_string(), expected.to_string())),
        "{}",
        actual.code
    );
}

#[rstest]
#[case("`${color}` + ' !important'", true)]
#[case("`${color}` + ';'", false)]
#[serial]
fn binary_css_suffix_when_captured_is_not_left_in_the_variable(
    #[case] value: &str,
    #[case] important: bool,
) {
    let source = format!("{BOX}export const a=(color)=><Box bg={{{value}}}/>;");
    let actual = output(&source);
    let runtime = evaluate(&source, "a('red')");
    assert!(
        runtime.element["props"]["style"]
            .as_object()
            .is_some_and(|style| style.values().all(|value| value == "red")),
        "{}",
        runtime.element
    );
    assert!(actual.styles.iter().any(|style|matches!(style,ExtractStyleValue::Dynamic(style) if style.important()==important)),"{}",actual.code);
}

#[rstest]
#[case(
    "async function a(active,load,rest){return <Box color={active && await load()} {...rest}/>;}",
    "await load()"
)]
#[case(
    "function* a(active,load,rest){return <Box color={active && (yield load())} {...rest}/>;}",
    "yield load()"
)]
#[serial]
fn lazy_suspending_props_when_a_sync_wrapper_cannot_move_them_are_located(
    #[case] declaration: &str,
    #[case] written: &str,
) {
    let message = error(&format!("{BOX}export {declaration}"));
    assert!(
        message.starts_with("a.tsx:2:")
            && message.contains(written)
            && !message.contains("__devup"),
        "{message}"
    );
}

#[test]
#[serial]
fn nested_iterator_when_later_props_execute_is_expanded_at_its_original_position() {
    let source = format!(
        "{BOX}export const a=(items,later)=><Box color={{[...items,1][0]}} id={{later()}}/>;"
    );
    let actual = evaluate(
        &source,
        "a({[Symbol.iterator](){trace.push('iterator');return ['red'][Symbol.iterator]();}},()=>{trace.push('later');return 'id';})",
    );
    assert_eq!(actual.trace, serde_json::json!(["iterator", "later"]));
}

#[test]
#[serial]
fn missing_native_choice_when_classes_are_combined_emits_no_undefined_class() {
    let source =
        format!("{BOX}export const a=(key)=><Box color={{{{a:'red',b:'blue'}}[key]}} p={{1}}/>;");
    let actual = evaluate(&source, "a('missing')");
    assert!(
        actual.element["props"]["className"]
            .as_str()
            .is_some_and(|class| !class.contains("undefined")),
        "{}",
        actual.element
    );
}

#[rstest]
#[case("@media")]
#[case("@supports")]
#[serial]
fn hidden_at_rule_when_an_opaque_literal_is_snapshotted_is_located(#[case] rule: &str) {
    let message = error(&format!(
        "{BOX}export const a=(rest,key)=><Box {{...rest}} {{...{{'{rule}':{{'(min-width:500px)':{{color:'red'}}}},[key]:'x'}}}}/>;"
    ));
    assert!(
        message.starts_with("a.tsx:2:") && message.contains(rule),
        "{message}"
    );
}

#[test]
#[serial]
fn object_valued_conditional_template_when_selected_coerces_before_later_interpolations() {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=(active,later)=>jsx(Box,{{className:`${{active ? {{toString(){{trace.push('coerce');return 'external';}}}} : 'fallback'}} ${{later()}}`}});"
    );
    let actual = evaluate(&source, "a(true,()=>{trace.push('later');return 'other';})");
    assert_eq!(actual.trace, serde_json::json!(["coerce", "later"]));
}
