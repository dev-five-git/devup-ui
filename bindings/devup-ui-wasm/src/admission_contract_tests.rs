use super::*;
use exact_test_support::{authority, fixture};
use rstest::rstest;
use serial_test::serial;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[rstest]
#[case("set_debug", || set_debug(true))]
#[case("set_prefix", || set_prefix(Some("changed".into())))]
#[case("set_naming_root", || set_naming_root(Some("/changed".into()), None))]
#[case("set_atom_hoist", || set_atom_hoist(Some(2)))]
#[case("import_canonical_map_internal", || import_canonical_map_internal(HashMap::new()))]
#[case("import_file_routes_internal", || import_file_routes_internal(HashMap::new()))]
#[case("register_theme_internal", || register_theme_internal(sheet::theme::Theme::default()))]
#[case("register_shorthands_internal", || register_shorthands_internal(BTreeMap::new()))]
#[case("set_module_resolver", || set_module_resolver_internal(None))]
#[case("seed_file_map", || seed_file_map(vec!["changed.tsx".into()]))]
#[case("reset_build_state_internal", reset_build_state_internal)]
#[case("import_sheet_internal", || { import_sheet_internal(StyleSheet::default()).unwrap_or_else(|error| panic!("sheet import failed: {error}")); })]
#[case("cache_restore::clear", cache_restore::clear)]
#[case("cache_restore::seeded", || cache_restore::seeded(&["changed.tsx".into()]))]
#[case("cache_restore::classes", || cache_restore::classes(None))]
#[case("cache_restore::files", || cache_restore::files(None))]
#[case("cache_restore::import", || { cache_restore::import(StyleSheet::default()).unwrap_or_else(|error| panic!("cache import failed: {error}")); })]
#[case("StyleSheet::set_theme", || StyleSheet::default().set_theme(sheet::theme::Theme::default()))]
#[serial]
fn guarded_entry_panics_with_exact_message_before_mutation(
    #[case] operation: &str,
    #[case] administer: fn(),
) {
    // Given
    fixture();
    set_prefix(Some("retained".into()));
    set_atom_hoist(Some(7));
    register_shorthands_internal(BTreeMap::from([("custom".into(), vec!["color".into()])]));
    let before = authority();
    cache_names::record(&Err("sticky retained".into()));
    // When
    let panic = match catch_unwind(AssertUnwindSafe(|| {
        let _: Result<(), ()> = css::exact_attempt::with_exclusive_attempt(|| {
            administer();
            Ok(())
        });
    })) {
        Err(payload) => payload,
        Ok(()) => panic!("guarded administration did not panic"),
    };
    // Then
    let message = match panic.downcast_ref::<String>() {
        Some(message) => message.as_str(),
        None => *panic
            .downcast_ref::<&str>()
            .unwrap_or_else(|| panic!("guarded administration panic is not a string")),
    };
    assert_eq!(message, format!("{operation}: an exact attempt is active"));
    assert_eq!(authority(), before);
    assert_eq!(get_prefix(), Some("retained".into()));
    assert_eq!(css::atom_hoist::atom_hoist_threshold(), Some(7));
    assert_eq!(
        css::get_custom_shorthand_names(),
        vec!["custom".to_string()]
    );
    assert_eq!(cache_names::check(), Err("sticky retained".into()));
    register_shorthands_internal(BTreeMap::new());
    reset_build_state_internal();
}

#[test]
#[serial]
fn sheet_entry_marker_recovers_after_unwind_and_allows_sequential_calls() {
    // Given
    reset_build_state_internal();
    let _poison_cleanup = exact_test_support::SheetPoisonCleanup::new();
    // When
    let result = catch_unwind(AssertUnwindSafe(|| {
        with_style_sheet_mut(|_| panic!("sheet unwind"));
    }));
    // Then
    assert!(result.is_err());
    with_admission(|| {
        with_style_sheet_mut(|sheet| sheet.add_css("after.tsx", concat!("body{", "color:red}")));
        with_style_sheet(|sheet| assert!(sheet.css.contains_key("after.tsx")));
    });
    reset_build_state_internal();
}

#[test]
#[serial]
fn failed_extraction_leaves_cache_authority_and_ordered_fresh_seed_replay_unchanged() {
    // Given
    reset_build_state_internal();
    let serialized = export_sheet_internal()
        .unwrap_or_else(|error| panic!("cache fixture export failed: {error}"));
    seed_file_map(vec!["before.tsx".into()]);
    import_sheet_internal(
        serde_json::from_str(&serialized)
            .unwrap_or_else(|error| panic!("cache fixture decoding failed: {error}")),
    )
    .unwrap_or_else(|error| panic!("cache fixture import failed: {error}"));
    seed_file_map(vec!["b.tsx".into(), "b.tsx".into()]);
    seed_file_map(vec!["a.tsx".into(), "b.tsx".into()]);
    let before = authority();
    // When
    let failed = exact_test_support::compile(
        "broken.tsx",
        "import {Box} from '@devup-ui/react'; const x=<Box",
    );
    // Then
    assert!(failed.is_err());
    assert_eq!(authority(), before);
    cache_restore::classes(None);
    assert_eq!(
        css::file_map::get_original_ids(),
        BTreeMap::from([
            ("before.tsx".into(), 0),
            ("b.tsx".into(), 1),
            ("a.tsx".into(), 2),
        ])
    );
    assert_eq!(
        css::file_map::get_file_map(),
        [
            ("before.tsx".into(), 0),
            ("b.tsx".into(), 1),
            ("a.tsx".into(), 2),
        ]
        .into_iter()
        .collect()
    );
    reset_build_state_internal();
}
