use super::{
    BatchPhase,
    cleanup_tests::{cleanup, global_dynamic},
    fixtures::*,
    phase_fixtures::*,
    records, scratch,
};
use crate::{
    StyleSheet,
    counter_evidence::{Materialization, ReplayError},
};
use css::allocation_input::NameMode;
use serial_test::serial;

#[test]
fn exact_base_coverage_rejects_missing_emission_even_when_input_is_a_subset() {
    // Given
    let (candidate, authority) = static_fixture(NameMode::Counter);
    let base = linked(&[candidate], &authority);
    let incoming = empty(&authority);
    let input = StyleSheet::default();
    let evidence_before = base.evidence.clone();
    // When
    let result = scratch::apply_scratch(&input, &base, request(&incoming, None));
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
    assert_eq!(records::capture(&input), vec![]);
    assert_eq!(base.evidence, evidence_before);
}

#[test]
fn separate_linked_configurations_reject_before_base_or_incoming_mutation() {
    // Given
    let (candidate, authority) = static_fixture(NameMode::Counter);
    let base = linked(&[candidate], &authority);
    let (candidate, authority) = static_fixture(NameMode::Debug);
    let incoming = linked(&[candidate], &authority);
    let input = sheet(&base);
    let before = records::capture(&input);
    // When
    let result = scratch::apply_scratch(&input, &base, request(&incoming, None));
    // Then
    assert_eq!(result.err(), Some(ReplayError::Allocation));
    assert_eq!(records::capture(&input), before);
    assert_eq!(
        incoming.candidates[0].proof.emission.materialization,
        Materialization::Complete
    );
}

#[test]
#[serial]
fn fresh_duplicate_preserves_registration_when_historical_identical_proof_survives_elsewhere() {
    // Given
    let (candidate, _) = global_dynamic(false);
    let mut seed = candidate.proof.emission.seed;
    seed.placement.single_css = false;
    seed.placement.bucket = "other".into();
    let (candidate, authority) = fixture(seed, candidate.proof.allocation.input, NameMode::Counter);
    let base = linked(std::slice::from_ref(&candidate), &authority);
    let incoming = linked(&[candidate], &authority);
    let output = require_ok(scratch::apply_scratch(
        &sheet(&base),
        &base,
        request(&incoming, Some(&cleanup())),
    ));
    // When
    let retained = relink(&output, &authority);
    // Then
    assert!(output.cleaned);
    assert_eq!(retained.evidence.counters["D9-7"][&4].len(), 1);
    assert_eq!(retained.candidates.len(), 2);
    assert_eq!(retained.candidates[0], retained.candidates[1]);
    assert_eq!(
        retained.candidates[0].proof.emission.materialization,
        Materialization::Complete
    );
    assert_eq!(
        retained.phase,
        BatchPhase::Retained(["raw.tsx".into()].into())
    );
    assert!(sheet(&retained).global_css_files.contains("raw.tsx"));
}

#[test]
fn retained_batch_rejects_as_incoming_even_when_all_its_proofs_are_complete() {
    // Given
    let (candidate, authority) = static_fixture(NameMode::Counter);
    let incoming = linked(&[candidate], &authority);
    let base = empty(&authority);
    let output = require_ok(scratch::apply_scratch(
        &StyleSheet::default(),
        &base,
        request(&incoming, None),
    ));
    let retained = relink(&output, &authority);
    // When
    let result = scratch::ScratchRequest::new(&retained, None);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Cleanup));
}
