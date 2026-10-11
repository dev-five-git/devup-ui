use std::collections::BTreeMap;

use crate::{
    counter_evidence::*,
    emission_seed::*,
    emission_seed_test_helpers::{declaration, preset, require_ok, seed},
};

#[test]
fn keyframes_preserve_ordered_duplicate_declarations_when_values_repeat() {
    // Given
    let input = seed(EmissionInput::Keyframes {
        steps: BTreeMap::from([
            ("to".into(), vec![declaration("opacity", "1")]),
            (
                "from".into(),
                vec![
                    declaration("font-size", "14px"),
                    declaration("font-size", "24px"),
                    declaration("font-size", "14px"),
                ],
            ),
        ]),
    });
    // When
    let result = input.replay("animation");
    // Then
    let expected = vec![
        (
            "from".into(),
            vec![
                ("font-size".into(), "14px".into()),
                ("font-size".into(), "24px".into()),
                ("font-size".into(), "14px".into()),
            ],
        ),
        ("to".into(), vec![("opacity".into(), "1".into())]),
    ];
    assert_eq!(
        result,
        Ok(Expansion::Keyframes {
            steps: expected.clone(),
            record: RecordFootprint::Keyframes {
                bucket: "delivery.tsx".into(),
                name: "animation".into(),
                steps: expected,
            }
        })
    );
}

#[test]
fn keyframe_typography_uses_frozen_raw_frame_when_effective_member_needs_preset() {
    // Given
    let mut member = declaration("typography", "heading|font-weight:0");
    member.preset = Some(FrozenPreset {
        name: "heading".into(),
        frames: vec![Some(FrozenTypography {
            font_size: Some("14px".into()),
            ..FrozenTypography::default()
        })],
    });
    let input = seed(EmissionInput::Keyframes {
        steps: BTreeMap::from([("from".into(), vec![member])]),
    });
    // When
    let Expansion::Keyframes { steps, .. } = require_ok(input.replay("animation")) else {
        panic!("keyframes")
    };
    // Then
    assert_eq!(
        steps,
        vec![(
            "from".into(),
            vec![(
                "typography".into(),
                "t0000000000000001000000000000000009666f6e742d73697a65000000000000000431347078"
                    .into()
            )]
        )]
    );
}

#[test]
fn keyframe_typography_rejects_when_only_first_value_was_captured() {
    // Given
    let mut member = declaration("typography", "heading");
    member.first_value = Some("14px".into());
    let input = seed(EmissionInput::Keyframes {
        steps: BTreeMap::from([("from".into(), vec![member])]),
    });
    // When
    let result = input.replay("animation");
    // Then
    assert_eq!(result, Err(ReplayError::MissingPreset));
}

#[test]
fn keyframe_reordering_rejects_when_record_and_footprint_are_both_damaged() {
    // Given
    let input = seed(EmissionInput::Keyframes {
        steps: BTreeMap::from([(
            "from".into(),
            vec![
                declaration("font-size", "14px"),
                declaration("font-size", "24px"),
            ],
        )]),
    });
    let damaged_steps = vec![(
        "from".into(),
        vec![
            ("font-size".into(), "24px".into()),
            ("font-size".into(), "14px".into()),
        ],
    )];
    let proof = EmissionWitness {
        name: "a".into(),
        seed: input,
        materialization: Materialization::Complete,
        expansion: Expansion::Keyframes {
            steps: damaged_steps.clone(),
            record: RecordFootprint::Keyframes {
                bucket: "delivery.tsx".into(),
                name: "a".into(),
                steps: damaged_steps,
            },
        },
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Expansion));
}

#[test]
fn frames_replay_all_seven_fields_when_empty_values_are_skipped() {
    // Given
    let preset = FrozenPreset {
        name: "all".into(),
        frames: vec![
            None,
            Some(FrozenTypography {
                font_family: Some(" Serif ".into()),
                font_size: Some("24px".into()),
                font_style: Some("italic".into()),
                font_weight: Some("700".into()),
                line_height: Some("1.5".into()),
                letter_spacing: Some("1px".into()),
                text_transform: Some("uppercase".into()),
            }),
            Some(FrozenTypography {
                font_size: Some(" ".into()),
                font_style: Some(String::new()),
                ..FrozenTypography::default()
            }),
        ],
    };
    // When
    let result = preset.declarations(0, &[]);
    // Then
    assert_eq!(
        result,
        vec![
            (1, "font-family".into(), "Serif".into()),
            (1, "font-size".into(), "24px".into()),
            (1, "font-style".into(), "italic".into()),
            (1, "font-weight".into(), "700".into()),
            (1, "line-height".into(), "1.5".into()),
            (1, "letter-spacing".into(), "1px".into()),
            (1, "text-transform".into(), "uppercase".into()),
        ]
    );
}

#[test]
fn empty_preset_yields_no_members_when_nothing_materializes() {
    // Given
    let mut member = declaration("typography", "empty");
    member.preset = Some(FrozenPreset {
        name: "empty".into(),
        frames: vec![None],
    });
    // When
    let result = seed(EmissionInput::Typography(member)).replay("a");
    // Then
    assert_eq!(
        result,
        Ok(Expansion::Typography {
            preset: "empty".into(),
            yielded: vec![],
            members: vec![]
        })
    );
}

#[test]
fn wrapped_frame_index_keeps_existing_projection_when_more_than_256_frames_exist() {
    // Given
    let mut frames = vec![None; 257];
    frames[0] = Some(FrozenTypography {
        font_size: Some("14px".into()),
        ..FrozenTypography::default()
    });
    frames[256] = Some(FrozenTypography {
        font_size: Some("24px".into()),
        ..FrozenTypography::default()
    });
    let preset = FrozenPreset {
        name: "wide".into(),
        frames,
    };
    // When
    let result = preset.declarations(0, &[]);
    // Then
    assert_eq!(result, vec![(0, "font-size".into(), "24px".into())]);
}

#[test]
fn typography_duplicate_damage_rejects_when_independent_frames_are_unchanged() {
    // Given
    let mut declaration = declaration("typography", "heading");
    declaration.preset = Some(preset());
    let input = seed(EmissionInput::Typography(declaration));
    let mut expansion = require_ok(input.replay("a"));
    let Expansion::Typography { members, .. } = &mut expansion else {
        panic!("typography")
    };
    members.push(members[0].clone());
    let proof = EmissionWitness {
        name: "a".into(),
        seed: input,
        expansion,
        materialization: Materialization::Complete,
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Expansion));
}
