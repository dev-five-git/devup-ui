use super::{LinkedBatch, fixtures::*};
use crate::{
    counter_evidence::{Expansion, ReplayError},
    emission_seed::{DeclarationSeed, EmissionInput, Resolution},
};
use css::allocation_input::{LegacyInput, NameMode};
use rstest::rstest;
use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

fn old_input(steps: &BTreeMap<String, Vec<DeclarationSeed>>) -> String {
    let mut hasher = DefaultHasher::new();
    steps.iter().map(|(step, members)| (step,
        members.iter().map(|member| (&member.property, &member.value, member.level, &member.selector, member.style_order, &member.layer,
            match member.resolution { Resolution::CssVariable => extractor::extract_style::extract_static_style::ThemeTokenResolution::CssVariable,
                Resolution::FirstValue => extractor::extract_style::extract_static_style::ThemeTokenResolution::FirstValue })).collect())).collect::<BTreeMap<_, Vec<_>>>().hash(&mut hasher);
    hasher.finish().to_string()
}

pub(super) fn frames(mode: NameMode) -> (super::Candidate, super::FrozenAuthority) {
    let mut typography = declaration("typography", "heading");
    typography.preset = Some(preset());
    let members = vec![
        declaration("opacity", "0"),
        declaration("opacity", "0"),
        typography,
    ];
    let steps = BTreeMap::from([("from".into(), members), ("to".into(), vec![])]);
    let input = match mode {
        NameMode::Counter | NameMode::Debug => old_input(&steps),
        NameMode::AtomHoist => "66726f6d{6f706163697479:30;6f706163697479:30;7479706f677261706879:68656164696e67;}746f{}".into(),
    };
    fixture(
        seed(EmissionInput::Keyframes { steps }),
        LegacyInput::Keyframes(input),
        mode,
    )
}

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
fn frames_link_when_raw_name_projection_differs_from_effective_typography_steps(
    #[case] mode: NameMode,
) {
    // Given
    let (candidate, authority) = frames(mode);
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    let Expansion::Keyframes { steps, .. } = &batch.candidates[0].proof.emission.expansion else {
        panic!("frames")
    };
    assert_eq!(steps[0].1.len(), 3);
    assert_eq!(steps[0].1[0], steps[0].1[1]);
    assert_eq!(steps[1].1, vec![]);
    assert_ne!(steps[0].1[2].1, "heading");
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
fn frames_reject_when_child_lineage_or_ordered_expansion_is_damaged(#[case] damage: u8) {
    // Given
    let (mut candidate, authority) = frames(NameMode::Counter);
    let evidence = evidence(std::slice::from_ref(&candidate), &authority);
    let expected = match damage {
        0 => {
            candidate.lineage.children.pop();
            ReplayError::Allocation
        }
        1 => {
            candidate.lineage.children[1] = 999;
            ReplayError::Allocation
        }
        2..=5 => {
            let Expansion::Keyframes { steps, record } = &mut candidate.proof.emission.expansion
            else {
                panic!("frames")
            };
            match damage {
                2 => steps.reverse(),
                3 => {
                    steps[0].1.pop();
                }
                4 => steps[0].1[0].1 = "1".into(),
                5 => steps[1].1.push(("opacity".into(), "1".into())),
                _ => unreachable!(),
            }
            let crate::counter_evidence::RecordFootprint::Keyframes {
                steps: authored, ..
            } = record
            else {
                panic!("frames footprint")
            };
            *authored = steps.clone();
            ReplayError::Expansion
        }
        _ => unreachable!(),
    };
    // When
    let result = LinkedBatch::link_captured_batch(&[candidate], &evidence, &authority);
    // Then
    assert_eq!(result.err(), Some(expected));
}

#[test]
fn frames_ignore_lookup_metadata_in_legacy_hash_but_replay_frozen_typography() {
    // Given
    let (mut candidate, authority) = frames(NameMode::Counter);
    let EmissionInput::Keyframes { steps } = &mut candidate.proof.emission.seed.body else {
        panic!("frames")
    };
    steps.get_mut("from").unwrap_or_else(|| panic!("step"))[2].first_value =
        Some("ignored-metadata".into());
    // When
    let batch = linked(&[candidate], &authority);
    // Then
    assert_eq!(batch.records.len(), 1);
}

#[test]
fn scratch_preserves_order_duplicates_and_empty_steps_when_frames_are_linked() {
    // Given
    let (candidate, authority) = frames(NameMode::Counter);
    let batch = linked(&[candidate], &authority);
    let base = empty(&authority);
    // When
    let output = require_ok(super::scratch::apply_scratch(
        &crate::StyleSheet::default(),
        &base,
        request(&batch, None),
    ));
    // Then
    let name = &batch.candidates[0].proof.emission.name;
    assert_eq!(output.keyframes["delivery.tsx"][name]["from"].len(), 3);
    assert_eq!(output.keyframes["delivery.tsx"][name]["to"], vec![]);
    assert_eq!(
        output.keyframes["delivery.tsx"][name]["from"][0],
        output.keyframes["delivery.tsx"][name]["from"][1]
    );
    assert_eq!((output.collected, output.updated_base_style), (true, false));
}

#[test]
fn scratch_rejects_coordinated_sheet_damage_when_frozen_keyframe_seed_is_unchanged() {
    // Given
    let (candidate, authority) = frames(NameMode::Counter);
    let batch = linked(&[candidate], &authority);
    let mut input = sheet(&batch);
    let name = &batch.candidates[0].proof.emission.name;
    input
        .keyframes
        .get_mut("delivery.tsx")
        .unwrap_or_else(|| panic!("bucket"))
        .get_mut(name)
        .unwrap_or_else(|| panic!("name"))
        .get_mut("from")
        .unwrap_or_else(|| panic!("step"))[0]
        .1 = "1".into();
    let before = input.keyframes.clone();
    // When
    let incoming = empty(&authority);
    let result = super::scratch::apply_scratch(&input, &batch, request(&incoming, None));
    // Then
    assert_eq!(result.err(), Some(ReplayError::Expansion));
    assert_eq!(input.keyframes, before);
}
