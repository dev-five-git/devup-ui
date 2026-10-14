use super::test_support::*;

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[case(6)]
#[case(7)]
#[serial_test::serial]
fn raw_set_duplicates_reject_when_normalization_would_otherwise_hide_them(#[case] kind: u8) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(0);
    sheet.add_css("literal", "body{}");
    sheet.add_import("literal", "theme.css");
    sheet.add_font_face(
        "literal",
        &BTreeMap::from([("font-family".into(), "literal".into())]),
    );
    let mut value = packet(&mut sheet);
    let path = match kind {
        0 => "/properties/a/255/0",
        1 => "/css/literal",
        2 => "/imports/literal",
        3 => "/font_faces/literal",
        4 => "/global_css_files",
        5 => "/evidence/counters/D9-0/0",
        6 => "/evidence/authored",
        7 => "/evidence/counters/D9-0/0/0/proof/emission/expansion/Static",
        _ => panic!("kind"),
    };
    let items = value
        .pointer_mut(path)
        .required("set")
        .as_array_mut()
        .required("array");
    items.push(items[0].clone());
    // When
    let result = parse(&serde_json::to_vec(&value).required("damage"));
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)));
}

#[rstest::rstest]
#[case("00")]
#[case("-1")]
#[case("1.0")]
#[case("256")]
#[case("18446744073709551616")]
#[serial_test::serial]
fn coordinate_rejects_when_aliased_or_outside_unsigned_width(#[case] key: &str) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = family_sheet(0);
    let mut value = packet(&mut sheet);
    let members = value["properties"]["a"]["255"]["0"].clone();
    value["properties"]["a"]["255"]
        .as_object_mut()
        .required("levels")
        .insert(key.into(), members);
    // When
    let result = parse(&serde_json::to_vec(&value).required("damage"));
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)));
}

#[test]
#[serial_test::serial]
fn full_eq_distinct_properties_survive_when_lossy_ord_would_merge_them() {
    // Given
    let _guard = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty");
    sheet.add_property("first", "color", 0, "red", None, None, None);
    sheet.add_property("second", "color", 0, "red", None, None, None);
    let bytes = encoded(&mut sheet);
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert_eq!(target.properties[""][&255][&0].len(), 2);
}

#[test]
#[serial_test::serial]
fn distinct_lineage_witnesses_survive_when_same_proof_shares_order_zero_slot() {
    // Given
    let _guard = state();
    let mut sheet = StyleSheet::default();
    for source in ["a", "b"] {
        let items = fixture(source, || {
            let mut declaration = ExtractStaticStyle::new("color", "red", 0, None);
            declaration.style_order = Some(0);
            styles([ExtractStyleValue::Static(declaration)])
        });
        update(&mut sheet, &items).required("update");
    }
    let bytes = encoded(&mut sheet);
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    let candidates: Vec<_> = target
        .counter_state
        .as_ref()
        .required("state")
        .candidates()
        .collect();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].proof, candidates[1].proof);
    assert_ne!(candidates[0].lineage, candidates[1].lineage);
}
