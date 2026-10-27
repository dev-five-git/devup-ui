use std::collections::HashMap;

use bimap::BiHashMap;
use css::{file_map, slot_reservation};
use rstest::rstest;
use serial_test::serial;

use crate::*;

pub(super) fn capture_owner() -> Result<[String; 5], String> {
    Ok([
        export_sheet_internal()?,
        export_class_map_internal()?,
        export_file_map_internal()?,
        export_canonical_map_internal()?,
        reservation::export_reservation_state_internal()?,
    ])
}

fn restore_owner(state: &[String; 5]) -> Result<(), serde_json::Error> {
    import_sheet_json_internal(&state[0])?;
    import_class_map_json_internal(&state[1])?;
    import_file_map_json_internal(&state[2])?;
    import_canonical_map_json_internal(&state[3])?;
    import_reservation_state_json_internal(&state[4])
}

pub(super) fn install_owner(name: &str) -> Result<[String; 5], Box<dyn std::error::Error>> {
    reset_build_state_internal();
    css::debug::set_debug(false);
    import_sheet_internal(serde_json::from_value(serde_json::json!({
        "atom_plan": [name],
        "properties": {name: {"0": {"0": [{"c": "owner", "p": "color", "v": "red"}]}}},
        "css": {name: [{"css": concat!(".owner", "{", "color:red", "}")}]},
        "keyframes": {name: {"motion": {"from": [["opacity", "0"]]}}},
        "global_css_files": [name], "imports": {name: ["url('owner.css')"]},
        "font_faces": {name: [{"font-family": "owner", "src": "url('owner.woff')"}]},
        "layer_orders": {name: ["owner"]}, "atomNamingVersion": 2
    }))?);
    css::class_map::set_class_map(HashMap::from([(
        name.into(),
        HashMap::from([("allocated".into(), 7)]),
    )]));
    file_map::set_file_map(BiHashMap::from_iter([(name.into(), 0)]));
    import_canonical_map_internal(HashMap::from([("child".into(), name.into())]));
    reservation::import_reservation_state_internal(serde_json::from_str(
        r#"{"phase":"Live","misses":17,"suppressed":3}"#,
    )?);
    Ok(capture_owner()?)
}

#[test]
#[serial]
fn owner_files_preserve_both_directions_when_exported() -> Result<(), Box<dyn std::error::Error>> {
    // Given a real bimap export with empty, numeric-looking and escaped keys.
    let _state = TestStateGuard::new();
    let max = usize::try_from(u32::MAX)?;
    let expected = BiHashMap::from_iter([
        (String::new(), 0),
        ("01".into(), 1),
        ("1e3".into(), 255),
        ("4294967295\0𐀀\"\\".into(), max),
    ]);
    file_map::set_file_map(expected.clone());
    let exported = export_file_map_internal()?;
    file_map::set_file_map(BiHashMap::new());
    // When decoding the matching Rust export.
    import_file_map_json_internal(&exported)?;
    // Then the raw export and forward/reverse lookup contracts survive.
    assert_eq!(export_file_map_internal()?, exported);
    assert_eq!(file_map::get_file_map(), expected);
    assert_eq!(file_map::get_filename_by_file_num(max), "4294967295\0𐀀\"\\");
    assert_eq!(file_map::get_file_num_by_filename("4294967295\0𐀀\"\\"), max);
    Ok(())
}

#[test]
#[serial]
fn owner_canonical_preserves_targets_when_exported() -> Result<(), Box<dyn std::error::Error>> {
    // Given empty, numeric-looking, global and escaped canonical targets.
    let _state = TestStateGuard::new();
    let expected = HashMap::from([
        (String::new(), "01\0𐀀\"\\".into()),
        ("01".into(), String::new()),
        ("1e3".into(), "4294967295".into()),
        ("shared".into(), file_map::GLOBAL_BUCKET.into()),
    ]);
    import_canonical_map_internal(expected.clone());
    let exported = export_canonical_map_internal()?;
    import_canonical_map_internal(HashMap::new());
    // When decoding the matching Rust export.
    import_canonical_map_json_internal(&exported)?;
    // Then targets retain their meaning rather than being normalized.
    assert_eq!(export_canonical_map_internal()?, exported);
    assert_eq!(file_map::get_canonical_map(), expected);
    assert_eq!(file_map::canonical(""), "01\0𐀀\"\\");
    assert_eq!(file_map::canonical("01"), "");
    assert_eq!(file_map::canonical("absent"), "absent");
    assert!(file_map::is_global("shared"));
    Ok(())
}

