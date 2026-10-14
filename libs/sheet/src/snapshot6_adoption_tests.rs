use super::test_support::*;

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[serial_test::serial]
fn complete_state_is_adopted_before_compilation_when_genuine_export_is_admitted(
    #[case] family: u8,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut original = family_sheet(family);
    original.add_css("literal", "body{}");
    original.add_import("literal", "theme.css");
    original.add_font_face(
        "literal",
        &BTreeMap::from([("font-family".into(), "literal".into())]),
    );
    original.add_property("literal", "opacity", 0, "1", None, None, Some("literal"));
    original.add_keyframes("literal", BTreeMap::new(), None);
    let bytes = encoded(&mut original);
    let expected_records = super::super::records::capture(&original);
    let expected_state = original.counter_state.clone();
    let classes = super::super::state_live::maps().classes;
    let files = super::super::state_live::maps().files;
    let originals = css::file_map::get_original_ids();
    fresh();
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "fresh".into())]));
    let mut target = StyleSheet::default();
    target.theme.breakpoints = vec![0, 999];
    let certificate = validate_snapshot(parse(&bytes).required("parse")).required("admission");
    certificate
        .compare_companions(Some(&classes), Some(&files))
        .required("companions");
    certificate
        .compare_companions(None, None)
        .required("absence");
    // When
    certificate.install_into(&mut target).required("install");
    // Then
    assert_eq!(super::super::records::capture(&target), expected_records);
    assert_eq!(target.counter_state, expected_state);
    assert_eq!(target.source_ids, originals);
    assert_eq!(super::super::state_live::maps().classes, classes);
    assert_eq!(super::super::state_live::maps().files, files);
    assert_eq!(target.theme.breakpoints, vec![0, 999]);
    assert_eq!(css::file_map::canonical("a"), "fresh");
}

#[rstest::rstest]
#[case(0, 0)]
#[case(1, 0)]
#[case(2, 0)]
#[case(2, 1)]
#[case(2, 2)]
#[serial_test::serial]
fn explicit_empty_and_reservations_adopt_when_plan_and_mode_are_compatible(
    #[case] mode: u8,
    #[case] plan: u8,
) {
    // Given
    let _guard = state();
    css::debug::set_debug(mode == 1);
    css::atom_hoist::set_atom_hoist((mode == 2).then_some(2));
    let mut original = StyleSheet::default();
    update(&mut original, &styles([])).required("explicit empty");
    if plan == 2 {
        css::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["a".into()])));
    }
    if mode == 2 {
        original.atom_plan = if plan == 0 {
            None
        } else {
            css::atom_hoist::atom_plan()
        };
        css::atom_hoist::restore_atom_plan(original.atom_plan.clone());
    }
    css::class_map::set_class_map(HashMap::from([(
        "unused".into(),
        HashMap::from([("reserved".into(), 0)]),
    )]));
    css::file_map::set_file_map(
        BTreeMap::from([("unused".into(), 0usize)])
            .into_iter()
            .collect(),
    );
    css::file_map::set_original_ids(BTreeMap::from([("unused".into(), 0)]));
    let bytes = encoded(&mut original);
    fresh();
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert!(target.counter_state.is_some());
    assert_eq!(target.atom_plan, original.atom_plan);
    assert_eq!(css::atom_hoist::atom_plan(), original.atom_plan);
    assert_eq!(target.source_ids, BTreeMap::from([("unused".into(), 0)]));
    assert_eq!(css::class_map::get_class_map()["unused"]["reserved"], 0);
}

#[test]
#[serial_test::serial]
fn owned_export_retains_originals_when_live_registries_change_after_capture() {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(0);
    sheet
        .source_ids
        .insert("unrelated-sheet-metadata".into(), 42);
    let captured = CounterSheet::new(&mut sheet)
        .export_snapshot6()
        .required("export");
    fresh();
    // When
    let parsed = parse(&serde_json::to_vec(&captured).required("serialize")).required("parse");
    // Then
    assert_eq!(
        parsed.authority.sheet.source_ids,
        BTreeMap::from([("a".into(), 0)])
    );
    assert_eq!(
        sheet.source_ids,
        BTreeMap::from([("unrelated-sheet-metadata".into(), 42)])
    );
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[serial_test::serial]
fn strict_export_rejects_when_live_reservations_are_sparse(#[case] map: u8) {
    // Given
    let _guard = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty");
    match map {
        0 => css::class_map::set_class_map(HashMap::from([(
            "unused".into(),
            HashMap::from([("reserved".into(), 4)]),
        )])),
        1 => css::file_map::set_file_map(
            BTreeMap::from([("unused".into(), 4usize)])
                .into_iter()
                .collect(),
        ),
        2 => css::file_map::set_original_ids(BTreeMap::from([("unused".into(), 4)])),
        _ => panic!("map"),
    }
    // When
    let result = CounterSheet::new(&mut sheet).export_snapshot6();
    // Then
    assert!(matches!(result, Err(EvidenceError::Map)));
}
