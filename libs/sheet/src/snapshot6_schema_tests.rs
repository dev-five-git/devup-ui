use super::raw::{Object, Raw};
use super::test_support::*;

pub(super) fn object_paths(value: &Value, path: &str, result: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            if !object.is_empty() {
                result.push(path.into());
            }
            for (key, value) in object {
                object_paths(
                    value,
                    &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                    result,
                );
            }
        }
        Value::Array(items) => {
            for (index, value) in items.iter().enumerate() {
                object_paths(value, &format!("{path}/{index}"), result);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

pub(super) fn duplicate_at(raw: &mut Raw, pointer: &str) {
    let mut current = raw;
    for token in pointer.split('/').skip(1) {
        let token = token.replace("~1", "/").replace("~0", "~");
        current = match current {
            Raw::Object(Object(entries)) => {
                &mut entries
                    .iter_mut()
                    .find(|(key, _)| key == &token)
                    .required("key")
                    .1
            }
            Raw::Array(items) => &mut items[token.parse::<usize>().required("index")],
            _ => panic!("object path"),
        };
    }
    let Raw::Object(Object(entries)) = current else {
        panic!("object")
    };
    entries.push(entries[0].clone());
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[serial_test::serial]
fn decoded_key_duplicates_reject_at_every_object_layer_when_genuine_wire_is_damaged(
    #[case] family: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(family);
    sheet.add_font_face(
        "literal",
        &BTreeMap::from([("font-family".into(), "literal".into())]),
    );
    let bytes = encoded(&mut sheet);
    let value: Value = serde_json::from_slice(&bytes).required("inspection");
    let mut paths = Vec::new();
    object_paths(&value, "", &mut paths);
    let before = observe(&sheet);
    for path in paths {
        let mut raw: Raw = serde_json::from_slice(&bytes).required("raw");
        duplicate_at(&mut raw, &path);
        // When
        let result = parse(&serde_json::to_vec(&raw).required("duplicate wire"));
        // Then
        assert!(
            matches!(result, Err(EvidenceError::Schema)),
            "duplicate {path}"
        );
    }
    assert_eq!(observe(&sheet), before);
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[serial_test::serial]
fn every_required_struct_field_rejects_when_omitted_including_null_false_and_empty(
    #[case] family: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(family);
    sheet.add_property(
        "literal",
        "opacity",
        0,
        "1",
        Some(&css::style_selector::StyleSelector::At {
            kind: css::style_selector::AtRuleKind::Media,
            query: "print".into(),
            selector: None,
            outer: vec![],
            file: None,
        }),
        None,
        Some("literal"),
    );
    let value = packet(&mut sheet);
    let mut paths = Vec::new();
    object_paths(&value, "", &mut paths);
    for path in paths {
        let object = value
            .pointer(&path)
            .required("object")
            .as_object()
            .required("map");
        let required = path.is_empty()
            || object.contains_key("c")
            || object.contains_key("proof")
            || object.contains_key("seed")
            || object.contains_key("allocation")
            || object.contains_key("context")
            || object.contains_key("source_file")
            || object.contains_key("parent")
            || object.contains_key("kind")
            || object.contains_key("resolution")
            || object.contains_key("font_size")
            || object.contains_key("frames")
            || object.contains_key("namespace")
            || object.contains_key("mode")
            || object.contains_key("canonical")
            || object.contains_key("counters")
            || object.contains_key("order")
            || object.contains_key("important")
            || object.contains_key("bucket")
            || object.contains_key("original")
            || object.contains_key("at")
            || object.contains_key("members")
            || object.contains_key("record")
            || object.contains_key("steps")
            || object.contains_key("children")
            || object.contains_key("source")
            || (object.contains_key("from") && object.contains_key("property"))
            || object.contains_key("selector");
        if !required {
            continue;
        }
        for key in object.keys() {
            let mut damaged = value.clone();
            damaged
                .pointer_mut(&path)
                .required("object")
                .as_object_mut()
                .required("map")
                .remove(key);
            // When
            let result = parse(&serde_json::to_vec(&damaged).required("missing field"));
            // Then
            assert!(
                matches!(result, Err(EvidenceError::Schema)),
                "missing {path}/{key}"
            );
        }
    }
}

#[rstest::rstest]
#[case("4")]
#[case("5")]
#[case("7")]
#[case("6.0")]
#[case("6e0")]
#[case("-6")]
#[case("18446744073709551616")]
#[serial_test::serial]
fn version_rejects_when_not_exact_unsigned_six(#[case] version: &str) {
    // Given
    let _guard = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty");
    let bytes = String::from_utf8(encoded(&mut sheet)).required("json");
    let damaged = bytes.replace(
        "\"atomNamingVersion\":6",
        &format!("\"atomNamingVersion\":{version}"),
    );
    // When
    let result = parse(damaged.as_bytes());
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)));
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[serial_test::serial]
fn raw_syntax_rejects_when_truncated_trailing_escaped_duplicate_or_cache4(#[case] damage: u8) {
    // Given
    let _guard = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty");
    let mut bytes = encoded(&mut sheet);
    match damage {
        0 => {
            bytes.pop();
        }
        1 => bytes.extend_from_slice(b" true"),
        2 => bytes
            .splice(1..1, b"\"atomNamingVersio\\u006e\":6,".iter().copied())
            .for_each(drop),
        3 => bytes = serde_json::to_vec(&sheet.export_snapshot()).required("actual4"),
        _ => panic!("damage"),
    }
    // When
    let result = parse(&bytes);
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)));
}
