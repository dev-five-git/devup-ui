use super::super::{
    state_live,
    validation::{self, BorrowedInput, PlanPolicy, RegistryView},
};
use super::{Authority, EvidenceError};
use std::collections::BTreeSet;

fn dense<T: Ord + Copy + TryInto<usize>>(values: impl Iterator<Item = T>, count: usize) -> bool {
    let ids: BTreeSet<_> = values.collect();
    ids.len() == count
        && ids
            .into_iter()
            .enumerate()
            .all(|(expected, actual)| actual.try_into().is_ok_and(|actual| actual == expected))
}

pub(super) fn check(authority: &Authority) -> Result<(), EvidenceError> {
    if !dense(authority.files.values().copied(), authority.files.len())
        || !dense(
            authority.sheet.source_ids.values().copied(),
            authority.sheet.source_ids.len(),
        )
        || authority
            .classes
            .values()
            .any(|slots| !dense(slots.values().copied(), slots.len()))
    {
        return Err(EvidenceError::Map);
    }
    let state = authority
        .sheet
        .counter_state
        .as_ref()
        .ok_or(EvidenceError::State)?;
    if state
        .cleanups
        .iter()
        .any(|cleanup| cleanup.single_css && !cleanup.bucket.is_empty())
    {
        return Err(EvidenceError::Cleanup);
    }
    let build = state_live::build();
    if state.config != build.config
        || state.threshold != build.threshold
        || (build.atom_plan.is_some() && build.atom_plan != authority.sheet.atom_plan)
    {
        return Err(EvidenceError::Configuration);
    }
    validation::validate_borrowed(BorrowedInput {
        sheet: &authority.sheet,
        state,
        maps: RegistryView {
            classes: &authority.classes,
            files: &authority.files,
            originals: &authority.sheet.source_ids,
        },
        build: &build,
        policy: PlanPolicy::Snapshot,
    })
    .map_err(EvidenceError::Kernel)?;
    Ok(())
}
