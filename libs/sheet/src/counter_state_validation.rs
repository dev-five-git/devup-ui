use super::{KernelError, LinkedBatch, records, state::CounterState};
use crate::{
    StyleSheet,
    cache_snapshot::{ClassMap, FileMap},
    counter_evidence::CounterEvidence,
};
use css::{
    allocation_input::{CapturedNameConfig, LegacyInput, NameMode},
    counter_names::NameAddress,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct RegistryMaps {
    pub(super) classes: ClassMap,
    pub(super) files: FileMap,
    pub(super) originals: BTreeMap<String, u32>,
}

pub(super) struct RegistryView<'a> {
    pub(super) classes: &'a ClassMap,
    pub(super) files: &'a FileMap,
    pub(super) originals: &'a BTreeMap<String, u32>,
}

pub(super) enum PlanPolicy {
    Live,
    Snapshot,
}

pub(super) struct BorrowedInput<'a> {
    pub(super) sheet: &'a StyleSheet,
    pub(super) state: &'a CounterState,
    pub(super) maps: RegistryView<'a>,
    pub(super) build: &'a BuildConfig,
    pub(super) policy: PlanPolicy,
}

pub(super) struct BuildConfig {
    pub(super) config: CapturedNameConfig,
    pub(super) threshold: Option<usize>,
    pub(super) atom_plan: Option<BTreeSet<String>>,
}

/// Explicit live compatibility; snapshot compatibility is a later admission concern.
pub(super) struct ValidationInput<'a> {
    pub(super) sheet: &'a StyleSheet,
    pub(super) state: &'a CounterState,
    pub(super) maps: &'a RegistryMaps,
    pub(super) build: &'a BuildConfig,
}

/// Pure linkage: supplied maps/config/plan only, never allocator or registry calls.
pub(super) fn validate(input: ValidationInput<'_>) -> Result<LinkedBatch, KernelError> {
    let ValidationInput {
        sheet,
        state,
        maps,
        build,
    } = input;
    validate_borrowed(BorrowedInput {
        sheet,
        state,
        maps: RegistryView {
            classes: &maps.classes,
            files: &maps.files,
            originals: &maps.originals,
        },
        build,
        policy: PlanPolicy::Live,
    })
}

pub(super) fn validate_borrowed(input: BorrowedInput<'_>) -> Result<LinkedBatch, KernelError> {
    let BorrowedInput {
        sheet,
        state,
        maps,
        build,
        policy,
    } = input;
    if let Some(rejection) = state.rejection {
        return Err(rejection.0);
    }
    if state.config != build.config
        || state.threshold != build.threshold
        || match policy {
            PlanPolicy::Live => sheet.atom_plan != build.atom_plan,
            PlanPolicy::Snapshot => build.atom_plan.is_some() && sheet.atom_plan != build.atom_plan,
        }
        || state
            .originals
            .iter()
            .any(|(path, id)| maps.originals.get(path) != Some(id))
        || state
            .files
            .iter()
            .any(|(path, id)| maps.files.get(path) != Some(id))
    {
        return Err(KernelError::Authority);
    }
    for (namespace, slots) in &state.counters {
        for (slot, candidates) in slots {
            for candidate in candidates {
                if !matches!(&candidate.proof.allocation.allocation.address,
                    NameAddress::Counter { namespace: actual, slot: actual_slot, .. }
                    if actual == namespace && actual_slot == slot)
                {
                    return Err(KernelError::Authority);
                }
            }
        }
    }
    if state.baseline.iter().any(|candidate| {
        !matches!(
            candidate.proof.allocation.allocation.address,
            NameAddress::Baseline { .. }
        )
    }) {
        return Err(KernelError::Authority);
    }
    let authority = state.projection(maps.classes);
    let candidates: Vec<_> = state.candidates().cloned().collect();
    let mut evidence = CounterEvidence {
        authored: state.authored.clone(),
        ..CounterEvidence::default()
    };
    for candidate in &candidates {
        let placement = &candidate.proof.emission.seed.placement;
        let input = super::legacy::input(&candidate.proof.emission.seed.body, state.config.mode);
        let eligible = state.config.mode == NameMode::AtomHoist
            && !placement.single_css
            && matches!(&input, LegacyInput::Declaration(declaration) if declaration.order != Some(0));
        let expected = eligible
            && sheet
                .atom_plan
                .as_ref()
                .is_some_and(|plan| plan.contains(&placement.bucket));
        if placement.hoisted != expected
            || authority
                .delivery(placement)
                .is_some_and(|delivery| delivery.hoisted != expected)
        {
            return Err(KernelError::Authority);
        }
        evidence.insert(candidate.proof.clone())?;
    }
    let linked = LinkedBatch::link_captured_batch(&candidates, &evidence, &authority)?;
    records::coverage(sheet, &linked.records)?;
    Ok(linked)
}
