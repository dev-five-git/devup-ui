use super::test_support::*;

fn distinct_live_maps() {
    css::class_map::set_class_map(HashMap::from([(
        "live-sentinel".into(),
        HashMap::from([("reservation".into(), 37)]),
    )]));
    css::file_map::set_file_map(
        BTreeMap::from([("live-sentinel".into(), 29usize)])
            .into_iter()
            .collect(),
    );
    css::file_map::set_original_ids(BTreeMap::from([("live-sentinel".into(), 41)]));
}

fn sentinel_target() -> StyleSheet {
    let mut target = StyleSheet::default();
    target.source_ids.insert("target".into(), 77);
    target.add_css("target", concat!("body", "{", "color:blue", "}"));
    target.add_import("target", "sentinel.css");
    target.add_font_face(
        "target",
        &BTreeMap::from([("font-family".into(), "sentinel".into())]),
    );
    target.add_keyframes("sentinel", BTreeMap::new(), None);
    target.add_property("sentinel", "opacity", 0, "1", None, None, None);
    target
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[serial_test::serial]
fn install_rejects_without_writes_when_actual_build_changes_after_validation(#[case] change: u8) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut original = family_sheet(0);
    let certificate =
        validate_snapshot(parse(&encoded(&mut original)).required("parse")).required("admit");
    let mut target = sentinel_target();
    distinct_live_maps();
    match change {
        0 => css::set_prefix(Some("changed".into())),
        1 => css::debug::set_debug(true),
        2 => css::atom_hoist::set_atom_hoist(Some(2)),
        3 => css::atom_hoist::restore_atom_plan(Some(BTreeSet::new())),
        _ => panic!("change"),
    }
    let before = observe(&target);
    // When
    let result = certificate.install_into(&mut target);
    // Then
    assert_eq!(result, Err(EvidenceError::Configuration));
    assert_eq!(observe(&target), before);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn install_typed_rejection_precedes_every_write_when_exact_scope_is_active(#[case] nested: bool) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut original = family_sheet(0);
    let certificate =
        validate_snapshot(parse(&encoded(&mut original)).required("parse")).required("admit");
    let mut target = sentinel_target();
    distinct_live_maps();
    let before = observe(&target);
    // When
    let result = css::exact_attempt::with_exclusive_attempt(|| {
        let refuse = || {
            let result = certificate.install_into(&mut target);
            assert_eq!(
                observe(&target),
                before,
                "immediate deepest-scope no-write observer"
            );
            result
        };
        if nested {
            css::exact_attempt::with_exclusive_attempt(refuse)
        } else {
            refuse()
        }
    });
    // Then
    assert_eq!(
        result,
        Err(EvidenceError::ActiveExact(
            css::admission::ActiveExactAttempt
        ))
    );
    assert_eq!(observe(&target), before);
}

#[test]
#[serial_test::serial]
fn unequal_companion_rejects_when_snapshot_registry_is_not_the_supplied_map() {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut original = family_sheet(0);
    let certificate =
        validate_snapshot(parse(&encoded(&mut original)).required("parse")).required("admit");
    // When
    let classes = certificate.compare_companions(Some(&BTreeMap::new()), None);
    let files = certificate.compare_companions(None, Some(&BTreeMap::new()));
    // Then
    assert_eq!(classes, Err(EvidenceError::Companion));
    assert_eq!(files, Err(EvidenceError::Companion));
}

#[test]
#[serial_test::serial]
fn export_rejects_latched_state_when_visible_records_are_repaired() {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(0);
    let records = sheet.properties.clone();
    sheet.properties.clear();
    sheet.add_css("literal", "body{}");
    sheet.properties = records;
    sheet.css.clear();
    sheet.global_css_files.clear();
    // When
    let result = CounterSheet::new(&mut sheet).export_snapshot6();
    // Then
    assert!(matches!(
        result,
        Err(EvidenceError::Kernel(super::super::KernelError::Coverage))
    ));
}

#[test]
fn manual_sheet_has_no_export_authority_when_counter_attempt_never_committed() {
    // Given
    let mut sheet = StyleSheet::default();
    // When
    let result = CounterSheet::new(&mut sheet).export_snapshot6();
    // Then
    assert!(matches!(result, Err(EvidenceError::State)));
}

#[test]
#[serial_test::serial]
fn saved_certificate_cannot_clear_rejection_when_same_target_was_latched_by_public_literal() {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut target = family_sheet(0);
    let certificate =
        validate_snapshot(parse(&encoded(&mut target)).required("parse")).required("admit");
    let properties = target.properties.clone();
    target.properties.clear();
    target.add_css("literal", "body{}");
    target.properties = properties;
    target.css.clear();
    target.global_css_files.clear();
    target.source_ids.insert("target-metadata".into(), 77);
    assert_eq!(
        target
            .counter_state
            .as_ref()
            .required("state")
            .rejection
            .required("public literal latch")
            .0,
        super::super::KernelError::Coverage
    );
    let before = observe(&target);
    // When
    let result = certificate.install_into(&mut target);
    // Then
    assert_eq!(
        result,
        Err(EvidenceError::Kernel(super::super::KernelError::Coverage))
    );
    assert_eq!(observe(&target), before);
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn saved_certificate_adopts_when_target_is_manual_or_successfully_empty(
    #[case] counter_empty: bool,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut original = family_sheet(0);
    let bytes = encoded(&mut original);
    let mut target = StyleSheet::default();
    if counter_empty {
        update(&mut target, &styles([])).required("empty Counter target");
    } else {
        target = sentinel_target();
    }
    let certificate = validate_snapshot(parse(&bytes).required("parse")).required("admit");
    // When
    certificate.install_into(&mut target).required("install");
    // Then
    assert!(
        target
            .counter_state
            .as_ref()
            .required("state")
            .rejection
            .is_none()
    );
    assert!(
        target.properties["a"][&255][&0]
            .iter()
            .any(|record| record.property == "color" && record.value == "red")
    );
}
