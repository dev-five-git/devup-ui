use super::*;
use serial_test::serial;

#[test]
#[serial]
fn explicit_style_wins_when_the_spread_is_written_before_it() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box {{...rest}} color=\"red\" />;"
    ));

    assert_eq!(slots(&output).len(), 0);
    assert_eq!(
        static_styles(&output),
        vec![("color".to_string(), "red".to_string())]
    );
    assert!(!output.code.contains("rest?.color"), "{}", output.code);
}

#[test]
#[serial]
fn explicit_style_falls_back_when_a_spread_is_written_after_it() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box color=\"red\" {{...rest}} />;"
    ));

    let found = slots(&output);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].property, "color");
    assert_eq!(found[0].level, 0);
    assert_eq!(found[0].variable, "--color-spread-color-0-");
    assert_eq!(found[0].fallback, "red");
    assert!(
        output.code.contains("\"--color-spread-color-0-\":"),
        "{}",
        output.code
    );
    assert!(output.code.contains("...rest"), "{}", output.code);
}

#[test]
#[serial]
fn the_slot_holds_what_the_spread_assigns_as_it_is() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box color=\"red\" {{...rest}} />;"
    ));
    let identifier = &slots(&output)[0].identifier;

    assert_eq!(
        read(identifier, "const rest = { color: 'blue' };"),
        "\"blue\""
    );
    assert_eq!(read(identifier, "const rest = { color: 3 };"), "3");
    assert_eq!(read(identifier, "const rest = { color: null };"), "null");
    assert_eq!(read(identifier, "const rest = {};"), "undefined");
    assert_eq!(read(identifier, "const rest = null;"), "undefined");
    assert_eq!(
        read(identifier, "const rest = { color: undefined };"),
        "undefined"
    );
}

#[test]
#[serial]
fn a_key_the_spread_only_inherits_is_not_a_key_it_assigns() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box color=\"red\" {{...rest}} />;"
    ));
    let identifier = &slots(&output)[0].identifier;

    assert_eq!(
        read(
            identifier,
            "const rest = Object.create({ color: 'inherited' });"
        ),
        "undefined"
    );
    assert_eq!(
        read(
            identifier,
            "const rest = Object.defineProperty({}, 'color', { value: 'hidden' });"
        ),
        "undefined"
    );
}

#[test]
#[serial]
fn a_getter_of_the_spread_runs_once_for_the_slot() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box color=\"red\" {{...rest}} />;"
    ));
    let identifier = &slots(&output)[0].identifier;

    assert_eq!(
        run(&format!(
            "let reads = 0; const rest = Object.defineProperty({{}}, 'color', {{ enumerable: true, get() {{ reads += 1; return 'x'; }} }}); const slot = {identifier}; return reads + ':' + slot;"
        )),
        "1:x"
    );
}

#[test]
#[serial]
fn keys_only_the_spread_gives_stay_untouched() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box color=\"red\" {{...rest}} id=\"x\" title={{rest.t}} />;"
    ));

    assert_eq!(slots(&output).len(), 1);
    assert!(output.code.contains("...rest"), "{}", output.code);
    assert!(output.code.contains("rest.t"), "{}", output.code);
}

#[test]
#[serial]
fn responsive_values_share_one_override_variable_with_their_own_fallbacks() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box bg={{['red', 'blue', null, '$primary']}} {{...rest}} />;"
    ));

    let found = slots(&output);
    assert_eq!(
        found
            .iter()
            .map(|slot| (slot.level, slot.fallback.as_str()))
            .collect::<Vec<_>>(),
        vec![(0, "red"), (1, "blue"), (3, "$primary")]
    );
    assert!(found.iter().all(|slot| {
        slot.identifier == found[0].identifier && slot.variable == "--background-spread-bg-0-"
    }));
    assert_eq!(
        output
            .code
            .matches("\"--background-spread-bg-0-\":")
            .count(),
        1,
        "{}",
        output.code
    );
}

