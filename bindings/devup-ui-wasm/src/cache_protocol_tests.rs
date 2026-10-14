use super::*;
use rstest::rstest;
use serial_test::serial;
use std::collections::HashSet;

#[test]
#[serial]
fn source_ids_roundtrip_restores_dynamic_names_when_extracting_in_reverse_order() {
    // Given
    reset_build_state_internal();
    css::file_map::seed_file_numbers(&["a.tsx".into(), "z.tsx".into()]);
    let source = "import {Box} from '@devup-ui/react'; export const View=({pad})=><Box p={pad}/>;";
    let before = code_extract_internal(
        "z.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let ids = css::file_map::get_original_ids();
    assert_eq!(
        ids,
        BTreeMap::from([("a.tsx".into(), 0), ("z.tsx".into(), 1)])
    );
    let exported = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let files = css::file_map::get_file_map();
    let classes = with_class_map(Clone::clone);
    let imported: StyleSheet =
        serde_json::from_str(&exported).unwrap_or_else(|error| panic!("{error}"));
    reset_build_state_internal();
    // When
    import_sheet_internal(imported).unwrap_or_else(|error| panic!("{error}"));
    set_file_map(files);
    set_class_map(classes);
    let restored_ids = css::file_map::get_original_ids();
    let after = code_extract_internal(
        "z.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(restored_ids, ids);
    assert_eq!(after.code(), before.code());
    reset_build_state_internal();
}

#[rstest]
#[case(false, false)]
#[case(false, true)]
#[case(true, true)]
#[serial]
fn legacy_serialized_import_is_cold_and_rewritten_current(
    #[case] with_styles: bool,
    #[case] atom_mode: bool,
) {
    // Given
    reset_build_state_internal();
    let mut legacy = StyleSheet::default();
    if with_styles {
        legacy.add_property("old", "color", 0, "red", None, None, Some("private.tsx"));
    }
    let legacy_json = serde_json::to_string(&legacy).unwrap_or_else(|error| panic!("{error}"));
    let local = StyleSheet::default().create_css(Some("private.tsx"), false);
    let global = StyleSheet::default().create_css(None, false);
    css::atom_hoist::set_atom_hoist(atom_mode.then_some(2));
    let imported = serde_json::from_str(&legacy_json).unwrap_or_else(|error| panic!("{error}"));
    import_sheet_internal(imported).unwrap_or_else(|error| panic!("{error}"));
    // When
    let exported = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_ne!(exported, legacy_json);
    let json: serde_json::Value =
        serde_json::from_str(&exported).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(json["atomNamingVersion"], 4);
    let restored: StyleSheet =
        serde_json::from_str(&exported).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(restored.atom_plan, None);
    assert_eq!(restored.create_css(Some("private.tsx"), false), local);
    assert_eq!(restored.create_css(None, false), global);
    import_sheet_internal(restored).unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        exported
    );
    reset_build_state_internal();
}

#[test]
#[serial]
fn exported_generation_roundtrips_frozen_atom_placement() {
    // Given
    reset_build_state_internal();
    css::atom_hoist::set_atom_hoist(Some(2));
    css::file_routes::set_file_routes(HashMap::from([
        ("shared.tsx".into(), HashSet::from([0, 1])),
        ("private.tsx".into(), HashSet::from([0])),
    ]));
    for file in ["shared.tsx", "private.tsx"] {
        code_extract_internal(
            file,
            "import {Box} from '@devup-ui/react'; export const x=<Box color=\"red\"/>;",
            "@devup-ui/react",
            "df".into(),
            false,
            false,
            false,
            HashMap::new(),
        )
        .unwrap_or_else(|error| panic!("{error}"));
    }
    let exported = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let global = with_style_sheet(|sheet| sheet.create_css(None, false));
    let local = with_style_sheet(|sheet| sheet.create_css(Some("private.tsx"), false));
    let imported = serde_json::from_str(&exported).unwrap_or_else(|error| panic!("{error}"));
    reset_build_state_internal();
    // When
    import_sheet_internal(imported).unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(global.matches("color:red").count(), 1);
    assert_eq!(local.matches("color:red").count(), 1);
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        global
    );
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("private.tsx"), false)),
        local
    );
    assert!(
        !with_style_sheet(|sheet| sheet.create_css(Some("shared.tsx"), false))
            .contains("color:red")
    );
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        exported
    );
    reset_build_state_internal();
}
