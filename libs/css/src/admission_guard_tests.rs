use std::collections::{BTreeMap, HashMap};

use serial_test::serial;

use crate::{admission::with_admission, exact_attempt::with_exclusive_attempt};

fn during_exact(mutate: impl FnOnce()) {
    let result: Result<(), ()> = with_exclusive_attempt(|| {
        mutate();
        Ok(())
    });
    assert_eq!(result, Ok(()));
}

#[test]
#[serial]
#[should_panic(expected = "set_prefix: an exact attempt is active")]
fn rejects_prefix_when_exact_is_active() {
    // Given / When / Then: mutation must panic before changing configuration.
    during_exact(|| crate::set_prefix(Some("other".into())));
}

#[test]
#[serial]
#[should_panic(expected = "set_debug: an exact attempt is active")]
fn rejects_debug_when_exact_is_active() {
    during_exact(|| crate::debug::set_debug(true));
}

#[test]
#[serial]
#[should_panic(expected = "set_custom_shorthands: an exact attempt is active")]
fn rejects_shorthands_when_exact_is_active() {
    during_exact(|| crate::set_custom_shorthands(BTreeMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "set_atom_hoist: an exact attempt is active")]
fn rejects_threshold_when_exact_is_active() {
    during_exact(|| crate::atom_hoist::set_atom_hoist(Some(2)));
}

#[test]
#[serial]
#[should_panic(expected = "set_file_routes: an exact attempt is active")]
fn rejects_routes_when_exact_is_active() {
    during_exact(|| crate::file_routes::set_file_routes(HashMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "reset_file_routes: an exact attempt is active")]
fn rejects_route_reset_when_exact_is_active() {
    during_exact(crate::file_routes::reset_file_routes);
}

#[test]
#[serial]
#[should_panic(expected = "set_canonical_map: an exact attempt is active")]
fn rejects_canonical_when_exact_is_active() {
    during_exact(|| crate::file_map::set_canonical_map(HashMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "reset_canonical_map: an exact attempt is active")]
fn rejects_canonical_reset_when_exact_is_active() {
    during_exact(crate::file_map::reset_canonical_map);
}

#[test]
#[serial]
#[should_panic(expected = "set_collapsed_buckets: an exact attempt is active")]
fn rejects_collapsed_when_exact_is_active() {
    during_exact(|| crate::naming::set_collapsed_buckets(&HashMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "set_root: an exact attempt is active")]
fn rejects_root_when_exact_is_active() {
    during_exact(|| crate::naming_root::set_root(None));
}

#[test]
#[serial]
#[should_panic(expected = "set_context: an exact attempt is active")]
fn rejects_context_when_exact_is_active() {
    during_exact(|| crate::naming_root::set_context(None, None));
}

#[test]
#[serial]
#[should_panic(expected = "set_theme_token_levels: an exact attempt is active")]
fn rejects_token_levels_when_exact_is_active() {
    during_exact(|| crate::theme_tokens::set_theme_token_levels(BTreeMap::new(), BTreeMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "set_theme_token_values: an exact attempt is active")]
fn rejects_token_values_when_exact_is_active() {
    during_exact(|| crate::theme_tokens::set_theme_token_values(BTreeMap::new(), BTreeMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "set_typography_keys: an exact attempt is active")]
fn rejects_typography_keys_when_exact_is_active() {
    during_exact(|| crate::theme_tokens::set_typography_keys(vec![]));
}

#[test]
#[serial]
#[should_panic(expected = "content_typography::set: an exact attempt is active")]
fn rejects_presets_when_exact_is_active() {
    during_exact(|| crate::content_typography::set(BTreeMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "set_file_map: an exact attempt is active")]
fn rejects_file_replacement_when_exact_is_active() {
    // Given / When / Then: the exact scope rejects replacement before mutation.
    during_exact(|| crate::file_map::set_file_map(bimap::BiHashMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "reset_file_map: an exact attempt is active")]
fn rejects_file_reset_when_exact_is_active() {
    // Given / When / Then: neither delivery nor original state may be cleared.
    during_exact(crate::file_map::reset_file_map);
}

#[test]
#[serial]
#[should_panic(expected = "seed_file_numbers: an exact attempt is active")]
fn rejects_file_seed_when_exact_is_active() {
    // Given / When / Then: the prepass rejects even an empty batch during exact.
    during_exact(|| crate::file_map::seed_file_numbers(&[]));
}

#[test]
#[serial]
#[should_panic(expected = "set_original_ids: an exact attempt is active")]
fn rejects_original_replacement_when_exact_is_active() {
    // Given / When / Then: original authority replacement is administrative.
    during_exact(|| crate::file_map::set_original_ids(BTreeMap::new()));
}

#[test]
#[serial]
#[should_panic(expected = "seed_original_ids: an exact attempt is active")]
fn rejects_original_seed_when_exact_is_active() {
    // Given / When / Then: direct crate-level prepass seeding cannot bypass the guard.
    during_exact(|| crate::sparse_site::source_ids::seed_original_ids(&[]));
}

#[test]
#[serial]
#[should_panic(expected = "restore_atom_plan: an exact attempt is active")]
fn rejects_plan_restore_when_exact_is_active() {
    // Given / When / Then: plan replacement or clearing must await scope exit.
    during_exact(|| crate::atom_hoist::restore_atom_plan(None));
}

#[test]
#[serial]
fn keeps_outer_administration_guard_when_nested_exact_finishes() {
    // Given / When
    let result: Result<(), ()> = with_exclusive_attempt(|| {
        with_exclusive_attempt::<(), ()>(|| Ok(()))?;
        let blocked = std::panic::catch_unwind(|| crate::set_prefix(None));
        assert!(blocked.is_err());
        Ok(())
    });
    // Then
    assert_eq!(result, Ok(()));
    with_admission(|| crate::admission::assert_administration_allowed("after success"));
}
