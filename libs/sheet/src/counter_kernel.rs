use std::collections::{BTreeMap, BTreeSet};

use css::allocation_input::{CapturedDelivery, CapturedNameConfig};

use crate::{
    counter_evidence::{
        AllocationEvidence, CounterEvidence, ExpansionProof, RecordFootprint, ReplayError,
    },
    emission_seed::EmissionContext,
};

#[path = "counter_kernel_canonical_tests.rs"]
#[cfg(test)]
mod canonical_tests;
#[path = "counter_kernel_cleanup_receipt_tests.rs"]
#[cfg(test)]
mod cleanup_receipt_tests;
#[path = "counter_kernel_cleanup_tests.rs"]
#[cfg(test)]
mod cleanup_tests;
#[path = "counter_kernel_dynamic_fixture.rs"]
#[cfg(test)]
mod dynamic_fixture;
#[path = "counter_kernel_dynamic_tests.rs"]
#[cfg(test)]
mod dynamic_tests;
#[path = "counter_kernel_emission.rs"]
mod emission;
#[path = "counter_kernel_envelope.rs"]
mod envelope;
#[path = "counter_kernel_fixtures.rs"]
#[cfg(test)]
mod fixtures;
#[path = "counter_kernel_frame_tests.rs"]
#[cfg(test)]
mod frame_tests;
#[path = "counter_kernel_legacy.rs"]
mod legacy;
#[path = "counter_kernel_link.rs"]
mod link;
#[path = "counter_kernel_link_damage_tests.rs"]
#[cfg(test)]
mod link_damage_tests;
#[path = "counter_kernel_link_tests.rs"]
#[cfg(test)]
mod link_tests;
#[path = "counter_kernel_phase.rs"]
mod phase;
#[path = "counter_kernel_phase_boundary_tests.rs"]
#[cfg(test)]
mod phase_boundary_tests;
#[path = "counter_kernel_phase_fixtures.rs"]
#[cfg(test)]
mod phase_fixtures;
#[path = "counter_kernel_placement_tests.rs"]
#[cfg(test)]
mod placement_tests;
#[path = "counter_kernel_record_tests.rs"]
#[cfg(test)]
mod record_tests;
#[path = "counter_kernel_records.rs"]
mod records;
#[path = "counter_kernel_replacement_tests.rs"]
#[cfg(test)]
mod replacement_tests;
#[path = "counter_kernel_reset_tests.rs"]
#[cfg(test)]
mod reset_tests;
#[path = "counter_kernel_scratch.rs"]
mod scratch;
#[path = "counter_kernel_scratch_tests.rs"]
#[cfg(test)]
mod scratch_tests;
#[path = "counter_kernel_value_regression_tests.rs"]
#[cfg(test)]
mod value_regression_tests;
#[path = "counter_kernel_value_tests.rs"]
#[cfg(test)]
mod value_tests;
#[path = "counter_kernel_variable_tests.rs"]
#[cfg(test)]
mod variable_tests;

#[path = "counter_authored.rs"]
pub(crate) mod authored;
#[path = "counter_kernel_authority.rs"]
mod authority;
#[path = "counter_kernel_capture.rs"]
mod capture;
#[path = "counter_kernel_error.rs"]
mod error;
#[path = "counter_kernel_live.rs"]
mod live;
#[path = "counter_kernel_prepare.rs"]
mod prepare;
#[path = "counter_kernel_production.rs"]
mod production;
#[path = "counter_kernel_publication.rs"]
mod publication;
#[path = "counter_state.rs"]
pub(crate) mod state;
#[path = "counter_state_live.rs"]
mod state_live;
#[path = "counter_kernel_traversal.rs"]
mod traversal;
#[path = "counter_state_validation.rs"]
pub(crate) mod validation;
pub use error::{KernelError, UpdateError};
pub use live::{
    CompletedUpdate, CounterSheet, KernelAttempt, PreparedUpdate, UpdateEffects, UpdateRequest,
};
#[cfg(test)]
#[path = "counter_kernel_authentic_damage_tests.rs"]
mod authentic_damage_tests;
#[cfg(test)]
#[path = "counter_kernel_authentic_support.rs"]
mod authentic_support;
#[cfg(test)]
#[path = "counter_kernel_coverage_boundary_tests.rs"]
mod coverage_boundary_tests;
#[cfg(test)]
#[path = "counter_kernel_coverage_support_tests.rs"]
mod coverage_support_tests;
#[cfg(test)]
#[path = "counter_kernel_generated_ownership_tests.rs"]
mod generated_ownership_tests;
#[cfg(test)]
#[path = "counter_kernel_keyframe_order_tests.rs"]
mod keyframe_order_tests;
#[cfg(test)]
#[path = "counter_kernel_live_cleanup_tests.rs"]
mod live_cleanup_tests;
#[cfg(test)]
#[path = "counter_kernel_live_tests.rs"]
mod live_tests;
#[cfg(test)]
#[path = "counter_kernel_lookup_tests.rs"]
mod lookup_tests;
#[cfg(test)]
#[path = "counter_kernel_mutation_tests.rs"]
mod mutation_tests;
#[cfg(test)]
#[path = "counter_kernel_owned_tests.rs"]
mod owned_tests;
#[cfg(test)]
#[path = "counter_kernel_prepare_tests.rs"]
mod prepare_tests;
#[cfg(test)]
#[path = "counter_kernel_rejection_tests.rs"]
mod rejection_tests;
#[cfg(test)]
#[path = "counter_kernel_retention_tests.rs"]
mod retention_tests;
#[cfg(test)]
#[path = "counter_kernel_transaction_tests.rs"]
mod transaction_tests;

/// Explicit immutable build authority, never captured from allocator globals.
#[derive(Clone, Debug, PartialEq, Eq)]
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
