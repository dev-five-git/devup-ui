use super::{
    FrozenAuthority, KernelError, LinkedBatch, authority, records,
    validation::{self, BuildConfig, RegistryMaps, ValidationInput},
};
use crate::{StyleSheet, counter_evidence::CounterEvidence};

pub(super) fn maps() -> RegistryMaps {
    RegistryMaps {
        classes: authority::classes(),
        files: css::file_map::get_file_map().into_iter().collect(),
        originals: css::file_map::get_original_ids(),
    }
}

pub(super) fn build() -> BuildConfig {
    BuildConfig {
        config: authority::config(),
        threshold: css::atom_hoist::atom_hoist_threshold(),
        atom_plan: css::atom_hoist::atom_plan(),
    }
}

pub(super) fn validate(sheet: &StyleSheet) -> Result<LinkedBatch, KernelError> {
    if let Some(state) = &sheet.counter_state {
        validation::validate(ValidationInput {
            sheet,
            state,
            maps: &maps(),
            build: &build(),
        })
    } else {
        records::coverage(sheet, &[])?;
        if sheet.atom_plan.is_some() && sheet.atom_plan != css::atom_hoist::atom_plan() {
            return Err(KernelError::Authority);
        }
        Ok(LinkedBatch::link_captured_batch(
            &[],
            &CounterEvidence::default(),
            &FrozenAuthority::live(),
        )?)
    }
}

pub(super) fn capture_owned(sheet: &StyleSheet) -> Result<StyleSheet, KernelError> {
    css::admission::with_admission(|| {
        validate(sheet)?;
        let mut owned = super::emission::clone_sheet(sheet);
        owned.atom_plan.clone_from(&sheet.atom_plan);
        owned.counter_state.clone_from(&sheet.counter_state);
        Ok(owned)
    })
}
