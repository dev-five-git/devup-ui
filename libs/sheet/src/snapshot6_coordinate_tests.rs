use super::advanced_support::*;

#[rstest::rstest]
#[case(false, false)]
#[case(false, true)]
#[case(true, false)]
#[case(true, true)]
#[serial_test::serial]
fn checked_coordinate_width_accepts_maximum_and_rejects_successor_at_parse(
    #[case] platform: bool,
    #[case] overflow: bool,
) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = advanced_sheet(0);
    let mut value = packet(&mut sheet);
    let maximum = if platform {
        u128::try_from(usize::MAX).required("usize width")
    } else {
        u128::from(u32::MAX)
    };
    let number = maximum + u128::from(overflow);
    let coordinate = if platform {
        paths(&value)
            .into_iter()
            .find(|path| {
                value
                    .pointer(path)
                    .is_some_and(|node| node.get("at").is_some() && node.get("role").is_some())
            })
            .map(|path| format!("{path}/at"))
            .required("site")
    } else {
        "/sourceIds/a".into()
    };
    value
        .pointer_mut(&coordinate)
        .required("coordinate")
        .clone_from(&json!("coordinate-placeholder"));
    let bytes = serde_json::to_string(&value)
        .required("boundary packet")
        .replace("\"coordinate-placeholder\"", &number.to_string());
    // When
    let result = parse(bytes.as_bytes());
    // Then
    assert_eq!(
        result.is_ok(),
        !overflow,
        "checked coordinate {coordinate}={number}"
    );
}

#[rstest::rstest]
#[case("-1")]
#[case("1.0")]
#[case("1e0")]
#[serial_test::serial]
fn checked_coordinate_rejects_non_unsigned_integer_syntax_before_admission(#[case] number: &str) {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = advanced_sheet(0);
    let mut value = packet(&mut sheet);
    value["sourceIds"]["a"] = json!("coordinate-placeholder");
    let bytes = serde_json::to_string(&value)
        .required("boundary packet")
        .replace("\"coordinate-placeholder\"", number);
    // When
    let result = parse(bytes.as_bytes());
    // Then
    assert!(matches!(result, Err(EvidenceError::Schema)));
}
