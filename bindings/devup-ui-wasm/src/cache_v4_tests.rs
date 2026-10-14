use super::*;
use rstest::rstest;
use serial_test::serial;
#[path = "cache_admission_tests.rs"]
mod admission;
mod collisions;
mod configuration;
mod special;

fn compile() -> (String, String) {
    let output = code_extract_internal(
        "fresh.tsx",
        "import {Box} from '@devup-ui/react';export const View=(p)=><Box color={p.color} p={2}/>;",
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    (
        output.code(),
        with_style_sheet(|sheet| sheet.create_css(Some("fresh.tsx"), false)),
    )
}

fn snapshot() -> serde_json::Value {
    serde_json::from_str(&export_sheet_internal().unwrap_or_else(|error| panic!("{error}")))
        .unwrap_or_else(|error| panic!("{error}"))
}

fn import(value: serde_json::Value) -> Result<(), String> {
    import_sheet_internal(serde_json::from_value(value).unwrap_or_else(|_| cache_restore::absent()))
}

#[rstest]
#[case(None)]
#[case(Some(0))]
#[case(Some(3))]
#[case(Some(5))]
#[serial]
fn serialized_old_or_future_generation_restores_nothing(#[case] version: Option<u8>) {
    // Given: an independently compiled cold output and a poisoned old snapshot.
    reset_build_state_internal();
    let cold = compile();
    reset_build_state_internal();
    let mut poison = StyleSheet::default();
    poison.add_property(
        "old-alias",
        "color",
        0,
        "magenta",
        None,
        None,
        Some("fresh.tsx"),
    );
    import_sheet_internal(poison).unwrap_or_else(|error| panic!("{error}"));
    let mut value: serde_json::Value =
        serde_json::from_str(&export_sheet_internal().unwrap_or_else(|error| panic!("{error}")))
            .unwrap_or_else(|error| panic!("{error}"));
    let object = value
        .as_object_mut()
        .unwrap_or_else(|| panic!("snapshot object"));
    match version {
        Some(version) => {
            object.insert("atomNamingVersion".into(), version.into());
        }
        None => {
            object.remove("atomNamingVersion");
        }
    }
    reset_build_state_internal();
    // When: serialized admission runs through the existing typed internal API.
    let incoming = serde_json::from_value(value).unwrap_or_else(|error| panic!("{error}"));
    import_sheet_internal(incoming).unwrap_or_else(|error| panic!("{error}"));
    // Then: no old rule/claim survives; complete JS and CSS are the cold bytes.
    assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    assert_eq!(compile(), cold);
    reset_build_state_internal();
}

#[rstest]
#[case("names")]
#[case("atom_plan")]
#[case("properties")]
#[case("css")]
#[case("keyframes")]
#[case("global_css_files")]
#[case("imports")]
#[case("font_faces")]
#[case("sourceIds")]
#[case("classMap")]
#[case("fileMap")]
#[serial]
fn current_snapshot_requires_even_empty_restore_fields(#[case] field: &str) {
    // Given: independent cold output and a current snapshot with one missing field.
    reset_build_state_internal();
    let cold = compile();
    let mut value = snapshot();
    value
        .as_object_mut()
        .unwrap_or_else(|| panic!("snapshot object"))
        .remove(field);
    reset_build_state_internal();
    // When: the partially written object reaches serialized admission.
    assert_eq!(import(value), Ok(()));
    // Then: no partial state survives and complete output is cold.
    assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    assert_eq!(compile(), cold);
    reset_build_state_internal();
}

#[rstest]
#[case(serde_json::json!(null))]
#[case(serde_json::json!(42))]
#[case(serde_json::json!([]))]
#[case(serde_json::json!({"atomNamingVersion":"4"}))]
#[case(serde_json::json!({"atomNamingVersion":4,"properties":false}))]
#[serial]
fn malformed_typed_cache_objects_are_absent(#[case] value: serde_json::Value) {
    // Given: a separately compiled cold fixture and malformed serialized data.
    reset_build_state_internal();
    let cold = compile();
    reset_build_state_internal();
    // When: the typed deserializer receives the malformed value.
    assert_eq!(import(value), Ok(()));
    // Then: extraction and current rewrite remain usable and byte-exact.
    assert_eq!(compile(), cold);
    assert_eq!(snapshot()["atomNamingVersion"], 4);
    reset_build_state_internal();
}

#[rstest]
#[case(false, false)]
#[case(false, true)]
#[case(true, false)]
#[case(true, true)]
#[serial]
fn companion_mismatch_undoes_only_cache_restore(#[case] before: bool, #[case] class_map: bool) {
    // Given: a poisoned authoritative snapshot and genuine fresh D9 configuration.
    reset_build_state_internal();
    seed_file_map(vec!["fresh.tsx".into()]);
    compile();
    let mut value = snapshot();
    value["sourceIds"]["fresh.tsx"] = 777.into();
    value["fileMap"]["fresh.tsx"] = 888.into();
    value["classMap"] = serde_json::json!({"fresh.tsx":{"poison":999}});
    reset_build_state_internal();
    seed_file_map(vec!["configured.tsx".into()]);
    let cold = compile();
    reset_build_state_internal();
    seed_file_map(vec!["configured.tsx".into()]);
    let mismatch = || {
        if class_map {
            cache_restore::classes(Some(BTreeMap::new()));
        } else {
            cache_restore::files(Some(BTreeMap::new()));
        }
    };
    // When: an old/missing companion disagrees before or after sheet admission.
    if before {
        mismatch();
    }
    assert_eq!(import(value), Ok(()));
    if !before {
        mismatch();
    }
    // Then: cached IDs/counters/rules disappear, but prepass configuration remains.
    assert_eq!(
        css::file_map::get_original_ids(),
        BTreeMap::from([("configured.tsx".into(), 0)])
    );
    assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    assert_eq!(with_class_map(HashMap::len), 0);
    assert_eq!(compile(), cold);
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn complete_current_restore_works_in_both_companion_orders(#[case] before: bool) {
    // Given: a real current cache including original-owner counters and sparse variables.
    reset_build_state_internal();
    seed_file_map(vec!["a.tsx".into(), "fresh.tsx".into()]);
    let output = compile();
    let value = snapshot();
    let classes: sheet::cache_snapshot::ClassMap =
        serde_json::from_value(value["classMap"].clone()).unwrap_or_else(|error| panic!("{error}"));
    let files: sheet::cache_snapshot::FileMap =
        serde_json::from_value(value["fileMap"].clone()).unwrap_or_else(|error| panic!("{error}"));
    reset_build_state_internal();
    // When: only the complete sheet can authorize companion state in either order.
    if before {
        cache_restore::classes(Some(classes.clone()));
        cache_restore::files(Some(files.clone()));
    }
    assert_eq!(import(value.clone()), Ok(()));
    if !before {
        cache_restore::classes(Some(classes));
        cache_restore::files(Some(files));
    }
    // Then: the entire naming snapshot and full JS/CSS round trip exactly.
    assert_eq!(snapshot(), value);
    assert_eq!(compile(), output);
    reset_build_state_internal();
}

#[test]
#[serial]
fn companions_without_sheet_authority_never_allocate_poisoned_state() {
    // Given: independent cold output and arbitrary legacy map data.
    reset_build_state_internal();
    let cold = compile();
    reset_build_state_internal();
    // When: raw companions are imported without a sheet.
    cache_restore::classes(Some(BTreeMap::from([(
        "fresh.tsx".into(),
        BTreeMap::from([("poison".into(), 999)]),
    )])));
    cache_restore::files(Some(BTreeMap::from([("fresh.tsx".into(), 888)])));
    // Then: neither map becomes live state and complete output remains cold.
    assert_eq!(css::file_map::get_file_map().len(), 0);
    assert_eq!(with_class_map(HashMap::len), 0);
    assert_eq!(compile(), cold);
    reset_build_state_internal();
}
