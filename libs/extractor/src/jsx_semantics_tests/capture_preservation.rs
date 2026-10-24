use super::*;
use serial_test::serial;

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
