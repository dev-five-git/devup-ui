use std::collections::BTreeMap;

use css::style_selector::StyleSelector;
use serde::{Deserialize, Serialize};

use crate::{
    StyleSheetProperty,
    emission_seed::{EmissionSeed, NumericSite, Yield},
};

#[path = "counter_evidence_allocation.rs"]
pub mod allocation;
pub use allocation::{AllocationEvidence, ExpansionProof};

/// Full record identity. Intentionally no Ord: sheet property ordering is lossy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecordFootprint {
    Property {
        bucket: String,
        order: u8,
        level: u8,
        record: StyleSheetProperty,
    },
    Keyframes {
        bucket: String,
        name: String,
        steps: Vec<(String, Vec<(String, String)>)>,
    },
    Css {
        source: String,
        css: String,
    },
    Import {
        source: String,
        url: String,
    },
    FontFace {
        source: String,
        properties: BTreeMap<String, String>,
    },
    GlobalCssOwner {
        source: String,
    },
}

impl RecordFootprint {
    /// Existing raw-owner cleanup predicate, restricted to its frozen bucket.
    pub fn removed_by(&self, source: &str, bucket: &str) -> bool {
        match self {
            Self::Property {
                bucket: actual,
                record,
                ..
            } => {
                actual == bucket
                    && match &record.selector {
                        Some(
                            StyleSelector::Global(_, owner)
                            | StyleSelector::At {
                                file: Some(owner), ..
                            },
                        ) => owner == source,
                        Some(StyleSelector::Selector(_) | StyleSelector::At { file: None, .. })
                        | None => false,
                    }
            }
            Self::Keyframes { .. } => false,
            Self::Css { source: owner, .. }
            | Self::Import { source: owner, .. }
            | Self::FontFace { source: owner, .. }
            | Self::GlobalCssOwner { source: owner } => owner == source,
        }
    }
}

/// Complete expansion, preserving member multiplicity before set insertion.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Expansion {
    Static(Vec<RecordFootprint>),
    Dynamic {
        variable: String,
        site: Option<NumericSite>,
        important: bool,
        consumer: Box<RecordFootprint>,
        reset: Box<RecordFootprint>,
    },
    Typography {
        preset: String,
        yielded: Vec<Yield>,
        members: Vec<RecordFootprint>,
    },
    Keyframes {
        steps: Vec<(String, Vec<(String, String)>)>,
        record: RecordFootprint,
    },
}

impl Expansion {
    pub fn records(&self) -> Vec<&RecordFootprint> {
        match self {
            Self::Static(members) | Self::Typography { members, .. } => members.iter().collect(),
            Self::Dynamic {
                consumer, reset, ..
            } => vec![consumer.as_ref(), reset.as_ref()],
            Self::Keyframes { record, .. } => vec![record],
        }
    }
}

/// Historical execution receipt, never an arbitrary member mask.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Materialization {
    Complete,
    AfterGlobalCleanup { source: String, bucket: String },
}

/// Pure replay failures; no snapshot/allocator admission is implemented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayError {
    MissingPreset,
    Preset,
    Expansion,
    Cleanup,
    Allocation,
}

/// Emission-only witness; caller-supplied name is NOT allocator authority.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmissionWitness {
    pub name: String,
    pub seed: EmissionSeed,
    pub expansion: Expansion,
    pub materialization: Materialization,
}

impl EmissionWitness {
    /// Model successful guarded cleanup: drop all-removed, retain unaffected.
    pub fn after_cleanup(&self, source: &str, bucket: &str) -> Result<Option<Self>, ReplayError> {
        let records = self.materialized()?;
        let removed = records
            .iter()
            .filter(|record| record.removed_by(source, bucket))
            .count();
        if removed == 0 {
            return Ok(Some(self.clone()));
        }
        if removed == records.len() {
            return Ok(None);
        }
        let mut projected = self.clone();
        projected.materialization = Materialization::AfterGlobalCleanup {
            source: source.into(),
            bucket: bucket.into(),
        };
        projected.materialized()?;
        Ok(Some(projected))
    }

    /// Compare complete independent replay before accepting any cleanup receipt.
    pub fn materialized(&self) -> Result<Vec<&RecordFootprint>, ReplayError> {
        if self.seed.replay(&self.name)? != self.expansion {
            return Err(ReplayError::Expansion);
        }
        let records = self.expansion.records();
        match &self.materialization {
            Materialization::Complete => Ok(records),
            Materialization::AfterGlobalCleanup { source, bucket } => {
                let survivors: Vec<_> = records
                    .iter()
                    .copied()
                    .filter(|record| !record.removed_by(source, bucket))
                    .collect();
                if survivors.is_empty() || survivors.len() == records.len() {
                    return Err(ReplayError::Cleanup);
                }
                Ok(survivors)
            }
        }
    }
}

/// Candidate namespace/usize grouping only, not a forged validated `CounterSlot`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CounterEvidence {
    pub counters: BTreeMap<String, BTreeMap<usize, Vec<ExpansionProof>>>,
    pub baseline: Vec<ExpansionProof>,
    pub authored: Vec<RecordFootprint>,
}

impl CounterEvidence {
    /// Dedupe the complete witness, retaining distinct expansions at one address.
    pub fn insert(&mut self, proof: ExpansionProof) -> Result<(), ReplayError> {
        proof.materialized()?;
        let witnesses = match &proof.allocation.allocation.address {
            css::counter_names::NameAddress::Counter {
                namespace, slot, ..
            } => self
                .counters
                .entry(namespace.clone())
                .or_default()
                .entry(*slot)
                .or_default(),
            css::counter_names::NameAddress::Baseline { .. } => &mut self.baseline,
        };
        if !witnesses.contains(&proof) {
            witnesses.push(proof);
        }
        Ok(())
    }
}
