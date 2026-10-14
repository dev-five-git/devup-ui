use super::{dynamic_fixture::dynamic, fixtures::*};
use crate::emission_seed::{EmissionInput, NumericSite};
use css::allocation_input::{LegacyInput, NameMode};
use rstest::rstest;

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
fn no_site_plain_class_links_when_legacy_value_is_none_not_variable_text(#[case] mode: NameMode) {
    // Given
    let (candidate, _) = dynamic(mode, false);
    let variable = candidate.lineage.variable.clone();
    let mut seed = candidate.proof.emission.seed;
    let EmissionInput::Dynamic { important, .. } = &mut seed.body else {
        panic!("dynamic")
    };
    *important = false;
    let mut input = candidate.proof.allocation.input;
    let LegacyInput::Declaration(declaration) = &mut input else {
        panic!("declaration")
    };
    declaration.value = None;
    let (mut candidate, mut authority) = fixture(seed, input, mode);
    candidate.lineage.variable = variable;
    let envelope = &candidate
        .lineage
        .variable
        .as_ref()
        .unwrap_or_else(|| panic!("variable"))
        .evidence;
    retain_map(&mut authority, envelope);
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    assert_eq!(batch.records.len(), 2);
    let LegacyInput::Declaration(declaration) = &batch.candidates[0].proof.allocation.input else {
        panic!("declaration")
    };
    assert_eq!(declaration.value, None);
}

#[test]
fn numeric_site_omits_zero_role_when_captured_prefix_is_explicit() {
    // Given
    let site = NumericSite {
        source: 9,
        at: 2,
        role: 0,
    };
    // When
    let name = super::link::site_name(&site, "captured");
    // Then
    assert_eq!(name, "---capturedSj-c");
}

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
fn no_site_original_rejects_when_only_lineage_changes_to_another_registered_original(
    #[case] mode: NameMode,
) {
    // Given
    let (mut candidate, authority) = dynamic(mode, false);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    candidate
        .lineage
        .variable
        .as_mut()
        .unwrap_or_else(|| panic!("variable"))
        .original = 9;
    // When
    let result = super::LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(
        result.err(),
        Some(crate::counter_evidence::ReplayError::Allocation)
    );
}
