use super::*;
use serial_test::serial;

#[test]
#[serial]
fn a_selector_object_before_a_spread_is_a_located_error() {
    let message = error(&format!(
        "{BOX}export const a = (rest) => <Box\n  _hover={{{{ bg: 'red' }}}}\n  {{...rest}}\n/>;"
    ));

    assert!(
        message.starts_with("a.tsx:3:3: `<Box>` cannot use `_hover`"),
        "{message}"
    );
    assert!(
        message.contains("write the spread before `_hover`"),
        "{message}"
    );
}

#[test]
#[serial]
fn a_selector_object_after_a_spread_keeps_its_rule() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box {{...rest}} _hover={{{{ bg: 'red' }}}} />;"
    ));

    assert_eq!(slots(&output).len(), 0);
    assert_eq!(static_styles(&output).len(), 1);
}

#[test]
#[serial]
fn a_runtime_value_under_a_selector_before_a_spread_is_an_error() {
    let message = error(&format!(
        "{BOX}export const a = (rest, x) => <Box _hover={{{{ bg: x }}}} {{...rest}} />;"
    ));

    assert!(message.contains("cannot use `_hover`"), "{message}");
}

#[test]
#[serial]
fn shapes_no_variable_can_follow_are_located_errors() {
    for (attribute, key) in [
        ("typography=\"body\"", "typography"),
        ("bg={['red', 'blue'][index]}", "bg"),
        ("positioning={mode}", "positioning"),
        ("selectors={{ '& > a': { color: 'red' } }}", "selectors"),
    ] {
        let message = error(&format!(
            "{BOX}export const a = (rest, index, mode) => <Box {attribute} {{...rest}} />;"
        ));

        assert!(
            message.contains(&format!("cannot use `{key}`")),
            "{attribute}: {message}"
        );
    }
}

#[test]
#[serial]
fn a_value_the_build_cannot_read_stays_an_error_before_a_spread() {
    let message = error(&format!(
        "{BOX}export const a = (rest) => <Box bg={{() => 1}} {{...rest}} />;"
    ));

    assert!(message.contains("cannot use `() => 1`"), "{message}");
}

#[test]
#[serial]
fn a_spread_beside_suspending_props_is_evaluated_once() {
    let rendered = code(&format!(
        "{BOX}export const a = async (f, load) => <Box color=\"red\" bg={{await load()}} {{...f()}} />;"
    ));

    assert_eq!(rendered.matches("f()").count(), 1, "{rendered}");
    assert!(rendered.contains("await load()"), "{rendered}");
}
