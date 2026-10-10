use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};

use crate::{file_map::canonical, file_routes::with_file_routes};

static FROZEN_PLAN: LazyLock<Mutex<Option<BTreeSet<String>>>> = LazyLock::new(|| Mutex::new(None));

/// Freeze every eligible canonical bucket before any source is processed.
pub fn freeze_atom_plan() {
    let _admission = crate::admission::enter();
    if let Some(threshold) = atom_hoist_threshold() {
        let _root = crate::root_held::RootHeld::enter("atom_plan");
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
    let _admission = crate::admission::enter();
    let _root = crate::root_held::RootHeld::enter("atom_plan");
    FROZEN_PLAN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// Restore a cached plan, or clear it when starting a new build.
pub fn restore_atom_plan(plan: Option<BTreeSet<String>>) {
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("restore_atom_plan");
    let _root = crate::root_held::RootHeld::enter("atom_plan");
    *FROZEN_PLAN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = plan;
}

/// Restore only the frozen-plan snapshot owned by a private exact attempt.
pub(crate) fn restore_atom_plan_snapshot(snapshot: Option<BTreeSet<String>>) {
    let _admission = crate::admission::enter();
    let _root = crate::root_held::RootHeld::enter("atom_plan");
    *FROZEN_PLAN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = snapshot;
}

/// Whether this bucket was eligible at the build's extraction boundary.
pub fn is_hoisted_bucket(filename: &str) -> bool {
    let _admission = crate::admission::enter();
    freeze_atom_plan();
    let _root = crate::root_held::RootHeld::enter("atom_plan");
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
    let _admission = crate::admission::enter();
    crate::admission::assert_administration_allowed("set_atom_hoist");
    ATOM_HOIST_THRESHOLD.store(threshold.unwrap_or(0), Ordering::Relaxed);
}

#[inline(always)]
#[must_use]
pub fn atom_hoist_threshold() -> Option<usize> {
    let _admission = crate::admission::enter();
    match ATOM_HOIST_THRESHOLD.load(Ordering::Relaxed) {
        0 => None,
        v => Some(v),
    }
}

#[inline(always)]
#[must_use]
pub fn is_atom_hoist() -> bool {
    let _admission = crate::admission::enter();
    ATOM_HOIST_THRESHOLD.load(Ordering::Relaxed) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_atom_hoist() {
        set_atom_hoist(None);
        assert!(!is_atom_hoist());
        assert_eq!(atom_hoist_threshold(), None);
        set_atom_hoist(Some(3));
        assert!(is_atom_hoist());
        assert_eq!(atom_hoist_threshold(), Some(3));
        set_atom_hoist(None);
        assert!(!is_atom_hoist());
    }
}
