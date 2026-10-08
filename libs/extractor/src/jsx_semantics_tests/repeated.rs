use super::*;
use serial_test::serial;

fn slot_of<'a>(found: &'a [Slot], property: &str) -> &'a Slot {
    found
        .iter()
        .find(|slot| slot.property == property)
        .unwrap_or_else(|| panic!("a slot for `{property}`"))
}

#[test]
#[serial]
fn the_last_spread_having_the_key_wins_and_a_missing_key_keeps_the_earlier_one() {
    let output = output(&format!(
        "{BOX}export const a = (a, b) => <Box color=\"red\" {{...a}} p={{1}} {{...b}} />;"
    ));
    let found = slots(&output);
    let color = &slot_of(&found, "color").identifier;
    let padding = &slot_of(&found, "padding").identifier;

    assert_eq!(found.len(), 2);
    let both = |a: &str, b: &str| format!("const a = {a}; const b = {b};");
    assert_eq!(read(color, &both("{ color: 'blue' }", "{}")), "\"blue\"");
    assert_eq!(
        read(color, &both("{ color: 'blue' }", "{ color: 'green' }")),
        "\"green\""
    );
    assert_eq!(read(color, &both("{}", "{ color: 'green' }")), "\"green\"");
    assert_eq!(read(color, &both("{}", "{}")), "undefined");
    assert_eq!(read(padding, &both("{ p: 2 }", "{}")), "undefined");
    assert_eq!(read(padding, &both("{}", "{ p: 2 }")), "2");
}

#[test]
#[serial]
fn a_later_own_undefined_clears_what_an_earlier_spread_gave() {
    let output = output(&format!(
        "{BOX}export const a = (first, last) => <Box color=\"red\" {{...first}} {{...last}} />;"
    ));
    let identifier = &slots(&output)[0].identifier;
    let both = |first: &str, last: &str| format!("const first = {first}; const last = {last};");

    assert_eq!(
        read(
            identifier,
            &both("{ color: 'blue' }", "{ color: undefined }")
        ),
        "undefined"
    );
    assert_eq!(
        read(identifier, &both("{ color: 'blue' }", "{ color: null }")),
        "null"
    );
    assert_eq!(
        read(identifier, &both("{ color: 'blue' }", "{}")),
        "\"blue\""
    );
}

#[test]
#[serial]
fn an_explicit_style_written_after_the_spread_is_the_only_one_that_counts() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box color=\"red\" {{...rest}} color=\"blue\" />;"
    ));

    assert_eq!(slots(&output).len(), 0);
    assert_eq!(
        static_styles(&output),
        vec![("color".to_string(), "blue".to_string())]
    );
}

#[test]
#[serial]
fn an_alias_written_later_replaces_the_earlier_key_of_the_property() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box bg=\"red\" background=\"blue\" {{...rest}} />;"
    ));
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fallback, "blue");
    assert_eq!(found[0].variable, "--background-spread-background-0-");
}

#[test]
#[serial]
fn a_literal_spread_before_an_unknown_one_gives_way_with_its_own_keys() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box {{...{{ color: 'red', p: 1 }}}} {{...rest}} />;"
    ));
    let found = slots(&output);

    assert_eq!(
        found
            .iter()
            .map(|slot| (slot.property.as_str(), slot.fallback.as_str()))
            .collect::<Vec<_>>(),
        vec![("color", "red"), ("padding", "4px")]
    );
    assert_eq!(
        read(
            &slot_of(&found, "color").identifier,
            "const rest = { color: 'blue', margin: 1 };"
        ),
        "\"blue\""
    );
    assert_eq!(
        read(
            &slot_of(&found, "padding").identifier,
            "const rest = { color: 'blue', margin: 1 };"
        ),
        "undefined"
    );
}

#[test]
#[serial]
fn a_literal_holding_a_spread_is_one_more_source_to_read() {
    let output = output(&format!(
        "{BOX}export const a = (a, b) => <Box color=\"red\" {{...{{ ...a }}}} {{...b}} />;"
    ));
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    let both = |a: &str, b: &str| format!("const a = {a}; const b = {b};");
    assert_eq!(
        read(&found[0].identifier, &both("{ color: 'x' }", "{}")),
        "\"x\""
    );
    assert_eq!(
        read(
            &found[0].identifier,
            &both("{ color: 'x' }", "{ color: 'y' }")
        ),
        "\"y\""
    );
}

#[test]
#[serial]
fn a_literal_spread_after_the_unknown_one_stays_a_plain_style() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box {{...rest}} {{...{{ color: 'red' }}}} />;"
    ));

    assert_eq!(slots(&output).len(), 0);
    assert_eq!(
        static_styles(&output),
        vec![("color".to_string(), "red".to_string())]
    );
}

#[test]
#[serial]
fn a_literal_spread_replaced_by_a_later_attribute_leaves_no_slot() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box {{...{{ color: 'red' }}}} {{...rest}} color=\"blue\" />;"
    ));

    assert_eq!(slots(&output).len(), 0);
    assert_eq!(
        static_styles(&output),
        vec![("color".to_string(), "blue".to_string())]
    );
}

#[test]
#[serial]
fn a_literal_spread_keeps_the_props_that_are_not_styles() {
    let rendered = code(&format!(
        "{BOX}export const a = (rest) => <Box {{...{{ color: 'red', id: 'x' }}}} {{...rest}} />;"
    ));

    assert!(rendered.contains("id: \"x\""), "{rendered}");
    assert!(!rendered.contains("color: \"red\""), "{rendered}");
}

#[test]
#[serial]
fn two_attributes_of_one_property_do_not_share_a_variable_when_written_differently() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box mx={{1}} ml={{2}} {{...rest}} />;"
    ));
    let variables: std::collections::BTreeSet<String> = slots(&output)
        .iter()
        .map(|slot| slot.variable.clone())
        .collect();

    assert_eq!(
        variables,
        std::collections::BTreeSet::from([
            "--margin-left-spread-ml-0-".to_string(),
            "--margin-right-spread-mx-0-".to_string()
        ])
    );
}
