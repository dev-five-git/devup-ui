use super::{BatchPhase, Cleanup, LinkedBatch, emission, phase, records};
use crate::{
    KeyframesMap, PropertyMap, StyleSheet, StyleSheetCss,
    counter_evidence::{CounterEvidence, ReplayError},
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct ScratchRequest<'a> {
    incoming: &'a LinkedBatch,
    cleanup: Option<&'a Cleanup>,
}

impl<'a> ScratchRequest<'a> {
    pub(super) fn new(
        incoming: &'a LinkedBatch,
        cleanup: Option<&'a Cleanup>,
    ) -> Result<Self, ReplayError> {
        match &incoming.phase {
            BatchPhase::Fresh => Ok(Self { incoming, cleanup }),
            BatchPhase::Retained(_) => Err(ReplayError::Cleanup),
        }
    }
}

/// Emission collections and retained registration state only; no live installation.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct ScratchUpdate {
    pub(super) properties: BTreeMap<String, PropertyMap>,
    pub(super) css: BTreeMap<String, BTreeSet<StyleSheetCss>>,
    pub(super) keyframes: KeyframesMap,
    pub(super) global_css_files: BTreeSet<String>,
    pub(super) imports: BTreeMap<String, BTreeSet<String>>,
    pub(super) font_faces: BTreeMap<String, BTreeSet<BTreeMap<String, String>>>,
    pub(super) evidence: CounterEvidence,
    pub(super) candidates: Vec<super::Candidate>,
    pub(super) cleanups: Vec<Cleanup>,
    pub(super) phase: BatchPhase,
    pub(super) collected: bool,
    pub(super) updated_base_style: bool,
    pub(super) cleaned: bool,
}

pub(super) fn apply_scratch(
    sheet: &StyleSheet,
    base: &LinkedBatch,
    request: ScratchRequest<'_>,
) -> Result<ScratchUpdate, ReplayError> {
    records::coverage(sheet, &base.records)?;
    if base.config != request.incoming.config {
        return Err(ReplayError::Allocation);
    }
    if let Some(cleanup) = request.cleanup {
        let target = if cleanup.single_css {
            String::new()
        } else {
            css::file_map::canonical(&cleanup.source)
        };
        if target != cleanup.bucket {
            return Err(ReplayError::Cleanup);
        }
    }
    let untouched = phase::stage(base, request.incoming, None)?;
    let projected = phase::stage(base, request.incoming, request.cleanup)?;
    let mut scratch = emission::clone_sheet(sheet);
    let cleaned = request
        .cleanup
        .is_some_and(|cleanup| scratch.rm_global_css(&cleanup.source, cleanup.single_css));
    let state = if cleaned { projected } else { untouched };
    let (collected, updated_base_style) = emission::apply(&mut scratch, request.incoming);
    Ok(ScratchUpdate {
        properties: scratch.properties,
        css: scratch.css,
        keyframes: scratch.keyframes,
        global_css_files: scratch.global_css_files,
        imports: scratch.imports,
        font_faces: scratch.font_faces,
        evidence: state.evidence,
        candidates: state.candidates,
        cleanups: state.cleanups,
        phase: state.phase,
        collected,
        updated_base_style,
        cleaned,
    })
}
