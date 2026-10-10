use super::{scratch::ScratchUpdate, state_live};
use crate::StyleSheet;

/// This owner is outside CSS exact ownership and inside admission.
pub(super) struct Publication<'a> {
    pub(super) sheet: &'a mut StyleSheet,
    before: Option<StyleSheet>,
}
impl<'a> Publication<'a> {
    pub(super) fn checked(sheet: &'a mut StyleSheet) -> Result<Self, super::KernelError> {
        let before = Some(state_live::capture_owned(sheet)?);
        Ok(Self { sheet, before })
    }

    #[cfg(test)]
    pub(super) fn new(sheet: &'a mut StyleSheet) -> Self {
        let mut before = super::emission::clone_sheet(sheet);
        before.atom_plan.clone_from(&sheet.atom_plan);
        before.counter_state.clone_from(&sheet.counter_state);
        Self {
            sheet,
            before: Some(before),
        }
    }
    pub(super) fn commit(&mut self) {
        self.before = None;
    }
}
impl Drop for Publication<'_> {
    fn drop(&mut self) {
        if let Some(before) = self.before.take() {
            self.sheet.properties = before.properties;
            self.sheet.css = before.css;
            self.sheet.keyframes = before.keyframes;
            self.sheet.global_css_files = before.global_css_files;
            self.sheet.imports = before.imports;
            self.sheet.font_faces = before.font_faces;
            self.sheet.atom_plan = before.atom_plan;
            self.sheet.counter_state = before.counter_state;
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
