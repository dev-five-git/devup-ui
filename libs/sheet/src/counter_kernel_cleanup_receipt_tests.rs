use super::{
    Cleanup, LinkedBatch,
    cleanup_tests::{cleanup, global_dynamic},
    fixtures::*,
    records, scratch,
};
use crate::counter_evidence::ReplayError;
use css::allocation_input::AllocationFile;
use serial_test::serial;

#[test]
#[serial]
fn repeated_cleanup_preserves_historical_receipt_when_only_reset_survives() {
    // Given
    let (candidate, mut authority) = global_dynamic(false);
    let batch = linked(&[candidate], &authority);
    let incoming = empty(&authority);
    let first = require_ok(scratch::apply_scratch(
        &sheet(&batch),
        &batch,
        request(&incoming, Some(&cleanup())),
    ));
    authority.cleanups = first.cleanups;
    authority.phase = first.phase;
    let historical = require_ok(LinkedBatch::link_captured_batch(
        &first.candidates,
        &first.evidence,
        &authority,
    ));
    let input = sheet(&historical);
    // When
    let repeated = require_ok(scratch::apply_scratch(
        &input,
        &historical,
        request(&incoming, Some(&cleanup())),
    ));
    // Then
    assert!(!repeated.cleaned);
    assert_eq!(repeated.evidence, historical.evidence);
    assert_eq!(repeated.cleanups, historical.cleanups);
}

#[test]
#[serial]
fn cleanup_rejects_wrong_frozen_target_before_scratch_or_evidence_mutation() {
    // Given
    let (candidate, authority) = global_dynamic(false);
    let batch = linked(&[candidate], &authority);
    let input = sheet(&batch);
    let before = records::capture(&input);
    let evidence_before = batch.evidence.clone();
    let wrong = Cleanup {
        bucket: "wrong".into(),
        ..cleanup()
    };
    let incoming = empty(&authority);
    // When
    let result = scratch::apply_scratch(&input, &batch, request(&incoming, Some(&wrong)));
    // Then
    assert_eq!(result.err(), Some(ReplayError::Cleanup));
    assert_eq!(records::capture(&input), before);
    assert_eq!(batch.evidence, evidence_before);
}

#[test]
#[serial]
fn receipt_linkage_uses_frozen_bucket_when_current_delivery_mapping_is_irrelevant() {
    // Given
    let (candidate, mut authority) = global_dynamic(false);
    let batch = linked(&[candidate], &authority);
    let incoming = empty(&authority);
    let output = require_ok(scratch::apply_scratch(
        &sheet(&batch),
        &batch,
        request(&incoming, Some(&cleanup())),
    ));
    authority.cleanups = output.cleanups;
    authority.phase = output.phase;
    authority
        .deliveries
        .get_mut("raw.tsx")
        .unwrap_or_else(|| panic!("delivery"))
        .canonical = "new-delivery".into();
    authority.files.insert("new-delivery".into(), 42);
    // When
    let result = LinkedBatch::link_captured_batch(&output.candidates, &output.evidence, &authority);
    // Then
    assert!(result.is_ok());
    assert_eq!(
        output.candidates[0].proof.allocation.context.file,
        None::<AllocationFile>
    );
}

#[test]
#[serial]
fn guarded_cleanup_retains_complete_proof_when_only_raw_payload_is_removed() {
    // Given
    let (candidate, mut authority) =
        super::fixtures::static_fixture(css::allocation_input::NameMode::Counter);
    authority
        .authored
        .push(crate::counter_evidence::RecordFootprint::Css {
            source: "raw.tsx".into(),
            css: concat!("body", "{", "color:red", "}").into(),
        });
    let batch = linked(&[candidate], &authority);
    let incoming = empty(&authority);
    // When
    let output = require_ok(scratch::apply_scratch(
        &sheet(&batch),
        &batch,
        request(&incoming, Some(&cleanup())),
    ));
    // Then
    assert!(output.cleaned);
    assert_eq!(output.css.len(), 0);
    assert_eq!(
        output.candidates[0].proof.emission.materialization,
        crate::counter_evidence::Materialization::Complete
    );
}