#[test]
#[serial]
fn owner_maps_replace_sentinels_when_empty_exports_resume() -> Result<(), Box<dyn std::error::Error>>
{
    // Given genuinely exported empty maps and different installed sentinels.
    let _state = TestStateGuard::new();
    reset_build_state_internal();
    let files = export_file_map_internal()?;
    let canonical = export_canonical_map_internal()?;
    install_owner("sentinel")?;
    // When importing matching empty exports, then old entries are replaced.
    import_file_map_json_internal(&files)?;
    import_canonical_map_json_internal(&canonical)?;
    assert_eq!(export_file_map_internal()?, files);
    assert_eq!(export_canonical_map_internal()?, canonical);
    Ok(())
}

#[rstest]
#[case("Legacy", 17)]
#[case("Scratch", 17)]
#[case("Live", 18)]
#[serial]
fn owner_reservation_preserves_phase_when_allocating_later(
    #[case] phase: &str,
    #[case] expected: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given each phase and both counters at the wasm32 boundary.
    let _state = TestStateGuard::new();
    reset_build_state_internal();
    let max = usize::try_from(u32::MAX)?;
    reservation::import_reservation_state_internal(serde_json::from_str(&format!(
        r#"{{"phase":"{phase}","misses":{max},"suppressed":{max}}}"#
    ))?);
    let exported = reservation::export_reservation_state_internal()?;
    slot_reservation::reset();
    // When decoding matching Rust bytes, then every opaque field survives.
    import_reservation_state_json_internal(&exported)?;
    assert_eq!(reservation::export_reservation_state_internal()?, exported);
    assert_eq!(slot_reservation::unreserved_slot_count(), max);
    assert_eq!(slot_reservation::suppressed_row_count(), max);
    reservation::import_reservation_state_internal(serde_json::from_str(&format!(
        r#"{{"phase":"{phase}","misses":17,"suppressed":3}}"#
    ))?);
    let saved = reservation::export_reservation_state_internal()?;
    slot_reservation::reset();
    import_reservation_state_json_internal(&saved)?;
    let first = file_map::get_file_num_by_filename("later");
    let repeat = file_map::get_file_num_by_filename("later");
    assert_eq!((first, repeat), (0, 0));
    assert_eq!(slot_reservation::unreserved_slot_count(), expected);
    assert_eq!(slot_reservation::suppressed_row_count(), 3);
    Ok(())
}

#[test]
#[serial]
fn owner_remaining_restores_aba_when_allocations_follow() -> Result<(), Box<dyn std::error::Error>>
{
    // Given independent A/B exports and A's uninterrupted later allocation.
    let _state = TestStateGuard::new();
    let a = install_owner("A")?;
    let expected_file = file_map::get_file_num_by_filename("next");
    let expected_class = css::sheet_to_variable_name("gap", 0, None);
    let expected = capture_owner()?;
    let b = install_owner("B")?;
    // When owners resume A/B/A with the same reset and five-import sequence.
    for snapshot in [&a, &b, &a] {
        reset_build_state_internal();
        restore_owner(snapshot)?;
        // Then each raw handoff retains every field, not only the touched map.
        assert_eq!(&capture_owner()?, snapshot);
    }
    assert_eq!(file_map::canonical("child"), "A");
    assert_eq!(file_map::get_file_num_by_filename("next"), expected_file);
    assert_eq!(css::sheet_to_variable_name("gap", 0, None), expected_class);
    assert_eq!(capture_owner()?, expected);
    Ok(())
}
