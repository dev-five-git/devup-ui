use super::*;

#[test]
#[serial]
fn full_state_restore_preserves_all_symbol_slots_and_appends_new_identities() {
    // Given
    setup();
    reserve(&[("a.tsx", SOURCE_A)]);
    let old = compile("a.tsx", SOURCE_A);
    let serialized = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let classes = export_class_map_internal().unwrap_or_else(|error| panic!("{error}"));
    let files = export_file_map_internal().unwrap_or_else(|error| panic!("{error}"));
    let old_slots = css::class_map::get_class_map();
    let old_global = with_style_sheet(|sheet| sheet.create_css(None, false));
    let old_local = with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false));
    reset_build_state_internal();
    import_sheet_internal(
        serde_json::from_str(&serialized).unwrap_or_else(|error| panic!("{error}")),
    );
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        old_global
    );
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false)),
        old_local
    );
    set_class_map(serde_json::from_str(&classes).unwrap_or_else(|error| panic!("{error}")));
    set_file_map(serde_json::from_str(&files).unwrap_or_else(|error| panic!("{error}")));
    css::atom_hoist::set_atom_hoist(Some(2));
    // When
    let repeat = compile("a.tsx", SOURCE_A);
    let changed_source = SOURCE_A
        .replace("opacity:1", "opacity:0.5")
        .replace("color={tone}", "outlineColor={tone}");
    reserve(&[("a.tsx", &changed_source)]);
    let changed = compile("a.tsx", &changed_source);
    // Then
    assert_eq!(old.code, repeat.code);
    assert_ne!(old.code, changed.code);
    let new_slots = css::class_map::get_class_map();
    for (scope, entries) in old_slots {
        for (key, slot) in entries {
            assert_eq!(new_slots[&scope][&key], slot);
        }
        let unique: HashSet<_> = new_slots[&scope].values().collect();
        assert_eq!(unique.len(), new_slots[&scope].len());
    }
    assert_eq!(with_style_sheet(|sheet| sheet.keyframes["a.tsx"].len()), 2);
    assert_references_resolve(&changed.code, &emitted("a.tsx"));
    reset_build_state_internal();
}
