use css::{allocation_input::*, counter_names::*};
use rstest::rstest;

use crate::{
    counter_evidence::*,
    emission_seed::*,
    emission_seed_test_helpers::{declaration, property, require_ok, require_some, seed},
};

pub(crate) fn counter_proof(emission: EmissionWitness) -> ExpansionProof {
    let EmissionInput::Static(declaration) = &emission.seed.body else {
        panic!("static fixture")
    };
    let input = LegacyInput::Declaration(LegacyDeclaration {
        property: declaration.property.clone(),
        level: declaration.level,
        value: Some(declaration.value.clone()),
        selector: None,
        order: declaration.style_order,
    });
    let context = AllocationContext {
        config: CapturedNameConfig {
            prefix: String::new(),
            mode: NameMode::Counter,
        },
        file: None,
        delivery: None,
    };
    let (namespace, legacy_key) = require_some(allocation_key(&input, &context));
    let allocation = AllocatedName {
        name: "a".into(),
        address: NameAddress::Counter {
            namespace,
            legacy_key,
            slot: 0,
        },
    };
    require_ok(ExpansionProof::new(
        AllocationEvidence {
            input,
            context,
            allocation,
        },
        emission,
    ))
}

fn static_witness() -> EmissionWitness {
    let seed = seed(EmissionInput::Static(declaration("font-size", "14px")));
    EmissionWitness {
        expansion: require_ok(seed.replay("a")),
        seed,
        name: "a".into(),
        materialization: Materialization::Complete,
    }
}

#[test]
fn captured_envelope_replays_css_when_no_live_allocator_lookup_is_needed() {
    // Given
    let proof = counter_proof(static_witness());
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(require_ok(result), vec![&property("14px")]);
    assert_eq!(
        proof.allocation.allocation.address,
        NameAddress::Counter {
            namespace: String::new(),
            legacy_key: "font-size-0-14px--255".into(),
            slot: 0
        }
    );
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
fn allocation_damage_rejects_when_captured_envelope_or_emission_name_disagrees(
    #[case] dimension: u8,
) {
    // Given
    let mut proof = counter_proof(static_witness());
    match dimension {
        0 => {
            let NameAddress::Counter { legacy_key, .. } = &mut proof.allocation.allocation.address
            else {
                panic!("counter")
            };
            *legacy_key = "wrong-key".into();
        }
        1 => {
            let NameAddress::Counter { slot, .. } = &mut proof.allocation.allocation.address else {
                panic!("counter")
            };
            *slot = 1;
        }
        2 => proof.allocation.context.config.prefix = "other".into(),
        3 => proof.emission.name = "b".into(),
        4 => {
            let NameAddress::Counter { namespace, .. } = &mut proof.allocation.allocation.address
            else {
                panic!("counter")
            };
            *namespace = "other".into();
        }
        _ => unreachable!(),
    }
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Allocation));
}

#[rstest]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
fn baseline_evidence_uses_captured_mode_when_no_counter_address_exists(#[case] mode: NameMode) {
    // Given
    let input = LegacyInput::Declaration(LegacyDeclaration {
        property: "font-size".into(),
        level: 0,
        value: Some("14px".into()),
        selector: None,
        order: None,
    });
    let context = AllocationContext {
        config: CapturedNameConfig {
            prefix: "test-".into(),
            mode,
        },
        file: None,
        delivery: None,
    };
    let name = render_name(&input, &context, 0);
    let seed = seed(EmissionInput::Static(declaration("font-size", "14px")));
    let emission = EmissionWitness {
        expansion: require_ok(seed.replay(&name)),
        seed,
        name: name.clone(),
        materialization: Materialization::Complete,
    };
    let allocation = AllocatedName {
        name,
        address: NameAddress::Baseline {
            mode,
            input: input.clone(),
            context: context.clone(),
        },
    };
    let proof = require_ok(ExpansionProof::new(
        AllocationEvidence {
            input,
            context,
            allocation,
        },
        emission,
    ));
    let mut evidence = CounterEvidence::default();
    // When
    require_ok(evidence.insert(proof));
    // Then
    assert_eq!(evidence.baseline.len(), 1);
    let RecordFootprint::Property { record, .. } =
        require_ok(evidence.baseline[0].materialized())[0]
    else {
        panic!("property")
    };
    assert_eq!(record.value, "14px");
    match mode {
        NameMode::Debug => assert_eq!(record.class_name, "test-font-size-0-14px--255"),
        NameMode::AtomHoist => assert_eq!(
            record.class_name,
            "test-a1-g-666f6e742d73697a65-0-s-31347078--255"
        ),
        NameMode::Counter => unreachable!(),
    }
    assert_eq!(evidence.counters, Default::default());
}

#[test]
fn counter_mode_baseline_rejects_when_candidate_tries_to_avoid_slot_identity() {
    // Given
    let mut proof = counter_proof(static_witness());
    proof.allocation.allocation.address = NameAddress::Baseline {
        mode: NameMode::Counter,
        input: proof.allocation.input.clone(),
        context: proof.allocation.context.clone(),
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Allocation));
}

#[test]
fn storage_rejects_damage_when_record_and_footprint_match_but_seed_still_says_14px() {
    // Given
    let mut proof = counter_proof(static_witness());
    proof.emission.expansion = Expansion::Static(vec![property("24px")]);
    let mut evidence = CounterEvidence::default();
    // When
    let result = evidence.insert(proof);
    // Then
    assert_eq!(result, Err(ReplayError::Expansion));
    assert_eq!(evidence.counters, Default::default());
}

#[test]
fn captured_ordinal_replays_private_name_when_delivery_is_independent() {
    // Given
    let mut proof = counter_proof(static_witness());
    proof.allocation.context.file = Some(AllocationFile::Legacy {
        filename: "raw.tsx".into(),
        ordinal: 2,
    });
    proof.allocation.context.delivery = Some(CapturedDelivery {
        canonical: "collapsed.tsx".into(),
        hoisted: false,
    });
    proof.allocation.allocation = AllocatedName {
        name: "c-a".into(),
        address: NameAddress::Counter {
            namespace: "raw.tsx".into(),
            legacy_key: "font-size-0-14px--255-c".into(),
            slot: 0,
        },
    };
    proof.emission.name = "c-a".into();
    proof.emission.expansion = require_ok(proof.emission.seed.replay("c-a"));
    // When
    let result = proof.materialized();
    // Then
    let RecordFootprint::Property { bucket, record, .. } = require_ok(result)[0] else {
        panic!("property")
    };
    assert_eq!(
        (
            bucket.as_str(),
            record.class_name.as_str(),
            record.value.as_str()
        ),
        ("delivery.tsx", "c-a", "14px")
    );
}

#[test]
fn baseline_address_rejects_when_embedded_input_disagrees_with_supplied_input() {
    // Given
    let mut proof = counter_proof(static_witness());
    proof.allocation.context.config.mode = NameMode::Debug;
    proof.allocation.allocation.name = "font-size-0-14px--255".into();
    proof.allocation.allocation.address = NameAddress::Baseline {
        mode: NameMode::Debug,
        input: LegacyInput::Keyframes("different".into()),
        context: proof.allocation.context.clone(),
    };
    // When
    let result = proof.materialized();
    // Then
    assert_eq!(result, Err(ReplayError::Allocation));
}
