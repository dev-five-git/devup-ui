use super::{LinkedBatch, fixtures::*};
use crate::counter_evidence::ReplayError;
use css::{
    allocation_input::{AllocationFile, NameMode},
    counter_names::NameAddress,
};
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
#[case(8)]
#[case(9)]
#[case(10)]
#[case(11)]
#[case(12)]
fn rejects_when_frozen_linkage_dimension_is_damaged(#[case] dimension: u8) {
    // Given
    let (mut candidate, mut authority) = static_fixture(NameMode::Counter);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    match dimension {
        0 => candidate.lineage.parent = 99,
        1 => candidate.proof.allocation.context.config.prefix = "forged".into(),
        2 => candidate.proof.allocation.context.config.mode = NameMode::Debug,
        3 => candidate.proof.allocation.context.file = None,
        4 => candidate.proof.allocation.context.file = Some(AllocationFile::Original(9)),
        5 => candidate.proof.allocation.context.delivery = None,
        6 => candidate.proof.emission.seed.placement.bucket = "other".into(),
        7 => candidate.proof.emission.seed.placement.hoisted = true,
        8 => authority.classes.clear(),
        9 => authority.files.clear(),
        10 => authority.originals.clear(),
        11 => candidate.lineage.children.push(9),
        12 => candidate.proof.allocation.allocation.name = "forged".into(),
        _ => unreachable!(),
    }
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(
        result.err(),
        Some(if dimension == 11 {
            ReplayError::Expansion
        } else {
            ReplayError::Allocation
        })
    );
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
fn rejects_when_actual_counter_address_disagrees_with_frozen_map(#[case] damage: u8) {
    // Given
    let (mut candidate, authority) = static_fixture(NameMode::Counter);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let NameAddress::Counter {
        namespace,
        legacy_key,
        slot,
    } = &mut candidate.proof.allocation.allocation.address
    else {
        panic!("counter")
    };
    match damage {
        0 => *namespace = String::new(),
        1 => *legacy_key = "wrong".into(),
        2 => *slot = 0,
        3 => {
            candidate.proof.allocation.allocation.address = NameAddress::Baseline {
                mode: NameMode::Counter,
                input: candidate.proof.allocation.input.clone(),
                context: candidate.proof.allocation.context.clone(),
            }
        }
        _ => unreachable!(),
    }
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Allocation));
}
