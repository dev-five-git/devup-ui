use rstest::rstest;
use serial_test::serial;

use crate::owner_codec_remaining_tests::{capture_owner, install_owner};
use crate::*;

#[rstest]
#[case(import_file_map_json_internal, "{", serde_json::error::Category::Eof)]
#[case(
    import_file_map_json_internal,
    "null",
    serde_json::error::Category::Data
)]
#[case(
    import_file_map_json_internal,
    r#"{"valid":1,"bad":"2"}"#,
    serde_json::error::Category::Data
)]
#[case(
    import_file_map_json_internal,
    r#"{"bad":-1}"#,
    serde_json::error::Category::Data
)]
#[case(
    import_canonical_map_json_internal,
    "{",
    serde_json::error::Category::Eof
)]
#[case(
    import_canonical_map_json_internal,
    "null",
    serde_json::error::Category::Data
)]
#[case(
    import_canonical_map_json_internal,
    r#"{"valid":"target","bad":2}"#,
    serde_json::error::Category::Data
)]
#[case(
    import_reservation_state_json_internal,
    "{",
    serde_json::error::Category::Eof
)]
#[case(
    import_reservation_state_json_internal,
    "null",
    serde_json::error::Category::Data
)]
#[case(
    import_reservation_state_json_internal,
    r#"{"phase":"Other","misses":1,"suppressed":2}"#,
    serde_json::error::Category::Data
)]
#[case(
    import_reservation_state_json_internal,
    r#"{"phase":"Live","misses":1,"suppressed":"2"}"#,
    serde_json::error::Category::Data
)]
#[case(
    import_reservation_state_json_internal,
    r#"{"phase":"Live","misses":-1,"suppressed":2}"#,
    serde_json::error::Category::Data
)]
#[serial]
fn owner_remaining_keeps_all_fields_when_individual_decode_fails(
    #[case] import: fn(&str) -> Result<(), serde_json::Error>,
    #[case] input: &str,
    #[case] category: serde_json::error::Category,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given nonempty sentinels in every owner field and a matching global plan.
    let _state = TestStateGuard::new();
    let before = install_owner("sentinel")?;
    let plan = css::atom_hoist::atom_plan();
    // When one codec rejects syntax or type, including a valid parsed prefix.
    let error = import(input).err().ok_or("invalid owner field must fail")?;
    // Then no existing owner field or global plan was touched before parse success.
    assert_eq!(error.classify(), category);
    assert_eq!(capture_owner()?, before);
    assert_eq!(css::atom_hoist::atom_plan(), plan);
    Ok(())
}