#[test]
#[serial]
fn distinct_fallbacks_are_distinct_classes() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <><Box color=\"red\" {{...rest}} /><Box color=\"blue\" {{...rest}} /></>;"
    ));

    let classes: std::collections::BTreeSet<String> = output
        .styles
        .iter()
        .filter_map(|style| match style.extract(None) {
            Some(StyleProperty::Variable { class_name, .. }) => Some(class_name),
            _ => None,
        })
        .collect();
    assert_eq!(classes.len(), 2, "{classes:?}");
}

#[test]
#[serial]
fn a_later_own_undefined_does_not_bring_back_the_runtime_value() {
    let output = output(&format!(
        "{BOX}export const a = (rest, x) => <Box p={{x}} {{...rest}} />;"
    ));

    assert_eq!(slots(&output).len(), 0);
    let identifier = output
        .styles
        .iter()
        .find_map(|style| match style {
            ExtractStyleValue::Dynamic(dynamic) => {
                Some(captures::materialized(dynamic.identifier(), &output.code))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("a dynamic style: {}", output.code));
    let setup = |rest: &str| format!("const x = 5; const rest = {rest};");
    assert_eq!(read(&identifier, &setup("{}")), "5");
    assert_eq!(read(&identifier, &setup("{ p: 3 }")), "3");
    assert_eq!(read(&identifier, &setup("{ p: undefined }")), "undefined");
    assert_eq!(read(&identifier, &setup("{ p: null }")), "null");
}

#[test]
#[serial]
fn a_conditional_style_gives_way_in_each_branch() {
    let output = output(&format!(
        "{BOX}export const a = (rest, on) => <Box color={{on ? 'red' : 'blue'}} {{...rest}} />;"
    ));

    assert_eq!(slots(&output).len(), 2);
}

#[test]
#[serial]
fn style_order_stays_with_a_style_that_gives_way() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box color=\"red\" styleOrder={{4}} {{...rest}} />;"
    ));

    assert_eq!(slots(&output).len(), 1);
    assert!(output.code.contains("--4-"), "{}", output.code);
}

#[test]
#[serial]
fn a_spread_source_is_evaluated_once_in_the_order_written() {
    let output = output(&format!(
        "{BOX}export const a = (f) => <Box color=\"red\" {{...f()}} />;"
    ));

    assert_eq!(output.code.matches("f()").count(), 1, "{}", output.code);
    assert!(output.code.contains("__devupSpread0"), "{}", output.code);
}

#[test]
#[serial]
fn non_identifier_spread_sources_are_read_through_parentheses() {
    let output = output(&format!(
        "{BOX}export const a = (p) => <Box color=\"red\" {{...(p.on ?? p.b)}} />;"
    ));

    let identifier = &slots(&output)[0].identifier;
    assert_eq!(
        read(identifier, "const p = {on:null,b:{color:'blue'}};"),
        "\"blue\""
    );
    assert_eq!(output.code.matches("p.on ?? p.b").count(), 1);
}

#[test]
#[serial]
fn an_object_literal_holding_a_spread_or_a_computed_key_may_carry_the_key() {
    let output = output(&format!(
        "{BOX}export const a = (rest) => <Box color=\"red\" {{...{{ ...rest }}}} />;\nexport const b = (k) => <Box color=\"red\" {{...{{ [k]: 1 }}}} />;"
    ));

    let found = slots(&output);
    assert_eq!(found.len(), 2);
    let actual = super::whole::evaluate(
        &format!(
            "{BOX}export const a=(rest)=><Box color=\"red\" {{...{{...rest}}}}/>; export const b=(k)=><Box color=\"red\" {{...{{[k]:1}}}}/>;"
        ),
        "[a({color:'blue'}).props.color,b('color').props.color]",
    );
    assert_eq!(actual.element, serde_json::json!(["blue", 1]));
}

#[test]
#[serial]
fn a_key_that_is_not_an_identifier_is_read_by_name() {
    let rendered = code(&format!(
        "{BOX}export const a = (rest) => <Box background-color=\"red\" {{...rest}} />;"
    ));

    assert!(rendered.contains("[\"background-color\"]"), "{rendered}");
}
