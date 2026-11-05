use super::*;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn lone_dynamic_style_keeps_the_value_in_place_without_an_arrow_wrapper() {
    let actual = output(&format!(
        "{BOX}export const a=(color)=><Box bg={{color}}/>;"
    ));
    assert!(
        !actual.code.contains("__devupValue"),
        "unexpected capture wrapper: {}",
        actual.code
    );
    assert!(actual.code.contains("color"), "{}", actual.code);
}

#[test]
#[serial]
fn lone_dynamic_template_keeps_the_value_in_place_without_an_arrow_wrapper() {
    let actual = output(&format!(
        r"{BOX}export const a=(color)=><Box bg={{`${{color}}`}}/>;"
    ));
    assert!(
        !actual.code.contains("__devupValue"),
        "unexpected capture wrapper: {}",
        actual.code
    );
}

#[test]
#[serial]
fn known_object_spreads_are_flattened_before_attribute_ordering() {
    let actual = output(&format!(
        r"{BOX}export const a=(color)=><Box {{...{{bg:color}}}}/>;"
    ));
    assert!(actual.code.contains("color"), "{}", actual.code);
}

#[test]
#[serial]
fn lone_static_member_reads_a_getter_once_without_a_capture_wrapper() {
    let source = format!(
        r"{BOX}const value={{get p(){{trace.push('p');return 'red';}}}};export const a=()=> <Box bg={{value.p}}/>;"
    );
    let compiled = output(&source);
    assert!(
        !compiled.code.contains("__devupValue"),
        "unexpected capture wrapper: {}",
        compiled.code
    );
    let actual = whole::evaluate(&source, "a().props");
    assert_eq!(
        actual.element["style"]
            .as_object()
            .and_then(|style| style.values().next()),
        Some(&serde_json::json!("red"))
    );
    assert_eq!(actual.trace, serde_json::json!(["p"]));
}

#[rstest]
#[case("'12px'", serde_json::json!(["12px"]))]
#[case("undefined", serde_json::json!([null]))]
#[serial]
fn lone_dynamic_call_runs_once_without_a_capture_wrapper(
    #[case] returned: &str,
    #[case] expected: serde_json::Value,
) {
    let source = format!("{BOX}export const a=(getWidth)=><Box w={{getWidth()}}/>;");
    let actual = whole::evaluate(
        &source,
        &format!("Object.values(a(()=>{{trace.push('width');return {returned};}}).props.style)"),
    );
    assert_eq!(actual.element, expected);
    assert_eq!(actual.trace, serde_json::json!(["width"]));
    assert!(!code(&source).contains("__devupValue"));
}

#[test]
#[serial]
fn lone_dynamic_member_call_keeps_its_receiver_and_argument_evaluation() {
    let source =
        format!("{BOX}export const a=(receiver,args)=><Box w={{receiver.read(...args)}}/>;");
    let actual = whole::evaluate(
        &source,
        "Object.values(a({width:'12px',get read(){trace.push('get');return function(value){trace.push(value);return this.width;};}},{[Symbol.iterator](){trace.push('iterator');return ['call'][Symbol.iterator]();}}).props.style)",
    );
    assert_eq!(actual.element, serde_json::json!(["12px"]));
    assert_eq!(actual.trace, serde_json::json!(["get", "iterator", "call"]));
    assert!(!code(&source).contains("__devupValue"));
}

#[rstest]
#[case("{get read(){trace.push('get');throw 'boom';}}", "[]", serde_json::json!(["get"]))]
#[case("{get read(){trace.push('get');return ()=>{trace.push('call');};}}", "{[Symbol.iterator](){trace.push('iterator');throw 'boom';}}", serde_json::json!(["get","iterator"]))]
#[case("{get read(){trace.push('get');return ()=>{trace.push('call');throw 'boom';};}}", "[]", serde_json::json!(["get","call"]))]
#[serial]
fn lone_dynamic_call_preserves_throw_cutoff(
    #[case] receiver: &str,
    #[case] args: &str,
    #[case] trace: serde_json::Value,
) {
    let source =
        format!("{BOX}export const a=(receiver,args)=><Box w={{receiver.read(...args)}}/>;");
    let actual = whole::evaluate(
        &source,
        &format!("(()=>{{try{{return a({receiver},{args});}}catch(error){{return error;}}}})()"),
    );
    assert_eq!(actual.element, "boom");
    assert_eq!(actual.trace, trace);
}

