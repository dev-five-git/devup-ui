//! Everything the CSS crate keeps between calls, held as one value so an
//! engine session can own it while another session's state is live.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::atomic::Ordering;

use crate::{
    atom_hoist, class_map, debug, file_map, file_routes, slot_reservation, tailwind_definitions,
    theme_tokens,
};

/// The prefix, debug flag, atom hoisting, class and file numbering, routes,
/// shorthands, theme tokens and Tailwind definitions of one engine session.
///
/// The default value is a clean engine, as after a full reset.
#[derive(Default)]
pub struct CssState {
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
}

impl CssState {
    /// Exchange this state with the live one: afterwards the live engine runs
    /// on what this value held, and this value holds what was live.
    pub fn swap_live(&mut self) {
        let prefix = crate::get_prefix();
        crate::set_prefix(std::mem::replace(&mut self.prefix, prefix));
        let debug = debug::is_debug();
        debug::set_debug(std::mem::replace(&mut self.debug, debug));
        let hoist = atom_hoist::atom_hoist_threshold();
        atom_hoist::set_atom_hoist(std::mem::replace(&mut self.hoist, hoist));
        let plan = atom_hoist::atom_plan();
        atom_hoist::restore_atom_plan(std::mem::replace(&mut self.plan, plan));
        self.reservation = slot_reservation::replace_state(self.reservation);
        class_map::with_class_map_mut(|live| std::mem::swap(live, &mut self.classes));
        file_map::with_file_map_mut(|live| std::mem::swap(live, &mut self.files));
        file_map::with_canonical_map_mut(|live| std::mem::swap(live, &mut self.canonical));
        file_routes::with_file_routes_mut(|live| std::mem::swap(live, &mut self.routes));
        self.shorthands = replace_shorthands(std::mem::take(&mut self.shorthands));
        self.tokens = theme_tokens::replace_registry(std::mem::take(&mut self.tokens));
        self.tailwind =
            tailwind_definitions::replace_definitions(std::mem::take(&mut self.tailwind));
    }
}

/// Install `state` as the live custom shorthands, returning the previous ones
/// and clearing a poisoned lock.
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
