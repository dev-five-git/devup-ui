use super::{LinkedBatch, fixtures::*};
use crate::{
    counter_evidence::{Expansion, RecordFootprint, ReplayError},
    emission_seed::{EmissionInput, Resolution},
};
use css::allocation_input::{LegacyDeclaration, LegacyInput, NameMode};
use rstest::rstest;

#[rstest]
#[case("margin", ("$gap", Some("14px 14px 14px 14px")), "14px 14px 14px 14px")]
#[case("margin", ("14px 14px", None), "14px 14px")]
#[case("content", ("\"red\"", None), "\"red\"")]
fn frozen_lookup_links_when_legacy_value_is_independently_projected(
    #[case] property: &str,
    #[case] lookup: (&str, Option<&str>),
    #[case] expected: &str,
) {
    // Given
    let mut declaration = declaration(property, lookup.0);
    declaration.resolution = Resolution::FirstValue;
    declaration.first_value = lookup.1.map(str::to_string);
    let (candidate, authority) = fixture(
        seed(EmissionInput::Static(declaration)),
        LegacyInput::Declaration(LegacyDeclaration {
            property: property.into(),
            level: 0,
            value: Some(expected.into()),
            selector: None,
            order: None,
        }),
        NameMode::Counter,
    );
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    let RecordFootprint::Property { record, .. } = &batch.records[0] else {
        panic!("property")
    };
    assert_eq!(record.value, expected);
}

pub(super) fn typography() -> (super::Candidate, super::FrozenAuthority) {
    let mut declaration = declaration("typography", "heading");
    declaration.preset = Some(preset());
    fixture(
        seed(EmissionInput::Typography(declaration)),
        LegacyInput::Declaration(LegacyDeclaration {
            property: "typography".into(),
            level: 0,
            value: Some("heading".into()),
            selector: None,
            order: None,
        }),
        NameMode::Counter,
    )
}

#[test]
fn complete_typography_links_when_frozen_sparse_frames_supply_all_members() {
    // Given
    let (candidate, authority) = typography();
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    assert_eq!(batch.records.len(), 4);
    let values: Vec<_> = batch
        .records
        .iter()
        .filter_map(|footprint| match footprint {
            RecordFootprint::Property { level, record, .. } => {
                (record.property == "font-size").then_some((*level, record.value.as_str()))
            }
            RecordFootprint::Keyframes { .. }
            | RecordFootprint::Css { .. }
            | RecordFootprint::Import { .. }
            | RecordFootprint::FontFace { .. }
            | RecordFootprint::GlobalCssOwner { .. } => None,
        })
        .collect();
    assert_eq!(values, vec![(0, "14px"), (2, "24px")]);
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
fn typography_rejects_when_expansion_is_partial_extra_or_mutated(#[case] damage: u8) {
    // Given
    let (mut candidate, authority) = typography();
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let Expansion::Typography { members, .. } = &mut candidate.proof.emission.expansion else {
        panic!("typography")
    };
    match damage {
        0 => {
            members.pop();
        }
        1 => members.push(members[0].clone()),
        2 => {
            let RecordFootprint::Property { record, .. } = &mut members[0] else {
                panic!("property")
            };
            record.value = "99px".into();
        }
        _ => unreachable!(),
    }
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
}

#[test]
fn preset_naming_stays_raw_when_frozen_typography_expands_to_effective_declarations() {
    // Given
    let (mut candidate, authority) = typography();
    let EmissionInput::Typography(declaration) = &mut candidate.proof.emission.seed.body else {
        panic!("typography")
    };
    declaration.resolution = Resolution::FirstValue;
    declaration.first_value = Some("not-a-preset".into());
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    let LegacyInput::Declaration(input) = &batch.candidates[0].proof.allocation.input else {
        panic!("declaration")
    };
    assert_eq!(input.value.as_deref(), Some("heading"));
    assert_eq!(batch.records.len(), 4);
}