#[rstest]
#[case("<Box w={read()} id={later()}/>", serde_json::json!(["read", "later"]), true)]
#[case("<Box w={read()}>{later()}</Box>", serde_json::json!(["read", "later"]), false)]
#[case("<Box boxSize={read()}/>", serde_json::json!(["read"]), true)]
#[serial]
fn dynamic_calls_that_move_or_expand_still_run_once(
    #[case] element: &str,
    #[case] trace: serde_json::Value,
    #[case] captured: bool,
) {
    let source = format!("{BOX}export const a=(read,later)=>{element};");
    let actual = whole::evaluate(
        &source,
        "a(()=>{trace.push('read');return '12px';},()=>{trace.push('later');return 'id';})",
    );
    assert_eq!(actual.trace, trace);
    let style = actual.element["props"]["style"]
        .as_object()
        .unwrap_or_else(|| panic!("missing generated style: {}", actual.element));
    assert_eq!(style.len(), if element.contains("boxSize") { 2 } else { 1 });
    assert!(style.values().all(|value| value == "12px"));
    assert_eq!(code(&source).contains("__devupValue"), captured);
}

#[test]
#[serial]
fn class_templates_when_captured_keep_their_static_tailwind_rules() {
    let actual = output(&format!(
        "{BOX}export const a=(get,rest)=><Box className={{`${{get()}} text-[#3B82F6] p-4`}} {{...rest}}/>;"
    ));
    let styles = static_styles(&actual);
    assert!(
        styles
            .iter()
            .any(|(property, value)| property == "color" && value == "#3B82F6"),
        "{styles:?}"
    );
    assert!(
        styles
            .iter()
            .any(|(property, value)| property == "padding" && value == "1rem"),
        "{styles:?}"
    );
    assert_eq!(actual.code.matches("get()").count(), 1);
}

#[test]
#[serial]
fn computed_literal_selector_keys_when_captured_keep_the_static_rule() {
    let actual = output(&format!(
        "{BOX}{JSX_RUNTIME}export const a=(rest)=>jsx(Box,{{['_hover']:{{bg:'blue'}},color:'red'}});"
    ));
    assert!(actual.styles.iter().any(|style|matches!(style,ExtractStyleValue::Static(style) if style.value=="blue"&&style.selector().is_some())),"{}",actual.code);
}

#[test]
#[serial]
fn computed_literal_selectors_when_unknown_keys_follow_keep_their_rule() {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=(key)=>jsx(Box,{{['_hover']:{{bg:'blue'}},[key]:'red'}});"
    );
    assert!(output(&source).styles.iter().any(|style|matches!(style,ExtractStyleValue::Static(value) if value.value()=="blue" && value.selector().is_some())));
    let actual = whole::evaluate(
        &source,
        "a({toString(){trace.push('key');return '_hover';}}).props._hover",
    );
    assert_eq!(actual.element, "red");
    assert_eq!(actual.trace, serde_json::json!(["key"]));
}

#[test]
#[serial]
fn named_blue_500_when_captured_keeps_the_main_oklch_palette() {
    let actual = output(&format!(
        "{BOX}export const a=(get,rest)=><Box className={{`${{get()}} text-blue-500 p-4`}} {{...rest}}/>;"
    ));
    let styles = static_styles(&actual);
    assert!(
        styles.iter().any(|(property, value)| {
            property == "color" && value == "oklch(62.3% .214 259.815)"
        }),
        "{styles:?}"
    );
    assert!(
        styles
            .iter()
            .any(|(property, value)| property == "padding" && value == "1rem"),
        "{styles:?}"
    );
    assert_eq!(actual.code.matches("get()").count(), 1);
}
