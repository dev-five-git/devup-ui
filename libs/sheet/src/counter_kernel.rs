use std::collections::{BTreeMap, BTreeSet};

use css::allocation_input::{CapturedDelivery, CapturedNameConfig};

use crate::{
    counter_evidence::{
        AllocationEvidence, CounterEvidence, ExpansionProof, RecordFootprint, ReplayError,
    },
    emission_seed::EmissionContext,
};

#[path = "counter_kernel_canonical_tests.rs"]
mod canonical_tests;
#[path = "counter_kernel_cleanup_receipt_tests.rs"]
mod cleanup_receipt_tests;
#[path = "counter_kernel_cleanup_tests.rs"]
mod cleanup_tests;
#[path = "counter_kernel_dynamic_fixture.rs"]
mod dynamic_fixture;
#[path = "counter_kernel_dynamic_tests.rs"]
mod dynamic_tests;
#[path = "counter_kernel_emission.rs"]
mod emission;
#[path = "counter_kernel_envelope.rs"]
mod envelope;
#[path = "counter_kernel_fixtures.rs"]
mod fixtures;
#[path = "counter_kernel_frame_tests.rs"]
mod frame_tests;
#[path = "counter_kernel_legacy.rs"]
mod legacy;
#[path = "counter_kernel_link.rs"]
mod link;
#[path = "counter_kernel_link_damage_tests.rs"]
mod link_damage_tests;
#[path = "counter_kernel_link_tests.rs"]
mod link_tests;
#[path = "counter_kernel_phase.rs"]
mod phase;
#[path = "counter_kernel_phase_boundary_tests.rs"]
mod phase_boundary_tests;
#[path = "counter_kernel_phase_fixtures.rs"]
mod phase_fixtures;
#[path = "counter_kernel_placement_tests.rs"]
mod placement_tests;
#[path = "counter_kernel_record_tests.rs"]
mod record_tests;
#[path = "counter_kernel_records.rs"]
mod records;
#[path = "counter_kernel_replacement_tests.rs"]
mod replacement_tests;
#[path = "counter_kernel_reset_tests.rs"]
mod reset_tests;
#[path = "counter_kernel_scratch.rs"]
mod scratch;
#[path = "counter_kernel_scratch_tests.rs"]
mod scratch_tests;
#[path = "counter_kernel_value_regression_tests.rs"]
mod value_regression_tests;
#[path = "counter_kernel_value_tests.rs"]
mod value_tests;
#[path = "counter_kernel_variable_tests.rs"]
mod variable_tests;

/// Explicit immutable build authority, never captured from allocator globals.
#[derive(Clone, Debug)]
struct FrozenAuthority {
    config: CapturedNameConfig,
    originals: BTreeMap<String, u32>,
    files: BTreeMap<String, usize>,
    classes: BTreeMap<String, BTreeMap<String, usize>>,
    placements: Vec<EmissionContext>,
    deliveries: BTreeMap<String, CapturedDelivery>,
    authored: Vec<RecordFootprint>,
    cleanups: Vec<Cleanup>,
    phase: BatchPhase,
}

/// Retained registrations are execution state, not inferred from cleanup history.
#[derive(Clone, Debug, PartialEq, Eq)]
enum BatchPhase {
    Fresh,
    Retained(BTreeSet<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Cleanup {
    source: String,
    bucket: String,
    single_css: bool,
}

/// Originals are independent of delivery and assignment-site identities.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Lineage {
    parent: u32,
    children: Vec<u32>,
    variable: Option<VariableLineage>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VariableLineage {
    original: u32,
    evidence: AllocationEvidence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Candidate {
    proof: ExpansionProof,
    lineage: Lineage,
}

/// Only link's checked constructor can create a batch; no unchecked builder.
#[derive(Debug)]
struct LinkedBatch {
    candidates: Vec<Candidate>,
    evidence: CounterEvidence,
    records: Vec<RecordFootprint>,
    config: CapturedNameConfig,
    cleanups: Vec<Cleanup>,
    phase: BatchPhase,
}

impl LinkedBatch {
    fn link_captured_batch(
        candidates: &[Candidate],
        evidence: &CounterEvidence,
        authority: &FrozenAuthority,
    ) -> Result<Self, ReplayError> {
        let mut reconstructed = CounterEvidence {
            authored: authority.authored.clone(),
            ..CounterEvidence::default()
        };
        for candidate in candidates {
            link::check(candidate, authority)?;
            reconstructed.insert(candidate.proof.clone())?;
        }
        if &reconstructed != evidence {
            return Err(ReplayError::Expansion);
        }
        match &authority.phase {
            BatchPhase::Fresh => {
                if !authority.cleanups.is_empty()
                    || candidates.iter().any(|candidate| {
                        candidate.proof.emission.materialization
                            != crate::counter_evidence::Materialization::Complete
                    })
                {
                    return Err(ReplayError::Cleanup);
                }
            }
            BatchPhase::Retained(_) => {}
        }
        let records = records::expected(&reconstructed, &authority.phase)?;
        Ok(Self {
            candidates: candidates.to_vec(),
            evidence: reconstructed,
            records,
            config: authority.config.clone(),
            cleanups: authority.cleanups.clone(),
            phase: authority.phase.clone(),
        })
    }
}
