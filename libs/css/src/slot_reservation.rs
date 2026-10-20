//! Tracks live allocator misses without changing counter keys or spelling.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::class_map::{get_class_map, set_class_map};
use crate::file_map::{get_file_map, set_file_map};
mod row;
pub use row::RowSlots;
pub(crate) use row::{record_class, record_file};
#[cfg(test)]
mod row_tests;

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
enum Phase {
    #[default]
    Legacy,
    Scratch,
    Live,
}

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
struct State {
    phase: Phase,
    misses: usize,
    suppressed: usize,
}

#[cfg_attr(target_arch = "wasm32", path = "slot_reservation/state_wasm.rs")]
#[cfg_attr(not(target_arch = "wasm32"), path = "slot_reservation/state_native.rs")]
mod state;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "slot_reservation/state_wasm.rs"]
mod wasm_state_for_tests;

use state::with_state;

/// Opaque reservation bookkeeping captured alongside an integration's allocator maps.
#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Snapshot(State);

/// Capture both the active accounting phase and its accumulated live misses.
#[must_use]
pub fn capture() -> Snapshot {
    Snapshot(with_state(|state| *state))
}

/// Restore bookkeeping after the matching integration's allocator maps are restored.
pub fn restore(snapshot: Snapshot) {
    with_state(|state| *state = snapshot.0);
}

pub(crate) fn record_insert() {
    with_state(|state| match state.phase {
        Phase::Legacy | Phase::Scratch => {}
        Phase::Live => state.misses = state.misses.saturating_add(1),
    });
}

/// Number of new live keys inserted since reservation activated this build.
#[must_use]
pub fn unreserved_slot_count() -> usize {
    with_state(|state| state.misses)
}

/// Number of failed rows suppressed by the most recently completed reservation batch.
#[must_use]
pub fn suppressed_row_count() -> usize {
    with_state(|state| state.suppressed)
}

/// Record a failed scratch row independently of actual live allocator misses.
pub fn record_suppressed_row() {
    with_state(|state| state.suppressed = state.suppressed.saturating_add(1));
}

/// Clear reservation bookkeeping at the existing build-reset boundary.
pub fn reset() {
    with_state(|state| *state = State::default());
}

pub(crate) struct FileSeedGuard(Phase);

impl Default for FileSeedGuard {
    fn default() -> Self {
        Self(with_state(|state| {
            let phase = state.phase;
            state.phase = Phase::Scratch;
            phase
        }))
    }
}

impl Drop for FileSeedGuard {
    fn drop(&mut self) {
        with_state(|state| state.phase = self.0);
    }
}

/// Suppresses scratch misses and rolls back allocator additions unless committed.
pub struct ScratchSlots {
    original: HashMap<String, HashMap<String, usize>>,
    files: bimap::BiHashMap<String, usize>,
    state: State,
    committed: bool,
}

impl Default for ScratchSlots {
    fn default() -> Self {
        let original = get_class_map();
        let files = get_file_map();
        let state = with_state(|state| {
            let saved = *state;
            state.phase = Phase::Scratch;
            state.suppressed = 0;
            saved
        });
        Self {
            original,
            files,
            state,
            committed: false,
        }
    }
}

impl ScratchSlots {
    /// Keep additions and activate live-miss accounting, including for an empty batch.
    pub fn commit(mut self) {
        self.committed = true;
    }
}

impl Drop for ScratchSlots {
    fn drop(&mut self) {
        if self.committed {
            with_state(|state| {
                *state = State {
                    phase: Phase::Live,
                    suppressed: state.suppressed,
                    ..self.state
                }
            });
        } else {
            set_class_map(std::mem::take(&mut self.original));
            set_file_map(std::mem::take(&mut self.files));
            with_state(|state| *state = self.state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn wasm_state_access_preserves_mutations_and_reset() {
        wasm_state_for_tests::with_state(|state| {
            state.phase = Phase::Scratch;
            state.misses = 7;
        });
        assert_eq!(wasm_state_for_tests::with_state(|state| state.misses), 7);
        wasm_state_for_tests::with_state(|state| *state = State::default());
        assert_eq!(wasm_state_for_tests::with_state(|state| state.misses), 0);
    }

    #[test]
    #[serial]
    fn insertions_count_only_when_live_and_keep_legacy_names() {
        // Given
        reset();
        crate::debug::set_debug(false);
        crate::class_map::reset_class_map();
        assert_eq!(crate::sheet_to_variable_name("gap", 0, None), "--a");
        let slots = ScratchSlots::default();
        assert_eq!(crate::sheet_to_variable_name("width", 0, None), "--b");
        slots.commit();
        // When
        let first = crate::sheet_to_variable_name("color", 0, None);
        let repeated = crate::sheet_to_variable_name("color", 0, None);
        // Then
        assert_eq!((first, repeated), ("--c".into(), "--c".into()));
        assert_eq!(unreserved_slot_count(), 1);
        ScratchSlots::default().commit();
        assert_eq!(unreserved_slot_count(), 1);
        reset();
        assert_eq!(unreserved_slot_count(), 0);
    }

    #[test]
    #[serial]
    fn owner_restore_keeps_live_phase_and_prior_misses() {
        // Given
        reset();
        ScratchSlots::default().commit();
        record_insert();
        let saved = capture();
        reset();
        // When
        restore(saved);
        record_insert();
        // Then
        assert_eq!(unreserved_slot_count(), 2);
        reset();
    }

    #[test]
    #[serial]
    fn unwind_restores_allocator_and_previous_live_count() {
        // Given
        reset();
        crate::debug::set_debug(false);
        crate::class_map::reset_class_map();
        ScratchSlots::default().commit();
        record_insert();
        let before = get_class_map();
        // When
        let result = std::panic::catch_unwind(|| {
            let _slots = ScratchSlots::default();
            record_insert();
            drop(crate::sheet_to_variable_name("gap", 0, None));
            panic!("reservation unwind");
        });
        // Then
        assert!(result.is_err());
        assert_eq!(get_class_map(), before);
        assert_eq!(unreserved_slot_count(), 1);
        reset();
    }
}
