use std::collections::{BTreeMap, HashSet};

use css::style_selector::{AtRule, AtRuleKind, StyleSelector};
use rstest::rstest;

use crate::{
    counter_evidence::*,
    emission_seed::*,
    emission_seed_test_helpers::{
        declaration, global_dynamic, preset, property, require_ok, require_some, seed, witness,
    },
};

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
fn footprint_retains_full_identity_when_one_dimension_changes(#[case] dimension: u8) {
    // Given
    let original = property("14px");
    let mut changed = original.clone();
    let RecordFootprint::Property {
        bucket,
        order,
        level,
        record,
    } = &mut changed
    else {
        panic!("property")
    };
    match dimension {
        0 => *bucket = "other".into(),
        1 => *order = 0,
        2 => *level = 2,
        3 => record.class_name = "b".into(),
        4 => record.property = "width".into(),
        5 => record.value = "24px".into(),
        6 => record.selector = Some(StyleSelector::Global("body".into(), "owner".into())),
        7 => record.layer = Some("layer".into()),
        8 => record.typography = true,
        9 => record.hoisted = true,
        10 => record.owner_reset = true,
        _ => unreachable!(),
    }
    // When
    let footprints = HashSet::from([original, changed]);
    // Then
    assert_eq!(footprints.len(), 2);
}

#[test]
fn matching_footprint_damage_rejects_when_seed_retains_14px() {
    // Given
    let mut proof = witness(seed(EmissionInput::Static(declaration(
        "font-size",
        "14px",
    ))));
    proof.expansion = Expansion::Static(vec![property("24px")]);
    let damaged_sheet_record = property("24px");
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(proof.expansion.records(), vec![&damaged_sheet_record]);
    assert_eq!(result, Err(ReplayError::Expansion));
}

#[test]
fn exact_cleanup_retains_unconditional_reset_when_global_consumer_is_removed() {
    // Given
    let proof = global_dynamic();
    // When
    let result = require_some(require_ok(proof.after_cleanup("raw.tsx", "delivery.tsx")));
    // Then
    let survivors = require_ok(result.materialized());
    assert_eq!(survivors.len(), 1);
    let RecordFootprint::Property { record, level, .. } = survivors[0] else {
        panic!("reset")
    };
    assert_eq!(
        (
            *level,
            record.property.as_str(),
            record.value.as_str(),
            record.owner_reset,
            &record.selector
        ),
        (0, "--v", "initial", true, &None)
    );
}

#[rstest]
#[case("other.tsx", "delivery.tsx")]
#[case("raw.tsx", "other.tsx")]
fn cleanup_retains_complete_when_frozen_owner_or_bucket_is_unaffected(
    #[case] source: &str,
    #[case] bucket: &str,
) {
    // Given
    let proof = global_dynamic();
    // When
    let result = require_some(require_ok(proof.after_cleanup(source, bucket)));
    // Then
    assert_eq!(result.materialization, Materialization::Complete);
    assert_eq!(require_ok(result.materialized()).len(), 2);
}

#[test]
fn cleanup_drops_witness_when_all_typography_members_are_removed() {
    // Given
    let mut declaration = declaration("typography", "heading");
    declaration.preset = Some(preset());
    declaration.selector = Some(StyleSelector::Global("body".into(), "raw.tsx".into()));
    let proof = witness(seed(EmissionInput::Typography(declaration)));
    // When
    let result = proof.after_cleanup("raw.tsx", "delivery.tsx");
    // Then
    assert_eq!(result, Ok(None));
}

#[test]
fn orphan_reset_rejects_before_cleanup_projection_when_complete_pair_is_damaged() {
    // Given
    let mut proof = global_dynamic();
    let Expansion::Dynamic { consumer, .. } = &mut proof.expansion else {
        panic!("dynamic")
    };
    *consumer.as_mut() = property("24px");
    proof.materialization = Materialization::AfterGlobalCleanup {
        source: "raw.tsx".into(),
        bucket: "delivery.tsx".into(),
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Expansion));
}

