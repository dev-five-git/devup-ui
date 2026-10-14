use crate::{
    counter_evidence::*,
    emission_seed::*,
    emission_seed_test_helpers::{declaration, property, require_ok, seed},
};

#[test]
fn emission_witness_roundtrips_when_serialized_separately_from_current_sheet() {
    // Given
    let input = seed(EmissionInput::Static(declaration("font-size", "14px")));
    let witness = EmissionWitness {
        expansion: require_ok(input.replay("a")),
        seed: input,
        name: "a".into(),
        materialization: Materialization::Complete,
    };
    let serialized = require_ok(serde_json::to_string(&witness));
    // When
    let restored: EmissionWitness = require_ok(serde_json::from_str(&serialized));
    // Then
    assert_eq!(require_ok(restored.materialized()), vec![&property("14px")]);
}

#[test]
fn multi_witness_storage_retains_inputs_when_legacy_equivalence_emits_same_record() {
    // Given
    let first = seed(EmissionInput::Static(declaration("font-size", "14px")));
    let mut explicit = declaration("font-size", "14px");
    explicit.style_order = Some(255);
    let second = seed(EmissionInput::Static(explicit));
    let proofs = [first, second].map(|seed| EmissionWitness {
        expansion: require_ok(seed.replay("a")),
        seed,
        name: "a".into(),
        materialization: Materialization::Complete,
    });
    let mut evidence = CounterEvidence::default();
    // When
    for proof in proofs {
        require_ok(
            evidence.insert(crate::counter_evidence_allocation_tests::counter_proof(
                proof,
            )),
        );
    }
    // Then
    let witnesses = &evidence.counters[""][&0];
    assert_eq!(witnesses.len(), 2);
    for witness in witnesses {
        assert_eq!(require_ok(witness.materialized()), vec![&property("14px")]);
    }
}
