use super::{
    BatchPhase, Cleanup, FrozenAuthority, LinkedBatch, phase,
    scratch::ScratchRequest,
    state::{CounterState, Rejection},
    state_live,
};
use crate::{
    StyleSheet,
    counter_evidence::{CounterEvidence, RecordFootprint},
};

pub(crate) fn literal<O>(
    sheet: &mut StyleSheet,
    mutate: impl FnOnce(&mut StyleSheet) -> (O, Option<RecordFootprint>),
) -> O {
    css::admission::with_admission(|| {
        let base = checked_base(sheet);
        let (output, record) = mutate(sheet);
        if let (Some(base), Some(record)) = (base, record) {
            let mut authority = FrozenAuthority::live();
            authority.authored.push(record);
            let incoming = LinkedBatch::link_captured_batch(
                &[],
                &CounterEvidence {
                    authored: authority.authored.clone(),
                    ..CounterEvidence::default()
                },
                &authority,
            );
            let projected = incoming.and_then(|incoming| {
                phase::stage(&base, &ScratchRequest::new(&incoming, None)?, None)
            });
            publish(sheet, projected.map_err(super::KernelError::from));
        }
        output
    })
}

pub(crate) fn cleanup(sheet: &mut StyleSheet, source: &str, single_css: bool) -> bool {
    css::admission::with_admission(|| {
        let base = checked_base(sheet);
        let (cleaned, bucket) = sheet.rm_global_css_capture(source, single_css);
        if let (Some(bucket), Some(base)) = (bucket, base) {
            let cleanup = Cleanup {
                source: source.into(),
                bucket,
                single_css,
            };
            let incoming = LinkedBatch {
                candidates: Vec::new(),
                evidence: CounterEvidence::default(),
                records: Vec::new(),
                config: base.config.clone(),
                cleanups: Vec::new(),
                phase: BatchPhase::Fresh,
            };
            let projected = ScratchRequest::new(&incoming, Some(&cleanup))
                .and_then(|request| phase::stage(&base, &request, Some(&cleanup)));
            publish(sheet, projected.map_err(super::KernelError::from));
        }
        cleaned
    })
}

fn checked_base(sheet: &mut StyleSheet) -> Option<LinkedBatch> {
    sheet.counter_state.as_ref()?;
    let result = state_live::validate(sheet);
    checked(sheet, result)
}

fn checked<T>(sheet: &mut StyleSheet, result: Result<T, super::KernelError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            if let Some(state) = &mut sheet.counter_state {
                state.rejection.get_or_insert(Rejection(error));
            }
            None
        }
    }
}

fn publish(sheet: &mut StyleSheet, projected: Result<phase::StagedState, super::KernelError>) {
    let projected = checked(sheet, projected);
    if let (Some(state), Some(projected)) = (&mut sheet.counter_state, projected) {
        let mut authority = state.projection(&state_live::maps().classes);
        authority.authored = projected.evidence.authored;
        authority.cleanups = projected.cleanups;
        authority.phase = projected.phase;
        authority.retain_references(&projected.candidates);
        *state = CounterState::from_captured(projected.candidates, authority);
    }
}
