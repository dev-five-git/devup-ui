use super::{LinkedBatch, dynamic_fixture::dynamic, fixtures::*};
use crate::{
    counter_evidence::{Expansion, ReplayError},
    emission_seed::EmissionInput,
};
use css::allocation_input::NameMode;
use rstest::rstest;

#[rstest]
#[case(NameMode::Counter, false)]
#[case(NameMode::Counter, true)]
#[case(NameMode::Debug, false)]
#[case(NameMode::Debug, true)]
#[case(NameMode::AtomHoist, false)]
#[case(NameMode::AtomHoist, true)]
fn links_complete_pair_when_site_is_independent_or_no_site_original_is_parent(
    #[case] mode: NameMode,
    #[case] site: bool,
) {
    // Given
    let (candidate, authority) = dynamic(mode, site);
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    assert_eq!(batch.records.len(), 2);
    let Expansion::Dynamic {
        variable,
        consumer,
        reset,
        ..
    } = &batch.candidates[0].proof.emission.expansion
    else {
        panic!("dynamic")
    };
    let crate::counter_evidence::RecordFootprint::Property { record, .. } = consumer.as_ref()
    else {
        panic!("consumer")
    };
    assert_eq!(record.value, format!("var({variable}) !important"));
    let crate::counter_evidence::RecordFootprint::Property { level, record, .. } = reset.as_ref()
    else {
        panic!("reset")
    };
    assert_eq!(
        (
            *level,
            record.value.as_str(),
            &record.selector,
            &record.layer,
            record.owner_reset
        ),
        (0, "initial", &None, &None, true)
    );
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
fn rejects_when_no_site_variable_has_unproved_independent_envelope(#[case] damage: u8) {
    // Given
    let (mut candidate, mut authority) = dynamic(NameMode::Counter, false);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let variable = candidate
        .lineage
        .variable
        .as_mut()
        .unwrap_or_else(|| panic!("variable"));
    match damage {
        0 => variable.original = 999,
        1 => {
            variable.evidence.context.file =
                Some(css::allocation_input::AllocationFile::Original(7));
        }
        2 => variable.evidence.allocation.name = "--forged".into(),
        3 => {
            let _removed_map = authority
                .classes
                .remove("")
                .unwrap_or_else(|| panic!("shared map"));
        }
        4 => variable.evidence.input = candidate.proof.allocation.input.clone(),
        5 => candidate.lineage.variable = None,
        _ => unreachable!(),
    }
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Allocation));
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
fn rejects_when_numeric_site_or_exact_variable_grammar_is_damaged(#[case] damage: u8) {
    // Given
    let (mut candidate, authority) = dynamic(NameMode::Counter, true);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let EmissionInput::Dynamic { variable, site, .. } = &mut candidate.proof.emission.seed.body
    else {
        panic!("dynamic")
    };
    let site = site.as_mut().unwrap_or_else(|| panic!("site"));
    match damage {
        0 => site.source = 999,
        1 => site.at = 3,
        2 => site.role = 0,
        3 => *variable = "---pSUHdeadbeef-c-b".into(),
        _ => unreachable!(),
    }
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Allocation));
}

#[test]
fn no_site_reuse_links_when_variable_slot_precedes_class_slot() {
    // Given
    let (candidate, authority) = dynamic(NameMode::Counter, false);
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    assert_eq!(batch.evidence.counters["D9-7"].len(), 1);
    assert_eq!(batch.records.len(), 2);
    assert_eq!(
        batch.candidates[0]
            .lineage
            .variable
            .as_ref()
            .unwrap_or_else(|| panic!("variable"))
            .evidence
            .allocation
            .name,
        "--pc"
    );
}
