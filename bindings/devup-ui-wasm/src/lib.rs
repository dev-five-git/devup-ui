mod compiled_guard;
mod configuration;
mod configuration_state;
mod driver;
mod extraction;
mod extraction_forward;
mod output;
mod owner_codec;
mod reservation;
mod resident_owner;
mod resolver;
mod sheet_checkpoint;
mod startup;
mod state;
mod state_forward;

#[cfg(not(tarpaulin_include))]
pub use compiled_guard::compiled_guard_facts;
pub use compiled_guard::compiled_guard_facts_internal;
pub use configuration::*;
use extraction::{SourceMapMode, checked_extraction, code_extract_internal_impl};
pub use extraction::{
    code_extract_internal, code_extract_with_modules_internal,
    code_extract_without_source_map_internal,
};
#[cfg(not(tarpaulin_include))]
pub use extraction_forward::*;
pub use output::Output;
#[cfg(not(tarpaulin_include))]
pub use owner_codec::{
    import_canonical_map_json, import_class_map_json, import_file_map_json,
    import_reservation_state_json, import_sheet_json,
};
pub use owner_codec::{
    import_canonical_map_json_internal, import_class_map_json_internal,
    import_file_map_json_internal, import_reservation_state_json_internal,
    import_sheet_json_internal,
};
pub use reservation::{
    ReservationInput, get_suppressed_reservation_row_count, get_unreserved_class_slot_count,
    reserve_class_slots_internal,
};
pub use state::{reset_state_for_testing, reset_state_internal};
pub use state_forward::*;

use sheet::StyleSheet;
use std::{
    cell::RefCell,
    sync::{LazyLock, Mutex},
};

static GLOBAL_STYLE_SHEET: LazyLock<Mutex<StyleSheet>> =
    LazyLock::new(|| Mutex::new(StyleSheet::default()));
thread_local! {
    static MODULE_RESOLVER: RefCell<Option<js_sys::Function>> = const { RefCell::new(None) };
}

fn with_style_sheet<F: FnOnce(&StyleSheet) -> R, R>(f: F) -> R {
    let guard = GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&guard)
}

fn with_style_sheet_mut<F: FnOnce(&mut StyleSheet) -> R, R>(f: F) -> R {
    let mut guard = GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut guard)
}

#[cfg(test)]
use css::{
    class_map::set_class_map,
    file_map::{canonical, set_file_map},
};
#[cfg(test)]
use extractor::{
    ExtractOption, ImportAlias, ModuleResolution, ModuleResolver, ResolvedModule, extract,
    has_devup_ui_with,
};
#[cfg(test)]
use rustc_hash::FxHashSet;
#[cfg(test)]
use state::TestStateGuard;
#[cfg(test)]
use std::collections::{BTreeMap, HashMap};

#[cfg(test)]
mod atom_hoist_measurement;
#[cfg(test)]
mod atom_tests;
#[cfg(test)]
mod cache_protocol_tests;
#[cfg(test)]
mod checkpoint_pair_tests;
#[cfg(test)]
mod css_output_edge_tests;
#[cfg(test)]
mod deferred_default_tests;
#[cfg(test)]
mod owner_codec_error_tests;
#[cfg(test)]
mod owner_codec_remaining_error_tests;
#[cfg(test)]
mod owner_codec_remaining_tests;
#[cfg(test)]
mod owner_codec_tests;
#[cfg(test)]
mod semantic_configuration_tests;
#[cfg(test)]
mod semantic_replay_tests;
#[cfg(test)]
mod sheet_checkpoint_tests;
#[cfg(test)]
mod source_type_tests;
#[cfg(test)]
include!("legacy_tests.rs");
