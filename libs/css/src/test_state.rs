//! Hidden cross-crate test support. Guards require the existing serial test lock;
//! they provide cleanup, not mutual exclusion. Keep scopes on their creating thread
//! and use them only when extraction and callbacks are quiescent.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::Ordering;

use crate::{
    atom_hoist, class_map, debug, file_map, file_routes, slot_reservation, tailwind_definitions,
    theme_tokens,
};

pub(crate) fn replace_shorthands(
    state: (BTreeMap<String, Vec<String>>, bool),
) -> (BTreeMap<String, Vec<String>>, bool) {
    let mut registry = crate::CUSTOM_SHORTHANDS
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let old = std::mem::replace(&mut *registry, state.0);
    let present = crate::HAS_CUSTOM_SHORTHANDS.swap(state.1, Ordering::Relaxed);
    drop(registry);
    crate::CUSTOM_SHORTHANDS.clear_poison();
    (old, present)
}

/// Reset CSS-owned persistent state without retaining a snapshot.
pub fn reset_state_for_testing() {
    crate::set_prefix(None);
    debug::set_debug(false);
    atom_hoist::set_atom_hoist(None);
    atom_hoist::restore_atom_plan(None);
    slot_reservation::reset();
    class_map::reset_class_map();
    file_map::reset_file_map();
    file_map::reset_canonical_map();
    file_routes::reset_file_routes();
    crate::reset_custom_shorthands();
    theme_tokens::reset_theme_tokens();
    tailwind_definitions::replace_definitions(Default::default());
}

/// Starts with clean CSS state and restores the exact prior state on Drop,
/// including unwind and nested scopes. Hold a named binding and the serial lock.
#[must_use]
pub struct TestStateGuard {
    prefix: Option<String>,
    debug: bool,
    hoist: Option<usize>,
    plan: Option<BTreeSet<String>>,
    reservation: slot_reservation::State,
    classes: HashMap<String, HashMap<String, usize>>,
    files: bimap::BiHashMap<String, usize>,
    canonical: HashMap<String, String>,
    routes: HashMap<String, HashSet<u32>>,
    shorthands: (BTreeMap<String, Vec<String>>, bool),
    tokens: theme_tokens::ThemeTokenRegistry,
    tailwind: tailwind_definitions::Definitions,
    same_thread: PhantomData<Rc<()>>,
}

impl TestStateGuard {
    /// Snapshot owned values; no state lock is held during the test body.
    pub fn new() -> Self {
        let guard = Self {
            prefix: crate::get_prefix(),
            debug: debug::is_debug(),
            hoist: atom_hoist::atom_hoist_threshold(),
            plan: atom_hoist::atom_plan(),
            reservation: slot_reservation::replace_state(Default::default()),
            classes: class_map::get_class_map(),
            files: file_map::get_file_map(),
            canonical: file_map::get_canonical_map(),
            routes: file_routes::get_file_routes(),
            shorthands: replace_shorthands((BTreeMap::new(), false)),
            tokens: theme_tokens::replace_registry(Default::default()),
            tailwind: tailwind_definitions::replace_definitions(Default::default()),
            same_thread: PhantomData,
        };
        reset_state_for_testing();
        guard
    }
}

impl Default for TestStateGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestStateGuard {
    fn drop(&mut self) {
        crate::set_prefix(self.prefix.take());
        debug::set_debug(self.debug);
        atom_hoist::set_atom_hoist(self.hoist);
        atom_hoist::restore_atom_plan(self.plan.take());
        slot_reservation::replace_state(std::mem::take(&mut self.reservation));
        class_map::set_class_map(std::mem::take(&mut self.classes));
        file_map::set_file_map(std::mem::take(&mut self.files));
        file_map::set_canonical_map(std::mem::take(&mut self.canonical));
        file_routes::set_file_routes(std::mem::take(&mut self.routes));
        replace_shorthands(std::mem::take(&mut self.shorthands));
        theme_tokens::replace_registry(std::mem::take(&mut self.tokens));
        tailwind_definitions::replace_definitions(std::mem::take(&mut self.tailwind));
    }
}

#[cfg(test)]
mod tests;
