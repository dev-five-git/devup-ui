use super::*;
use serial_test::serial;

#[test]
#[serial]
fn class_templates_when_captured_keep_their_static_tailwind_rules() {
    let actual = output(&format!(
        "{BOX}export const a=(get,rest)=><Box className={{`${{get()}} text-blue-500 p-4`}} {{...rest}}/>;"
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
fn a_computed_literal_selector_when_an_unknown_key_can_replace_it_is_located() {
    let message = error(&format!(
        "{BOX}{JSX_RUNTIME}export const a=(key)=>jsx(Box,{{['_hover']:{{bg:'blue'}},[key]:'red'}});"
    ));
    assert!(message.contains("cannot use `_hover`"), "{message}");
    assert!(message.starts_with("a.tsx:3:"), "{message}");
}
