use super::{BatchPhase, Candidate, Cleanup, LinkedBatch, records};
use crate::counter_evidence::{CounterEvidence, ReplayError};

#[cfg(test)]
#[path = "counter_kernel_gate_regression_tests.rs"]
mod gate_regression_tests;

pub(super) struct StagedState {
    pub(super) evidence: CounterEvidence,
    pub(super) candidates: Vec<Candidate>,
    pub(super) cleanups: Vec<Cleanup>,
    pub(super) phase: BatchPhase,
}

pub(super) fn stage(
    base: &LinkedBatch,
    request: &super::scratch::ScratchRequest<'_>,
    cleanup: Option<&Cleanup>,
) -> Result<StagedState, ReplayError> {
    let incoming = request.incoming;
    let mut candidates = Vec::new();
    for candidate in &base.candidates {
        let emission = match cleanup {
            Some(cleanup) => candidate
                .proof
                .emission
                .after_cleanup(&cleanup.source, &cleanup.bucket)?,
            None => Some(candidate.proof.emission.clone()),
        };
        if let Some(emission) = emission {
            let mut candidate = candidate.clone();
            candidate.proof.emission = emission;
            candidates.push(candidate);
        }
    }
    let mut authored: Vec<_> = base
        .evidence
        .authored
        .iter()
        .filter(|record| {
            cleanup.is_none_or(|cleanup| !record.removed_by(&cleanup.source, &cleanup.bucket))
        })
        .cloned()
        .collect();
    if let Some(operations) = request.operations {
        for record in operations {
            retire(&mut candidates, &mut authored, record);
        }
    }
    for candidate in &incoming.candidates {
        for record in candidate.proof.materialized()? {
            retire(&mut candidates, &mut authored, record);
        }
        candidates.push(candidate.clone());
    }
    for record in &incoming.evidence.authored {
        retire(&mut candidates, &mut authored, record);
        if !authored.contains(record) {
            authored.push(record.clone());
        }
    }
    let mut evidence = CounterEvidence {
        authored,
        ..CounterEvidence::default()
    };
    for candidate in &candidates {
        evidence.insert(candidate.proof.clone())?;
    }
    let mut owners = base
        .records
        .iter()
        .filter_map(|record| match record {
            crate::counter_evidence::RecordFootprint::GlobalCssOwner { source } => {
                Some(source.clone())
            }
            crate::counter_evidence::RecordFootprint::Property { .. }
            | crate::counter_evidence::RecordFootprint::Keyframes { .. }
            | crate::counter_evidence::RecordFootprint::Css { .. }
            | crate::counter_evidence::RecordFootprint::Import { .. }
            | crate::counter_evidence::RecordFootprint::FontFace { .. } => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    if let Some(cleanup) = cleanup {
        owners.remove(&cleanup.source);
    }
    owners.extend(records::registrations(&incoming.records));
    let phase = BatchPhase::Retained(owners);
    let _staged_records = records::expected(&evidence, &phase)?;
    let mut cleanups = base.cleanups.clone();
    if let Some(cleanup) = cleanup {
        cleanups.push(cleanup.clone());
    }
    Ok(StagedState {
        evidence,
        candidates,
        cleanups,
        phase,
    })
}

/// Keyed replacement retires payload authority, never allocator reservations.
pub(super) fn retire(
    candidates: &mut Vec<Candidate>,
    authored: &mut Vec<crate::counter_evidence::RecordFootprint>,
    replacement: &crate::counter_evidence::RecordFootprint,
) {
    use crate::counter_evidence::{Expansion, RecordFootprint};
    if let RecordFootprint::Keyframes {
        bucket,
        name,
        steps,
    } = replacement
    {
        let superseded = |record: &RecordFootprint| {
            matches!(record,
            RecordFootprint::Keyframes { bucket: b, name: n, steps: s }
            if b == bucket && n == name && s != steps)
        };
        candidates.retain(|candidate| match &candidate.proof.emission.expansion {
            Expansion::Keyframes { record, .. } => !superseded(record),
            Expansion::Static(_) | Expansion::Dynamic { .. } | Expansion::Typography { .. } => true,
        });
        authored.retain(|record| !superseded(record));
    }
}
