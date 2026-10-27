use rstest::rstest;
use serial_test::serial;
use std::collections::{BTreeSet, HashMap};

use crate::{
    TestStateGuard, export_class_map_internal, export_sheet_internal,
    import_class_map_json_internal, import_sheet_internal, import_sheet_json_internal,
};

#[rstest]
#[case("{", serde_json::error::Category::Eof)]
#[case("null", serde_json::error::Category::Data)]
#[case(
    r#"{"properties":{"x":{"256":{}}}}"#,
    serde_json::error::Category::Data
)]
#[case(r#"{"atom_plan":[1]}"#, serde_json::error::Category::Data)]
#[serial]
fn owner_sheet_keeps_installed_state_when_decode_fails(
    #[case] input: &str,
    #[case] category: serde_json::error::Category,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given a nonempty sentinel sheet and matching global plan.
    let _state = TestStateGuard::new();
    let plan = Some(BTreeSet::from(["sentinel".into()]));
    let sheet = sheet::StyleSheet {
        atom_plan: plan.clone(),
        ..Default::default()
    };
    import_sheet_internal(sheet);
    let before = export_sheet_internal()?;
    // When syntax or type decoding fails.
    let error = import_sheet_json_internal(input)
        .err()
        .ok_or("invalid sheet must fail")?;
    // Then the real error survives and neither sheet nor global plan changes.
    assert_eq!(error.classify(), category);
    assert_eq!(export_sheet_internal()?, before);
    assert_eq!(css::atom_hoist::atom_plan(), plan);
    Ok(())
}

#[rstest]
#[case("{", serde_json::error::Category::Eof)]
#[case("null", serde_json::error::Category::Data)]
#[case(r#"{"x":{"valid":1,"bad":"2"}}"#, serde_json::error::Category::Data)]
#[case(r#"{"x":{"bad":-1}}"#, serde_json::error::Category::Data)]
#[serial]
fn owner_classes_keep_installed_state_when_decode_fails(
    #[case] input: &str,
    #[case] category: serde_json::error::Category,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given an allocator sentinel distinct from the partial incoming map.
    let _state = TestStateGuard::new();
    css::class_map::set_class_map(HashMap::from([(
        "sentinel".into(),
        HashMap::from([("allocated".into(), 7)]),
    )]));
    let before = export_class_map_internal()?;
    // When syntax or type decoding fails after a potentially valid prefix.
    let error = import_class_map_json_internal(input)
        .err()
        .ok_or("invalid classes must fail")?;
    // Then installation never sees the partially parsed map.
    assert_eq!(error.classify(), category);
    assert_eq!(export_class_map_internal()?, before);
    Ok(())
}