#[test]
fn unchanged_cleanup_receipt_rejects_when_bucket_cannot_remove_consumer() {
    // Given
    let mut proof = global_dynamic();
    proof.materialization = Materialization::AfterGlobalCleanup {
        source: "raw.tsx".into(),
        bucket: "other".into(),
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Cleanup));
}

#[test]
fn multi_witness_storage_retains_distinct_expansions_when_candidate_slot_is_shared() {
    // Given
    let first = witness(seed(EmissionInput::Static(declaration(
        "font-size",
        "14px",
    ))));
    let second = witness(seed(EmissionInput::Static(declaration(
        "font-size",
        "24px",
    ))));
    let mut evidence = CounterEvidence::default();
    // When
    for proof in [first.clone(), second, first] {
        require_ok(
            evidence.insert(crate::counter_evidence_allocation_tests::counter_proof(
                proof,
            )),
        );
    }
    // Then
    let values: Vec<_> = evidence.counters[""][&0]
        .iter()
        .map(|proof| {
            let Expansion::Static(records) = &proof.emission.expansion else {
                panic!("static")
            };
            let RecordFootprint::Property { record, .. } = &records[0] else {
                panic!("property")
            };
            record.value.as_str()
        })
        .collect();
    assert_eq!(values, vec!["14px", "24px"]);
}

#[test]
fn raw_payload_and_owner_cleanup_use_source_when_delivery_bucket_differs() {
    // Given
    let records = [
        RecordFootprint::Css {
            source: "raw.tsx".into(),
            css: concat!("body", "{", "color:red", "}").into(),
        },
        RecordFootprint::Import {
            source: "raw.tsx".into(),
            url: "theme.css".into(),
        },
        RecordFootprint::FontFace {
            source: "raw.tsx".into(),
            properties: BTreeMap::from([("src".into(), "url(font.woff2)".into())]),
        },
        RecordFootprint::GlobalCssOwner {
            source: "raw.tsx".into(),
        },
    ];
    // When
    let removed: Vec<_> = records
        .iter()
        .map(|record| record.removed_by("raw.tsx", "other"))
        .collect();
    // Then
    assert_eq!(removed, vec![true, true, true, true]);
}

#[test]
fn at_cleanup_matches_raw_owner_when_outer_chain_is_present() {
    // Given
    let mut footprint = property("14px");
    let RecordFootprint::Property { record, .. } = &mut footprint else {
        panic!("property")
    };
    record.selector = Some(StyleSelector::At {
        kind: AtRuleKind::Media,
        query: "(min-width:800px)".into(),
        outer: vec![AtRule {
            kind: AtRuleKind::Supports,
            query: "(display:grid)".into(),
        }],
        selector: Some("body".into()),
        file: Some("raw.tsx".into()),
    });
    // When
    let removed = footprint.removed_by("raw.tsx", "delivery.tsx");
    // Then
    assert!(removed);
}

#[test]
fn typography_cleanup_receipt_rejects_when_it_would_remove_all_members() {
    // Given
    let mut declaration = declaration("typography", "heading");
    declaration.preset = Some(preset());
    declaration.selector = Some(StyleSelector::Global("body".into(), "raw.tsx".into()));
    let mut proof = witness(seed(EmissionInput::Typography(declaration)));
    proof.materialization = Materialization::AfterGlobalCleanup {
        source: "raw.tsx".into(),
        bucket: "delivery.tsx".into(),
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Cleanup));
}

#[test]
fn repeated_cleanup_retains_receipt_when_only_plain_reset_survives() {
    // Given
    let proof = require_some(require_ok(
        global_dynamic().after_cleanup("raw.tsx", "delivery.tsx"),
    ));
    // When
    let result = require_some(require_ok(proof.after_cleanup("raw.tsx", "delivery.tsx")));
    // Then
    assert_eq!(require_ok(result.materialized()).len(), 1);
    assert_eq!(
        result.materialization,
        Materialization::AfterGlobalCleanup {
            source: "raw.tsx".into(),
            bucket: "delivery.tsx".into()
        }
    );
}
