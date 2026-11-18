//! Hidden cross-crate test support. Guards require the existing serial test lock;
//! they provide cleanup, not mutual exclusion. Keep scopes on their creating thread
//! and use them only when extraction and callbacks are quiescent.

use std::marker::PhantomData;
use std::rc::Rc;

use crate::{
    atom_hoist, class_map, debug, engine_state::CssState, file_map, file_routes, license_banner,
    slot_reservation, style_resolution, tailwind_definitions, theme_tokens,
};

/// Reset CSS-owned persistent state without retaining a snapshot.
pub fn reset_state_for_testing() {
    crate::set_prefix(None);
    debug::set_debug(false);
    license_banner::set_license_banner(true);
    style_resolution::set_style_resolution(Default::default());
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
    outer: CssState,
    same_thread: PhantomData<Rc<()>>,
}

impl TestStateGuard {
    /// Move the live state into the guard; no state lock is held during the test body.
    pub fn new() -> Self {
        let mut outer = CssState::default();
        outer.swap_live();
        Self {
            outer,
            same_thread: PhantomData,
        }
    }
}

impl Default for TestStateGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TestStateGuard {
    fn drop(&mut self) {
        self.outer.swap_live();
    }
}

#[cfg(test)]
mod tests;
