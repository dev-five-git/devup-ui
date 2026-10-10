use super::{raw::Raw, test_support::*, wire::Wire};
use css::allocation_input::AllocationFile;
use serde::{
    Deserialize,
    de::value::{BytesDeserializer, Error, StringDeserializer},
};

#[test]
fn coverage_private_raw_owned_string_uses_serde_default_delegation() {
    // Given
    let text = "owned escaped \" text 한국어".to_string();
    let deserializer = StringDeserializer::<Error>::new(text.clone());
    // When
    let decoded = Raw::deserialize(deserializer).required("private owned-string codec");
    // Then
    assert_eq!(decoded, Raw::String(text));
}

#[test]
fn coverage_private_raw_generic_bytes_rejection_formats_expected_type() {
    // Given
    let deserializer = BytesDeserializer::<Error>::new(b"not snapshot JSON");
    // When
    let error = Raw::deserialize(deserializer)
        .err()
        .required("unsupported raw type");
    // Then
    assert_eq!(
        error.to_string(),
        "invalid type: byte array, expected strict snapshot data"
    );
}

#[test]
fn coverage_private_fresh_phase_codec_roundtrips_without_minting_state() {
    // Given
    let phase = super::super::BatchPhase::Fresh;
    // When
    let decoded = super::super::BatchPhase::read(phase.write()).required("private phase codec");
    // Then
    assert_eq!(decoded, phase);
}

#[test]
fn coverage_private_legacy_file_codec_roundtrips_without_producer_authority() {
    // Given
    let file = AllocationFile::Legacy {
        filename: "a".into(),
        ordinal: 0,
    };
    // When
    let decoded = AllocationFile::read(file.write()).required("private Legacy codec");
    // Then
    assert_eq!(decoded, file);
}

#[test]
#[serial_test::serial]
fn coverage_public_admission_rejects_coherent_legacy_context_against_original_seed() {
    // Given
    let _guard = state();
    let _presets = Presets::save();
    let mut sheet = super::advanced_support::advanced_sheet(1);
    let mut value = packet(&mut sheet);
    let allocation = &mut value["evidence"]["baseline"][0]["proof"]["allocation"];
    assert_eq!(allocation["context"]["file"], json!({"Original":0}));
    let legacy = serde_json::to_value(
        AllocationFile::Legacy {
            filename: "a".into(),
            ordinal: 0,
        }
        .write(),
    )
    .required("private Legacy wire");
    allocation["context"]["file"] = legacy.clone();
    allocation["allocation"]["address"]["Baseline"]["context"]["file"] = legacy;
    let parsed = parse(&serde_json::to_vec(&value).required("coherent context"))
        .required("schema accepts Legacy representation only");
    let before = observe(&sheet);
    // When
    let result = validate_snapshot(parsed);
    // Then
    assert!(matches!(
        result,
        Err(EvidenceError::Kernel(super::super::KernelError::Authority))
    ));
    assert_eq!(observe(&sheet), before);
}
