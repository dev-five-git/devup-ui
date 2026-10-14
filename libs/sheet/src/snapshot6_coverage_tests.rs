use super::test_support::*;
use css::style_selector::{AtRuleKind, StyleSelector};

#[rstest::rstest]
#[case("/atomNamingVersion", json!("6"))]
#[case("/sourceIds/a", json!(false))]
#[case("/fileMap/a", json!([]))]
#[case("/evidence/config/prefix", json!(0))]
#[case("/evidence/placements/0/single_css", json!("false"))]
#[case("/evidence/authored", json!({}))]
#[case("/evidence/authored/0", json!({"UnknownFootprint":{}}))]
#[case("/evidence/authored/0", json!(false))]
#[case("/evidence/authored/0", json!({}))]
#[case("/evidence/authored/0", json!({"Css":{},"Import":{}}))]
#[serial_test::serial]
fn coverage_wrong_wire_type_rejects_at_public_parse(
    #[case] path: &str,
    #[case] replacement: Value,
) {
    // Given
    let _guard = state();
    let mut sheet = family_sheet(0);
    sheet.add_css("literal", "body{}");
    let mut value = packet(&mut sheet);
    value
        .pointer_mut(path)
        .required("existing field")
        .clone_from(&replacement);
    let before = observe(&sheet);
    // When
    let result = parse(&serde_json::to_vec(&value).required("damaged wire"));
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)), "{path}");
    assert_eq!(observe(&sheet), before);
}

#[rstest::rstest]
#[case(json!({}))]
#[case(json!(["body", "owner", "extra"]))]
#[serial_test::serial]
fn coverage_global_tuple_rejects_wrong_shape_or_extra_element(#[case] tuple: Value) {
    // Given
    let _guard = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty");
    sheet.add_property(
        "literal",
        "color",
        0,
        "red",
        Some(&StyleSelector::Global("body".into(), "owner".into())),
        None,
        Some("a"),
    );
    let mut value = packet(&mut sheet);
    value["properties"]["a"]["255"]["0"][0]["s"]["Global"] = tuple;
    // When
    let result = parse(&serde_json::to_vec(&value).required("tuple wire"));
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)));
}

#[test]
#[serial_test::serial]
fn coverage_unknown_legacy_input_rejects_in_public_baseline_packet() {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = super::advanced_support::advanced_sheet(1);
    let mut value = packet(&mut sheet);
    value["evidence"]["baseline"][0]["proof"]["allocation"]["input"] = json!({"UnknownInput":{}});
    // When
    let result = parse(&serde_json::to_vec(&value).required("unknown input"));
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)));
}

#[rstest::rstest]
#[case(StyleSelector::Selector(":hover".into()))]
#[case(StyleSelector::At { kind: AtRuleKind::Layer, query: "components".into(), selector: Some(":focus".into()), outer: vec![], file: None })]
#[serial_test::serial]
fn coverage_selector_projection_survives_public_export_admission_and_install(
    #[case] selector: StyleSelector,
) {
    // Given
    let _guard = state();
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("empty");
    sheet.add_property(
        "literal",
        "color",
        0,
        "red",
        Some(&selector),
        None,
        Some("a"),
    );
    let bytes = encoded(&mut sheet);
    let expected = super::super::records::capture(&sheet);
    fresh();
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert_eq!(super::super::records::capture(&target), expected);
    assert!(
        target.properties["a"][&255][&0]
            .iter()
            .any(|record| record.selector.as_ref() == Some(&selector))
    );
}

#[test]
#[serial_test::serial]
fn coverage_public_typed_rejection_formats_schema_cause() {
    // Given
    let _guard = state();
    let mut sheet = family_sheet(0);
    let mut value = packet(&mut sheet);
    value["atomNamingVersion"] = json!("6");
    let error = parse(&serde_json::to_vec(&value).required("wire"))
        .err()
        .required("rejection");
    // When
    let rendered = error.to_string();
    // Then
    assert_eq!(error, EvidenceError::Schema);
    assert_eq!(rendered, "strict snapshot6: Schema");
}
