use super::{BatchPhase, Candidate, Cleanup, FrozenAuthority};
use crate::{counter_evidence::RecordFootprint, emission_seed::EmissionContext};
use css::{
    allocation_input::{CapturedDelivery, CapturedNameConfig},
    counter_names::NameAddress,
};
use std::collections::BTreeMap;

/// The sheet's single canonical witness graph. Checked projections are temporary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CounterState {
    pub(super) counters: BTreeMap<String, BTreeMap<usize, Vec<Candidate>>>,
    pub(super) baseline: Vec<Candidate>,
    pub(super) authored: Vec<RecordFootprint>,
    pub(super) config: CapturedNameConfig,
    pub(super) threshold: Option<usize>,
    pub(super) originals: BTreeMap<String, u32>,
    pub(super) files: BTreeMap<String, usize>,
    pub(super) placements: Vec<EmissionContext>,
    pub(super) deliveries: BTreeMap<String, CapturedDelivery>,
    pub(super) cleanups: Vec<Cleanup>,
    pub(super) phase: BatchPhase,
    pub(super) rejection: Option<Rejection>,
}

/// Irreversible for this sheet; a transaction rollback restores its prior latch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rejection(pub(super) super::KernelError);

impl CounterState {
    pub(super) fn from_captured(candidates: Vec<Candidate>, authority: FrozenAuthority) -> Self {
        let mut state = Self {
            counters: BTreeMap::new(),
            baseline: Vec::new(),
            authored: authority.authored,
            config: authority.config,
            threshold: css::atom_hoist::atom_hoist_threshold(),
            originals: authority.originals,
            files: authority.files,
            placements: authority.placements,
            deliveries: authority.deliveries,
            cleanups: authority.cleanups,
            phase: authority.phase,
            rejection: None,
        };
        for candidate in candidates {
            let witnesses = match &candidate.proof.allocation.allocation.address {
                NameAddress::Counter {
                    namespace, slot, ..
                } => state
                    .counters
                    .entry(namespace.clone())
                    .or_default()
                    .entry(*slot)
                    .or_default(),
                NameAddress::Baseline { .. } => &mut state.baseline,
            };
            if !witnesses.contains(&candidate) {
                witnesses.push(candidate);
            }
        }
        state
    }

    pub(super) fn candidates(&self) -> impl Iterator<Item = &Candidate> {
        self.counters
            .values()
            .flat_map(|slots| slots.values())
            .flatten()
            .chain(&self.baseline)
    }

    #[cfg(test)]
    pub(super) fn candidates_mut(&mut self) -> impl Iterator<Item = &mut Candidate> {
        self.counters
            .values_mut()
            .flat_map(|slots| slots.values_mut())
            .flatten()
            .chain(&mut self.baseline)
    }

    pub(super) fn projection(&self, classes: &crate::cache_snapshot::ClassMap) -> FrozenAuthority {
        FrozenAuthority {
            config: self.config.clone(),
            originals: self.originals.clone(),
            files: self.files.clone(),
            classes: classes.clone(),
            placements: self.placements.clone(),
            deliveries: self.deliveries.clone(),
            authored: self.authored.clone(),
            cleanups: self.cleanups.clone(),
            phase: self.phase.clone(),
        }
    }
}
