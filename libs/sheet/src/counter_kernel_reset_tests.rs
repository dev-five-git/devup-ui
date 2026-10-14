use super::{LinkedBatch, dynamic_fixture::dynamic, fixtures::*};
use crate::counter_evidence::{Expansion, RecordFootprint, ReplayError};
use css::{allocation_input::NameMode, style_selector::StyleSelector};
use rstest::rstest;

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[case(6)]
#[case(7)]
fn reset_rejects_when_plain_level_zero_initial_identity_is_forged(#[case] damage: u8) {
    // Given
    let (mut candidate, authority) = dynamic(NameMode::Counter, true);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let Expansion::Dynamic { reset, .. } = &mut candidate.proof.emission.expansion else {
        panic!("dynamic")
    };
    let RecordFootprint::Property { record, level, .. } = reset.as_mut() else {
        panic!("reset")
    };
    match damage {
        0 => *level = 1,
        1 => record.selector = Some(StyleSelector::Selector(":hover".into())),
        2 => record.layer = Some("layer".into()),
        3 => record.property = "--extra-slot".into(),
        4 => record.class_name = "extra".into(),
        5 => record.value = "inherit".into(),
        6 => record.typography = true,
        7 => record.owner_reset = false,
        _ => unreachable!(),
    }
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
}

#[test]
fn importance_rejects_when_consumer_omits_authored_important_suffix() {
    // Given
    let (mut candidate, authority) = dynamic(NameMode::Counter, false);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let Expansion::Dynamic {
        consumer, variable, ..
    } = &mut candidate.proof.emission.expansion
    else {
        panic!("dynamic")
    };
    let RecordFootprint::Property { record, .. } = consumer.as_mut() else {
        panic!("consumer")
    };
    record.value = format!("var({variable})");
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
}

#[test]
fn kind_rejects_when_static_body_is_mislabeled_as_typography() {
    // Given
    let (mut candidate, authority) = static_fixture(NameMode::Counter);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let crate::emission_seed::EmissionInput::Static(declaration) =
        &candidate.proof.emission.seed.body
    else {
        panic!("static")
    };
    candidate.proof.emission.seed.body =
        crate::emission_seed::EmissionInput::Typography(declaration.clone());
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
}

#[test]
fn sidecar_rejects_when_candidate_has_unproved_extra_allocation_witness() {
    // Given
    let (candidate, authority) = static_fixture(NameMode::Counter);
    let mut evidence = evidence(std::slice::from_ref(&candidate), &authority);
    evidence
        .counters
        .entry("unknown".into())
        .or_default()
        .insert(0, vec![candidate.proof.clone()]);
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
}
