use super::{LinkedBatch, fixtures::*};
use crate::counter_evidence::{Expansion, Materialization, ReplayError};
use css::allocation_input::NameMode;
use rstest::rstest;

#[rstest]
#[case(NameMode::Counter, "ph-e")]
#[case(NameMode::Debug, "pfont-size-0-14px--255-h")]
#[case(NameMode::AtomHoist, "")]
fn links_when_explicit_original_map_and_captured_mode_prove_name(
    #[case] mode: NameMode,
    #[case] counter_name: &str,
) {
    // Given
    let (candidate, authority) = static_fixture(mode);
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    assert_eq!(batch.records.len(), 1);
    match mode {
        NameMode::Counter | NameMode::Debug => {
            assert_eq!(batch.candidates[0].proof.emission.name, counter_name);
        }
        NameMode::AtomHoist => assert!(
            batch.candidates[0]
                .proof
                .emission
                .name
                .starts_with("pa1-l-")
        ),
    }
    assert_eq!(
        batch.evidence.baseline.len(),
        usize::from(mode != NameMode::Counter)
    );
}

#[rstest]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
fn baseline_rejects_when_shared_original_is_not_a_member(#[case] mode: NameMode) {
    // Given
    let (mut candidate, authority) = static_fixture(mode);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    candidate.lineage.parent = 123;
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Allocation));
}

#[test]
fn rejects_coordinated_record_and_footprint_damage_when_seed_is_unchanged() {
    // Given
    let (mut candidate, authority) = static_fixture(NameMode::Counter);
    let mut evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let Expansion::Static(records) = &mut candidate.proof.emission.expansion else {
        panic!("static")
    };
    let crate::counter_evidence::RecordFootprint::Property { record, .. } = &mut records[0] else {
        panic!("property")
    };
    record.value = "24px".into();
    evidence
        .counters
        .get_mut("D9-7")
        .unwrap_or_else(|| panic!("namespace"))
        .get_mut(&4)
        .unwrap_or_else(|| panic!("slot"))[0] = candidate.proof.clone();
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
}

#[test]
fn rejects_receipt_when_no_frozen_successful_cleanup_authority_exists() {
    // Given
    let (mut candidate, authority) = static_fixture(NameMode::Counter);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    candidate.proof.emission.materialization = Materialization::AfterGlobalCleanup {
        source: "raw.tsx".into(),
        bucket: "delivery.tsx".into(),
    };
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Cleanup));
}

#[test]
fn retains_multiple_witnesses_when_legacy_order_default_shares_one_slot() {
    // Given
    let (first, authority) = static_fixture(NameMode::Counter);
    let mut second = first.clone();
    let crate::emission_seed::EmissionInput::Static(declaration) =
        &mut second.proof.emission.seed.body
    else {
        panic!("static")
    };
    declaration.style_order = Some(255);
    let css::allocation_input::LegacyInput::Declaration(input) = &mut second.proof.allocation.input
    else {
        panic!("declaration")
    };
    input.order = Some(255);
    // When
    let batch = linked(&[first, second], &authority);
    // Then
    assert_eq!(batch.evidence.counters["D9-7"][&4].len(), 2);
    assert_eq!(batch.records.len(), 1);
}
