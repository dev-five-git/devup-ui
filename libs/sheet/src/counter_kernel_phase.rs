use super::{BatchPhase, Candidate, Cleanup, LinkedBatch, records};
use crate::counter_evidence::{CounterEvidence, ReplayError};

pub(super) struct StagedState {
    pub(super) evidence: CounterEvidence,
    pub(super) candidates: Vec<Candidate>,
    pub(super) cleanups: Vec<Cleanup>,
    pub(super) phase: BatchPhase,
}

pub(super) fn stage(
    base: &LinkedBatch,
    incoming: &LinkedBatch,
    cleanup: Option<&Cleanup>,
) -> Result<StagedState, ReplayError> {
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
    candidates.extend(incoming.candidates.iter().cloned());
    let mut authored: Vec<_> = base
        .evidence
        .authored
        .iter()
        .filter(|record| {
            cleanup.is_none_or(|cleanup| !record.removed_by(&cleanup.source, &cleanup.bucket))
        })
        .cloned()
        .collect();
    for record in &incoming.evidence.authored {
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
