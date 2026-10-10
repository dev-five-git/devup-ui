use super::advanced_support::*;
use super::raw::Raw;

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[serial_test::serial]
fn advanced_required_nested_fields_reject_when_real_baseline_outer_or_history_field_is_missing(
    #[case] mode: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = advanced_sheet(mode);
    let value = packet(&mut sheet);
    let mut exercised = 0;
    for path in paths(&value) {
        let object = value
            .pointer(&path)
            .required("object")
            .as_object()
            .required("map");
        let required = object.contains_key("kind")
            || object.contains_key("mode")
            || object.contains_key("config")
            || object.contains_key("single_css")
            || object.contains_key("source")
            || object.contains_key("parent")
            || object.contains_key("resolution")
            || object.contains_key("prefix")
            || object.contains_key("font_size")
            || object.contains_key("input")
            || object.contains_key("namespace");
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
            exercised += 1;
        }
    }
    assert!(exercised > 20);
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[serial_test::serial]
fn advanced_nested_duplicates_reject_when_root_is_valid_and_only_child_object_is_duplicated(
    #[case] mode: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = advanced_sheet(mode);
    let bytes = encoded(&mut sheet);
    let value: Value = serde_json::from_slice(&bytes).required("inspection");
    for path in paths(&value).into_iter().filter(|path| !path.is_empty()) {
        let mut raw: Raw = serde_json::from_slice(&bytes).required("valid raw");
        super::schema_tests::duplicate_at(&mut raw, &path);
        // When
        let result = parse(&serde_json::to_vec(&raw).required("nested duplicate"));
        // Then
        assert!(
            matches!(result, Err(EvidenceError::Schema)),
            "nested duplicate {path}"
        );
    }
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[case(6)]
#[case(7)]
#[case(8)]
#[case(9)]
#[case(10)]
#[case(11)]
#[case(12)]
#[case(13)]
#[serial_test::serial]
fn unknown_kind_variant_or_payload_rejects_at_parse_when_advanced_wire_is_damaged(
    #[case] damage: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = advanced_sheet(1);
    let mut value = packet(&mut sheet);
    let record = at_record_path(&value);
    match damage {
        0 => value
            .pointer_mut(&format!("{record}/s/At/kind"))
            .required("kind")
            .clone_from(&json!("FutureKind")),
        1 => value
            .pointer_mut(&format!("{record}/s/At/outer/0/kind"))
            .required("outer kind")
            .clone_from(&json!("FutureKind")),
        2 => value["evidence"]["config"]["mode"] = json!("FutureMode"),
        3 => {
            value["evidence"]["baseline"][0]["proof"]["emission"]["seed"]["body"] =
                json!({"FutureBody":{}});
        }
        4 => {
            value["evidence"]["baseline"][0]["proof"]["allocation"]["allocation"]["address"] =
                json!({"FutureAddress":{}});
        }
        5 => {
            value["evidence"]["baseline"][0]["proof"]["allocation"]["allocation"]["address"] =
                json!({"Baseline":[]});
        }
        6 => value
            .pointer_mut(&format!("{record}/s"))
            .required("selector")
            .clone_from(&json!({"At":"not-an-object"})),
        7 => value["evidence"]["phase"] = json!({"FuturePhase":[]}),
        8 => {
            value["evidence"]["baseline"][0]["proof"]["emission"]["materialization"] =
                json!("FutureMaterialization");
        }
        9 => {
            let declaration = paths(&value)
                .into_iter()
                .find(|path| {
                    value
                        .pointer(path)
                        .required("node")
                        .get("resolution")
                        .is_some()
                })
                .required("declaration");
            value
                .pointer_mut(&format!("{declaration}/resolution"))
                .required("resolution")
                .clone_from(&json!("FutureResolution"));
        }
        10 => {
            value["evidence"]["baseline"][0]["proof"]["emission"]["expansion"] =
                json!({"FutureExpansion":{}});
        }
        11 => value
            .pointer_mut(&format!("{record}/s"))
            .required("selector")
            .clone_from(&json!({"FutureSelector":[]})),
        12 => {
            value["evidence"]["baseline"][0]["proof"]["allocation"]["context"]["file"] =
                json!({"FutureFile":0});
        }
        13 => {
            value["evidence"]["baseline"][0]["proof"]["emission"]["materialization"] =
                json!({"Complete":[]});
        }
        _ => panic!("damage"),
    }
    // When
    let result = parse(&serde_json::to_vec(&value).required("damage"));
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)));
}
