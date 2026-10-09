use super::{BatchPhase, Candidate, FrozenAuthority, LinkedBatch, error::KernelError, records};
use crate::{StyleSheet, counter_evidence::CounterEvidence, emission_seed::EmissionContext};
use css::allocation_input::{CapturedDelivery, CapturedNameConfig, NameMode};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn config() -> CapturedNameConfig {
    CapturedNameConfig {
        prefix: css::get_prefix().unwrap_or_default(),
        mode: if css::atom_hoist::is_atom_hoist() {
            NameMode::AtomHoist
        } else if css::debug::is_debug() {
            NameMode::Debug
        } else {
            NameMode::Counter
        },
    }
}

fn placement_key(placement: &EmissionContext) -> String {
    format!(
        "{:?}",
        (
            placement.source_file.as_str(),
            placement.bucket.as_str(),
            placement.single_css,
            placement.hoisted
        )
    )
}

impl FrozenAuthority {
    pub(super) fn delivery(&self, placement: &EmissionContext) -> Option<&CapturedDelivery> {
        self.deliveries
            .get(&placement_key(placement))
            .or_else(|| self.deliveries.get(&placement.source_file))
    }

    pub(super) fn live() -> Self {
        Self {
            config: config(),
            originals: BTreeMap::new(),
            files: BTreeMap::new(),
            classes: classes(),
            placements: Vec::new(),
            deliveries: BTreeMap::new(),
            authored: Vec::new(),
            cleanups: Vec::new(),
            phase: BatchPhase::Fresh,
        }
    }

    /// Retain only referenced path/ordinal pairs from the admitted real registries.
    pub(super) fn capture(
        &mut self,
        candidate: &Candidate,
        delivery: CapturedDelivery,
    ) -> Result<(), KernelError> {
        let placement = &candidate.proof.emission.seed.placement;
        let mut ids: BTreeSet<_> = candidate.lineage.children.iter().copied().collect();
        ids.insert(candidate.lineage.parent);
        if let crate::emission_seed::EmissionInput::Dynamic {
            site: Some(site), ..
        } = &candidate.proof.emission.seed.body
        {
            ids.insert(site.source);
        }
        let originals = css::file_map::get_original_ids();
        if !originals.contains_key(&placement.source_file)
            || !ids
                .iter()
                .all(|id| originals.values().any(|actual| actual == id))
        {
            return Err(KernelError::Authority);
        }
        self.originals.extend(
            originals
                .into_iter()
                .filter(|(path, id)| path == &placement.source_file || ids.contains(id)),
        );
        if !placement.single_css {
            let ordinal = css::file_map::with_file_map(|files| {
                files.get_by_left(&delivery.canonical).copied()
            })
            .ok_or(KernelError::Authority)?;
            self.files.insert(delivery.canonical.clone(), ordinal);
        }
        self.deliveries.insert(placement_key(placement), delivery);
        if !self.placements.contains(placement) {
            self.placements.push(placement.clone());
        }
        Ok(())
    }

    pub(super) fn retain_references(&mut self, candidates: &[Candidate]) {
        let mut ids = BTreeSet::new();
        let mut paths = BTreeSet::new();
        let mut placements = Vec::new();
        for candidate in candidates {
            let placement = &candidate.proof.emission.seed.placement;
            paths.insert(placement.source_file.as_str());
            placements.push(placement);
            ids.insert(candidate.lineage.parent);
            ids.extend(&candidate.lineage.children);
            if let crate::emission_seed::EmissionInput::Dynamic {
                site: Some(site), ..
            } = &candidate.proof.emission.seed.body
            {
                ids.insert(site.source);
            }
        }
        self.originals
            .retain(|path, id| paths.contains(path.as_str()) || ids.contains(id));
        self.placements
            .retain(|placement| placements.contains(&placement));
        let keys: BTreeSet<_> = placements
            .iter()
            .map(|placement| placement_key(placement))
            .collect();
        self.deliveries.retain(|key, _| keys.contains(key));
        let files: BTreeSet<_> = placements
            .iter()
            .filter(|placement| !placement.single_css)
            .filter_map(|placement| self.delivery(placement))
            .map(|delivery| delivery.canonical.clone())
            .collect();
        self.files.retain(|path, _| files.contains(path));
    }

    pub(super) fn refresh(&self) -> Result<Self, KernelError> {
        let originals = css::file_map::get_original_ids();
        let files = css::file_map::get_file_map();
        if self.config != config()
            || self
                .originals
                .iter()
                .any(|(path, id)| originals.get(path) != Some(id))
            || self
                .files
                .iter()
                .any(|(path, id)| files.get_by_left(path) != Some(id))
        {
            return Err(KernelError::Authority);
        }
        let mut authority = self.clone();
        authority.classes = classes();
        Ok(authority)
    }
}

pub(super) fn classes() -> BTreeMap<String, BTreeMap<String, usize>> {
    css::class_map::get_class_map()
        .into_iter()
        .map(|(namespace, map)| (namespace, map.into_iter().collect()))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Retained {
    pub(super) candidates: Vec<Candidate>,
    pub(super) evidence: CounterEvidence,
    pub(super) authority: FrozenAuthority,
    pub(super) plan: Option<BTreeSet<String>>,
}

impl Retained {
    pub(super) fn validate(&self, sheet: &StyleSheet) -> Result<LinkedBatch, KernelError> {
        if sheet.atom_plan != self.plan || css::atom_hoist::atom_plan() != self.plan {
            return Err(KernelError::Authority);
        }
        let authority = self.authority.refresh()?;
        let linked =
            LinkedBatch::link_captured_batch(&self.candidates, &self.evidence, &authority)?;
        records::coverage(sheet, &linked.records)?;
        Ok(linked)
    }
}
