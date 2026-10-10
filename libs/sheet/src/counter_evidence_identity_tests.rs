use std::collections::{BTreeMap, HashSet};

use css::style_selector::{AtRule, AtRuleKind, StyleSelector};
use rstest::rstest;

use crate::{
    counter_evidence::*,
    emission_seed::*,
    emission_seed_test_helpers::{declaration, property, require_ok, seed},
};

#[test]
fn keyframe_identity_retains_duplicates_and_order_when_sets_would_lose_them() {
    // Given
    let step = (
        "from".into(),
        vec![
            ("font-size".into(), "14px".into()),
            ("font-size".into(), "24px".into()),
        ],
    );
    let mut reordered = step.clone();
    reordered.1.reverse();
    let mut duplicate = step.clone();
    duplicate.1.push(("font-size".into(), "14px".into()));
    // When
    let records =
        HashSet::from(
            [step, reordered, duplicate].map(|step| RecordFootprint::Keyframes {
                bucket: "delivery".into(),
                name: "a".into(),
                steps: vec![step],
            }),
        );
    // Then
    assert_eq!(records.len(), 3);
}

#[test]
fn keyframe_identity_retains_step_order_name_and_bucket_when_members_are_equal() {
    // Given
    let original = RecordFootprint::Keyframes {
        bucket: "delivery".into(),
        name: "a".into(),
        steps: vec![("from".into(), vec![]), ("to".into(), vec![])],
    };
    let mut variants = vec![original.clone(); 3];
    let RecordFootprint::Keyframes { steps, .. } = &mut variants[0] else {
        panic!("keyframes")
    };
    steps.reverse();
    let RecordFootprint::Keyframes { name, .. } = &mut variants[1] else {
        panic!("keyframes")
    };
    *name = "b".into();
    let RecordFootprint::Keyframes { bucket, .. } = &mut variants[2] else {
        panic!("keyframes")
    };
    *bucket = "other".into();
    variants.push(original);
    // When
    let records: HashSet<_> = variants.into_iter().collect();
    // Then
    assert_eq!(records.len(), 4);
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
fn structured_selector_identity_retains_dimensions_when_one_part_changes(#[case] dimension: u8) {
    // Given
    let original = StyleSelector::At {
        kind: AtRuleKind::Media,
        query: "screen".into(),
        selector: Some("body".into()),
        outer: vec![AtRule {
            kind: AtRuleKind::Supports,
            query: "(display:grid)".into(),
        }],
        file: Some("raw.tsx".into()),
    };
    let mut changed = original.clone();
    let StyleSelector::At {
        kind,
        query,
        selector,
        outer,
        file,
    } = &mut changed
    else {
        panic!("at")
    };
    match dimension {
        0 => *kind = AtRuleKind::Container,
        1 => *query = "print".into(),
        2 => *selector = None,
        3 => outer[0].kind = AtRuleKind::Layer,
        4 => outer[0].query = "other".into(),
        5 => *file = Some("other.tsx".into()),
        _ => unreachable!(),
    }
    let make_record = |selector| {
        let mut record = property("14px");
        let RecordFootprint::Property { record: inner, .. } = &mut record else {
            panic!("property")
        };
        inner.selector = Some(selector);
        record
    };
    // When
    let records = HashSet::from([make_record(original), make_record(changed)]);
    // Then
    assert_eq!(records.len(), 2);
}

#[test]
fn raw_records_retain_payloads_when_sources_are_equal() {
    // Given
    let source = "raw.tsx".to_string();
    let records = vec![
        RecordFootprint::Css {
            source: source.clone(),
            css: concat!("body", "{", "color:red", "}").into(),
        },
        RecordFootprint::Css {
            source: source.clone(),
            css: concat!("body", "{", "color:blue", "}").into(),
        },
        RecordFootprint::Import {
            source: source.clone(),
            url: "a.css".into(),
        },
        RecordFootprint::Import {
            source: source.clone(),
            url: "b.css".into(),
        },
        RecordFootprint::FontFace {
            source: source.clone(),
            properties: BTreeMap::from([("src".into(), "url(a)".into())]),
        },
        RecordFootprint::FontFace {
            source: source.clone(),
            properties: BTreeMap::from([("src".into(), "url(b)".into())]),
        },
        RecordFootprint::GlobalCssOwner { source },
        RecordFootprint::GlobalCssOwner {
            source: "other.tsx".into(),
        },
    ];
    // When
    let records: HashSet<_> = records.into_iter().collect();
    // Then
    assert_eq!(records.len(), 8);
}

#[test]
fn global_owner_identity_is_exact_when_selector_text_is_equal() {
    // Given
    let make_record = |owner: &str| {
        let mut record = property("14px");
        let RecordFootprint::Property { record: inner, .. } = &mut record else {
            panic!("property")
        };
        inner.selector = Some(StyleSelector::Global("body".into(), owner.into()));
        record
    };
    // When
    let records = HashSet::from([make_record("raw.tsx"), make_record("other.tsx")]);
    // Then
    assert_eq!(records.len(), 2);
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
fn dynamic_metadata_damage_rejects_when_seed_is_unchanged(#[case] dimension: u8) {
    // Given
    let input = seed(EmissionInput::Dynamic {
        declaration: declaration("padding", "normalized"),
        variable: "--v".into(),
        site: Some(NumericSite {
            source: 0,
            at: 1,
            role: 0,
        }),
        important: true,
    });
    let mut expansion = require_ok(input.replay("a"));
    let Expansion::Dynamic {
        variable,
        site,
        important,
        reset,
        ..
    } = &mut expansion
    else {
        panic!("dynamic")
    };
    match dimension {
        0 => *variable = "--other".into(),
        1 => {
            *site = Some(NumericSite {
                source: 1,
                at: 1,
                role: 0,
            });
        }
        2 => *important = false,
        3 | 4 => {
            let RecordFootprint::Property { record, .. } = reset.as_mut() else {
                panic!("reset")
            };
            if dimension == 3 {
                record.layer = Some("unexpected".into());
            } else {
                record.owner_reset = false;
            }
        }
        _ => unreachable!(),
    }
    let proof = EmissionWitness {
        seed: input,
        expansion,
        name: "a".into(),
        materialization: Materialization::Complete,
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Expansion));
}

#[test]
fn keyframe_cleanup_receipt_rejects_when_cleanup_does_not_remove_keyframes() {
    // Given
    let input = seed(EmissionInput::Keyframes {
        steps: BTreeMap::from([("from".into(), vec![declaration("font-size", "14px")])]),
    });
    let proof = EmissionWitness {
        expansion: require_ok(input.replay("a")),
        seed: input,
        name: "a".into(),
        materialization: Materialization::AfterGlobalCleanup {
            source: "raw.tsx".into(),
            bucket: "delivery.tsx".into(),
        },
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Cleanup));
}
