use super::*;
use serial_test::serial;

fn during_exact(build: impl FnOnce()) {
    let result: Result<(), ()> = css::exact_attempt::with_exclusive_attempt(|| {
        build();
        Ok(())
    });
    assert_eq!(result, Ok(()));
}

#[test]
#[serial]
#[should_panic(expected = "set_debug: an exact attempt is active")]
fn debug_rejects_owner_administration() {
    during_exact(|| set_debug(true));
}

#[test]
#[serial]
#[should_panic(expected = "set_prefix: an exact attempt is active")]
fn prefix_rejects_owner_administration() {
    during_exact(|| set_prefix(Some("other".into())));
}

#[test]
#[serial]
#[should_panic(expected = "set_naming_root: an exact attempt is active")]
fn naming_root_rejects_owner_administration() {
    during_exact(|| set_naming_root(Some("/other".into()), None));
}

#[test]
#[serial]
#[should_panic(expected = "import_canonical_map_internal: an exact attempt is active")]
fn canonical_import_rejects_owner_administration() {
    during_exact(|| import_canonical_map_internal(HashMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "import_file_routes_internal: an exact attempt is active")]
fn routes_import_rejects_owner_administration() {
    during_exact(|| import_file_routes_internal(HashMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "set_atom_hoist: an exact attempt is active")]
fn hoist_rejects_owner_administration() {
    during_exact(|| set_atom_hoist(Some(2)));
}

#[test]
#[serial]
#[should_panic(expected = "register_theme_internal: an exact attempt is active")]
fn theme_registration_rejects_owner_administration() {
    during_exact(|| register_theme_internal(sheet::theme::Theme::default()));
}

#[test]
#[serial]
#[should_panic(expected = "register_shorthands_internal: an exact attempt is active")]
fn shorthands_reject_owner_administration() {
    during_exact(|| register_shorthands_internal(BTreeMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "set_module_resolver: an exact attempt is active")]
fn resolver_selection_rejects_owner_administration() {
    during_exact(|| set_module_resolver_internal(None));
}

#[test]
#[serial]
#[should_panic(expected = "seed_file_map: an exact attempt is active")]
fn seed_rejects_owner_administration() {
    during_exact(|| seed_file_map(vec!["other.tsx".into()]));
}

#[test]
#[serial]
#[should_panic(expected = "reset_build_state_internal: an exact attempt is active")]
fn reset_rejects_owner_administration() {
    during_exact(reset_build_state_internal);
}

#[test]
#[serial]
#[should_panic(expected = "import_sheet_internal: an exact attempt is active")]
fn sheet_import_rejects_owner_administration() {
    during_exact(|| {
        import_sheet_internal(StyleSheet::default())
            .unwrap_or_else(|error| panic!("sheet import failed: {error}"));
    });
}

#[test]
#[serial]
#[should_panic(expected = "cache_restore::clear: an exact attempt is active")]
fn companion_clear_rejects_owner_administration() {
    during_exact(cache_restore::clear);
}

#[test]
#[serial]
#[should_panic(expected = "cache_restore::seeded: an exact attempt is active")]
fn companion_seed_rejects_owner_administration() {
    during_exact(|| cache_restore::seeded(&["other.tsx".into()]));
}

#[test]
#[serial]
#[should_panic(expected = "cache_restore::classes: an exact attempt is active")]
fn class_companion_rejects_owner_administration() {
    during_exact(|| cache_restore::classes(None));
}

#[test]
#[serial]
#[should_panic(expected = "cache_restore::files: an exact attempt is active")]
fn file_companion_rejects_owner_administration() {
    during_exact(|| cache_restore::files(None));
}

#[test]
#[serial]
#[should_panic(expected = "cache_restore::import: an exact attempt is active")]
fn cache_import_rejects_owner_administration() {
    during_exact(|| {
        cache_restore::import(StyleSheet::default())
            .unwrap_or_else(|error| panic!("cache import failed: {error}"));
    });
}

#[test]
#[serial]
#[should_panic(expected = "StyleSheet::set_theme: an exact attempt is active")]
fn native_theme_setter_rejects_owner_administration() {
    during_exact(|| StyleSheet::default().set_theme(sheet::theme::Theme::default()));
}

#[test]
#[serial]
#[should_panic(expected = "style sheet: recursive lock-held entry")]
fn sheet_read_rejects_recursive_read() {
    let _poison_cleanup = exact_test_support::SheetPoisonCleanup::new();
    with_style_sheet(|_| {
        with_style_sheet(|_| {});
    });
}

#[test]
#[serial]
#[should_panic(expected = "style sheet: recursive lock-held entry")]
fn sheet_read_rejects_recursive_write() {
    let _poison_cleanup = exact_test_support::SheetPoisonCleanup::new();
    with_style_sheet(|_| {
        with_style_sheet_mut(|_| {});
    });
}

#[test]
#[serial]
#[should_panic(expected = "style sheet: recursive lock-held entry")]
fn sheet_write_rejects_recursive_read() {
    let _poison_cleanup = exact_test_support::SheetPoisonCleanup::new();
    with_style_sheet_mut(|_| {
        with_style_sheet(|_| {});
    });
}

#[test]
#[serial]
#[should_panic(expected = "style sheet: recursive lock-held entry")]
fn sheet_write_rejects_recursive_write() {
    let _poison_cleanup = exact_test_support::SheetPoisonCleanup::new();
    with_style_sheet_mut(|_| {
        with_style_sheet_mut(|_| {});
    });
}
