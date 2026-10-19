use std::cell::Cell;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

use crate::{file_map::canonical, file_routes::with_file_routes};

static FROZEN_PLAN: LazyLock<Mutex<Option<BTreeSet<String>>>> = LazyLock::new(|| Mutex::new(None));

thread_local! {
    static SUPPRESSED: Cell<bool> = const { Cell::new(false) };
}

/// Suppress effective atom mode for actual single-CSS work without changing build configuration.
#[must_use]
pub struct AtomModeGuard {
    previous: bool,
    thread_bound: PhantomData<Rc<()>>,
}

impl AtomModeGuard {
    pub fn new(single_css: bool) -> Self {
        let previous = SUPPRESSED.with(|state| state.replace(state.get() || single_css));
        Self {
            previous,
            thread_bound: PhantomData,
        }
    }
}

impl Drop for AtomModeGuard {
    fn drop(&mut self) {
        SUPPRESSED.with(|state| state.set(self.previous));
    }
}

/// Freeze every eligible canonical bucket before any source is processed.
pub fn freeze_atom_plan() {
    if !is_atom_hoist() {
        return;
    }
    if let Some(threshold) = atom_hoist_threshold() {
        let mut plan = FROZEN_PLAN
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if plan.is_none() {
            let buckets = with_file_routes(|routes| {
                let mut buckets: HashMap<String, HashSet<u32>> = HashMap::new();
                for (file, reach) in routes {
                    buckets.entry(canonical(file)).or_default().extend(reach);
                }
                buckets
            });
            *plan = Some(
                buckets
                    .into_iter()
                    .filter_map(|(bucket, routes)| (routes.len() >= threshold).then_some(bucket))
                    .collect(),
            );
        }
    }
}

/// Snapshot the immutable build plan for sheet persistence.
pub fn atom_plan() -> Option<BTreeSet<String>> {
    FROZEN_PLAN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Restore a cached plan, or clear it when starting a new build.
pub fn restore_atom_plan(plan: Option<BTreeSet<String>>) {
    *FROZEN_PLAN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = plan;
}

/// Whether this bucket was eligible at the build's extraction boundary.
pub fn is_hoisted_bucket(filename: &str) -> bool {
    if !is_atom_hoist() {
        return false;
    }
    freeze_atom_plan();
    FROZEN_PLAN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .is_some_and(|plan| plan.contains(&canonical(filename)))
}

// Zero disables atom mode; eligible bucket reach is frozen before extraction.
static ATOM_HOIST_THRESHOLD: AtomicUsize = AtomicUsize::new(0);

#[inline(always)]
pub fn set_atom_hoist(threshold: Option<usize>) {
    ATOM_HOIST_THRESHOLD.store(threshold.unwrap_or(0), Ordering::Relaxed);
}

#[inline(always)]
#[must_use]
pub fn atom_hoist_threshold() -> Option<usize> {
    match ATOM_HOIST_THRESHOLD.load(Ordering::Relaxed) {
        0 => None,
        v => Some(v),
    }
}

#[inline(always)]
#[must_use]
pub fn is_atom_hoist() -> bool {
    ATOM_HOIST_THRESHOLD.load(Ordering::Relaxed) != 0 && !SUPPRESSED.with(Cell::get)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_atom_hoist() {
        let _state = crate::test_state::TestStateGuard::new();
        set_atom_hoist(None);
        assert!(!is_atom_hoist());
        assert_eq!(atom_hoist_threshold(), None);
        set_atom_hoist(Some(3));
        assert!(is_atom_hoist());
        assert_eq!(atom_hoist_threshold(), Some(3));
        set_atom_hoist(None);
        assert!(!is_atom_hoist());
    }

    #[test]
    #[serial]
    fn suppression_restores_nested_mode_and_preserves_requested_plan() {
        // Given
        set_atom_hoist(Some(2));
        restore_atom_plan(Some(BTreeSet::from(["shared".into()])));
        let plan = atom_plan();
        // When
        {
            let _outer = AtomModeGuard::new(true);
            assert!(!is_atom_hoist());
            let _inner = AtomModeGuard::new(false);
            assert!(!is_atom_hoist());
            assert!(!is_hoisted_bucket("shared"));
            freeze_atom_plan();
            assert_eq!(atom_hoist_threshold(), Some(2));
        }
        // Then
        assert!(is_atom_hoist());
        assert_eq!(atom_plan(), plan);
        assert!(is_hoisted_bucket("shared"));
        set_atom_hoist(None);
        restore_atom_plan(None);
    }

    #[test]
    #[serial]
    fn suppression_restores_after_error_and_unwind_without_freezing() {
        // Given
        set_atom_hoist(Some(2));
        restore_atom_plan(None);
        let error = || -> Result<(), &'static str> {
            let _guard = AtomModeGuard::new(true);
            freeze_atom_plan();
            Err("test boundary")
        };
        // When
        assert_eq!(error(), Err("test boundary"));
        assert!(is_atom_hoist());
        let unwind = std::panic::catch_unwind(|| {
            let _guard = AtomModeGuard::new(true);
            freeze_atom_plan();
            panic!("test unwind");
        });
        // Then
        assert!(unwind.is_err());
        assert!(is_atom_hoist());
        assert_eq!(atom_plan(), None);
        set_atom_hoist(None);
    }
}
