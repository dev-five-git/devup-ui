use super::{EvidenceError, ValidatedSnapshot, admission};
use crate::{StyleSheet, cache_snapshot::CacheRestore};

/// Check exact ownership and the target's irreversible rejection before retirement.
/// # Errors
/// Returns active-exact first, then the target's original latched kernel cause.
pub fn check_install_target(target: &StyleSheet) -> Result<(), EvidenceError> {
    css::admission::with_admission(|| {
        css::admission::check_administration_allowed().map_err(EvidenceError::ActiveExact)?;
        match target
            .counter_state
            .as_ref()
            .and_then(|state| state.rejection)
        {
            Some(rejection) => Err(EvidenceError::Kernel(rejection.0)),
            None => Ok(()),
        }
    })
}

impl ValidatedSnapshot {
    /// Consume the certificate and adopt its complete state before compilation.
    /// # Errors
    /// Rejects active exact scopes, a retained target rejection or stale build authority before writes.
    pub fn install_into(self, target: &mut StyleSheet) -> Result<(), EvidenceError> {
        let classes = self
            .authority
            .classes
            .iter()
            .map(|(namespace, slots)| {
                (
                    namespace.clone(),
                    slots
                        .iter()
                        .map(|(key, slot)| (key.clone(), *slot))
                        .collect(),
                )
            })
            .collect();
        let files = self
            .authority
            .files
            .iter()
            .map(|(path, ordinal)| (path.clone(), *ordinal))
            .collect();
        css::admission::with_admission(|| {
            check_install_target(target)?;
            admission::check(&self.authority)?;
            let incoming = self.authority.sheet;
            css::class_map::set_class_map(classes);
            css::file_map::set_file_map(files);
            css::file_map::set_original_ids(incoming.source_ids.clone());
            css::atom_hoist::restore_atom_plan(incoming.atom_plan.clone());
            target.properties = incoming.properties;
            target.css = incoming.css;
            target.keyframes = incoming.keyframes;
            target.global_css_files = incoming.global_css_files;
            target.imports = incoming.imports;
            target.font_faces = incoming.font_faces;
            target.source_ids = incoming.source_ids;
            target.atom_plan = incoming.atom_plan;
            target.counter_state = incoming.counter_state;
            target.names = Default::default();
            target.cache_restore = CacheRestore::Manual;
            Ok(())
        })
    }
}
