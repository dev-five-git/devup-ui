use super::{emission, live::KernelEvidence, scratch::ScratchUpdate};
use crate::StyleSheet;
use std::collections::BTreeSet;

/// This owner is outside CSS exact ownership and inside admission.
pub(super) struct Publication<'a> {
    pub(super) sheet: &'a mut StyleSheet,
    pub(super) evidence: &'a mut KernelEvidence,
    before: Option<(StyleSheet, Option<BTreeSet<String>>, KernelEvidence)>,
}
impl<'a> Publication<'a> {
    pub(super) fn new(sheet: &'a mut StyleSheet, evidence: &'a mut KernelEvidence) -> Self {
        let before = Some((
            emission::clone_sheet(sheet),
            sheet.atom_plan.clone(),
            evidence.snapshot(),
        ));
        Self {
            sheet,
            evidence,
            before,
        }
    }
    pub(super) fn commit(&mut self) {
        self.before = None;
    }
}
impl Drop for Publication<'_> {
    fn drop(&mut self) {
        if let Some((before, plan, evidence)) = self.before.take() {
            self.sheet.properties = before.properties;
            self.sheet.css = before.css;
            self.sheet.keyframes = before.keyframes;
            self.sheet.global_css_files = before.global_css_files;
            self.sheet.imports = before.imports;
            self.sheet.font_faces = before.font_faces;
            self.sheet.atom_plan = plan;
            *self.evidence = evidence;
        }
    }
}

pub(super) fn install(sheet: &mut StyleSheet, scratch: ScratchUpdate) {
    sheet.properties = scratch.properties;
    sheet.css = scratch.css;
    sheet.keyframes = scratch.keyframes;
    sheet.global_css_files = scratch.global_css_files;
    sheet.imports = scratch.imports;
    sheet.font_faces = scratch.font_faces;
}
