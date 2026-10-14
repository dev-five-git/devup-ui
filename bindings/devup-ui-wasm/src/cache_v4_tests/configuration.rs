use super::*;

#[test]
#[serial]
fn late_companion_mismatch_preserves_fresh_configuration_and_prepass() {
    // Given: restored cache followed by legitimate fresh names-only and routing inputs.
    reset_build_state_internal();
    compile();
    let value = snapshot();
    reset_build_state_internal();
    assert_eq!(import(value), Ok(()));
    set_prefix(Some("du-FLa-".into()));
    set_naming_root(Some("/checkout".into()), Some("/checkout".into()));
    set_debug(true);
    register_theme_internal(
        serde_json::from_value(serde_json::json!({"colors":{"default":{"primary":"#123456"}}}))
            .unwrap_or_else(|error| panic!("{error}")),
    );
    register_shorthands_internal(BTreeMap::from([(
        "insetX".into(),
        vec!["left".into(), "right".into()],
    )]));
    let theme_css = with_style_sheet(|sheet| sheet.theme.to_css());
    import_canonical_map_internal(HashMap::from([("child.tsx".into(), "bucket.tsx".into())]));
    import_file_routes_internal(HashMap::from([(
        "child.tsx".into(),
        std::collections::HashSet::from([1, 2]),
    )]));
    set_atom_hoist(Some(2));
    seed_file_map(vec!["child.tsx".into()]);
    // When: malformed companion data invalidates only authoritative cached state.
    cache_restore::classes(None);
    // Then: fresh prefix/root/debug/canonical/routes/hoist/D9 inputs remain in effect.
    assert_eq!(get_prefix(), Some("du-FLa-".into()));
    assert_eq!(css::naming_root::key("/checkout/src/a.tsx"), "src/a.tsx");
    assert!(is_debug());
    assert_eq!(css::file_map::canonical("child.tsx"), "bucket.tsx");
    assert_eq!(css::file_map::original_id("child.tsx"), Some(0));
    assert_eq!(
        css::file_map::get_file_map().get_by_left("bucket.tsx"),
        Some(&0)
    );
    assert!(css::atom_hoist::is_hoisted_bucket("child.tsx"));
    assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    assert_eq!(with_style_sheet(|sheet| sheet.theme.to_css()), theme_css);
    assert_eq!(
        css::get_custom_shorthand_names(),
        vec!["insetX".to_string()]
    );
    register_shorthands_internal(BTreeMap::new());
    register_theme_internal(sheet::theme::Theme::default());
    set_debug(false);
    reset_build_state_internal();
}
