use super::test_support::*;
use css::style_selector::StyleSelector;

#[test]
#[serial_test::serial]
fn historical_buckets_and_owner_subset_adopt_when_resolver_and_cleanup_target_change() {
    // Given
    let _guard = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color",
            0,
            "tone",
            Some(StyleSelector::Global("body".into(), "a".into())),
        ))])
    });
    let mut sheet = StyleSheet::default();
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "old".into())]));
    update(&mut sheet, &items).required("old");
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "new".into())]));
    update(&mut sheet, &items).required("new");
    assert!(sheet.rm_global_css("a", false));
    let bytes = encoded(&mut sheet);
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "fresh".into())]));
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert_eq!(target.properties["old"][&255][&0].len(), 2);
    assert_eq!(target.properties["new"][&255][&0].len(), 1);
    assert_eq!(target.global_css_files.len(), 0);
    assert_eq!(
        target
            .counter_state
            .as_ref()
            .required("state")
            .deliveries
            .len(),
        2
    );
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn retired_keyframe_payload_adopts_when_literal_replacement_is_restored_or_retained(
    #[case] restored: bool,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(4);
    let name = sheet.keyframes["a"].keys().next().required("name").clone();
    let original = sheet.keyframes["a"][&name].clone();
    let reservations = css::class_map::get_class_map();
    assert!(sheet.add_keyframes(
        &name,
        BTreeMap::from([("from".into(), vec![("opacity".into(), "1".into())])]),
        Some("a")
    ));
    if restored {
        assert!(sheet.add_keyframes(&name, original.clone(), Some("a")));
    }
    let bytes = encoded(&mut sheet);
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert_eq!(
        target
            .counter_state
            .as_ref()
            .required("state")
            .candidates()
            .count(),
        0
    );
    assert_eq!(css::class_map::get_class_map(), reservations);
    assert_eq!(
        target.keyframes["a"][&name],
        if restored {
            original
        } else {
            BTreeMap::from([("from".into(), vec![("opacity".into(), "1".into())])])
        }
    );
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[serial_test::serial]
fn lineage_rejects_when_numeric_site_or_no_site_receipt_branch_is_wrong(#[case] damage: u8) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(if damage < 2 { 1 } else { 2 });
    let mut value = packet(&mut sheet);
    match damage {
        0 => {
            witness(&mut value)["proof"]["emission"]["seed"]["body"]["Dynamic"]["site"]["role"] =
                json!(7);
        }
        1 => {
            witness(&mut value)["proof"]["emission"]["seed"]["body"]["Dynamic"]["site"]["source"] =
                json!(55);
        }
        2 => witness(&mut value)["lineage"]["variable"] = Value::Null,
        3 => {
            witness(&mut value)["proof"]["emission"]["seed"]["body"]["Dynamic"]["site"] =
                json!({"source":0,"at":2,"role":1});
        }
        _ => panic!("damage"),
    }
    let site = witness(&mut value)["proof"]["emission"]["seed"]["body"]["Dynamic"]["site"].clone();
    witness(&mut value)["proof"]["emission"]["expansion"]["Dynamic"]["site"] = site;
    // When
    let result = admit(value);
    // Then
    assert!(matches!(
        result,
        Err(EvidenceError::Kernel(super::super::KernelError::Authority))
    ));
}
