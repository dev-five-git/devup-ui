//! Exact mutable CSS naming-state rollback, subordinate to CSS admission.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use bimap::BiHashMap;

use crate::{
    admission,
    atom_hoist::{atom_plan, restore_atom_plan_snapshot},
    class_map::{Attempt, get_class_map, set_class_map},
    file_map::{get_file_map, get_original_ids, restore_file_map_snapshot},
    sparse_site::source_ids::restore_original_ids_snapshot,
};

struct NamingSnapshot {
    classes: HashMap<String, HashMap<String, usize>>,
    files: BiHashMap<String, usize>,
    originals: BTreeMap<String, u32>,
    plan: Option<BTreeSet<String>>,
}

struct ExactAttempt {
    attempt: Option<Attempt>,
    before: Option<NamingSnapshot>,
}

impl Drop for ExactAttempt {
    fn drop(&mut self) {
        drop(self.attempt.take());
        if let Some(before) = self.before.take() {
            set_class_map(before.classes);
            restore_file_map_snapshot(before.files);
            restore_original_ids_snapshot(before.originals);
            restore_atom_plan_snapshot(before.plan);
        }
    }
}

/// Build under exclusive admission, restoring mutable CSS naming state on error/unwind.
///
/// Full class, delivery-file, original-ID and frozen-plan snapshots are restored
/// after journal rollback, even while an enclosing exact scope remains active.
/// Nested exact attempts and standalone journal attempts remain subordinate.
/// Success commits only after `build` returns `Ok`.
///
/// # Errors
/// Returns the callback's error unchanged.
/// # Panics
/// Propagates callback panics after journal rollback and exact map restoration.
/// Panics/traps that do not run destructors cannot be rolled back.
pub fn with_exclusive_attempt<O, E>(build: impl FnOnce() -> Result<O, E>) -> Result<O, E> {
    admission::with_admission(|| {
        let before = NamingSnapshot {
            classes: get_class_map(),
            files: get_file_map(),
            originals: get_original_ids(),
            plan: atom_plan(),
        };
        let _scope = admission::ExactScope::enter();
        let mut exact = ExactAttempt {
            before: Some(before),
            attempt: Some(Attempt::begin()),
        };
        match build() {
            Ok(output) => {
                if let Some(attempt) = exact.attempt.take() {
                    attempt.commit();
                }
                exact.before = None;
                Ok(output)
            }
            Err(error) => Err(error),
        }
    })
}
