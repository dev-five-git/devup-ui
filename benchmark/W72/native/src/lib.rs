//! Private, synchronous bridge over the same core as the WASM binding.

#[expect(
    clippy::needless_pass_by_value,
    reason = "get_theme_interface preserves six measured napi-owned String conversions; napi duplicates function expectations onto generated shims"
)]
mod config;
#[expect(
    clippy::too_many_arguments,
    reason = "The two public extraction functions preserve the positional JavaScript ABI; napi duplicates function-level expectations onto generated shims"
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "has_devup_ui preserves three measured napi-owned String conversions; napi duplicates function expectations onto generated shims"
)]
mod extraction;
mod output;
mod reservation;
mod resolver;
#[expect(
    clippy::needless_pass_by_value,
    reason = "seed_file_map preserves the measured napi-owned Vec<String> conversion; napi duplicates function expectations onto generated shims"
)]
mod state;

pub use config::{
    get_css, get_default_theme, get_prefix, get_theme_interface, import_file_routes, is_debug,
    register_shorthands, register_theme, set_atom_hoist, set_debug, set_prefix,
};
pub use extraction::{code_extract, code_extract_without_source_map, has_devup_ui};
pub use output::Output;
pub use reservation::{get_unreserved_class_slot_count, reserve_class_slots};
pub use resolver::{ResolverModule, set_module_resolver};
pub use state::{
    export_canonical_map, export_class_map, export_file_map, export_sheet, import_canonical_map,
    import_class_map, import_file_map, import_sheet, reset_build_state, seed_file_map,
};

use std::fmt::Display;
use std::sync::{LazyLock, Mutex};

use napi::Error;
use sheet::StyleSheet;

static GLOBAL_STYLE_SHEET: LazyLock<Mutex<StyleSheet>> =
    LazyLock::new(|| Mutex::new(StyleSheet::default()));

fn with_style_sheet<R>(f: impl FnOnce(&StyleSheet) -> R) -> R {
    let guard = GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&guard)
}

fn with_style_sheet_mut<R>(f: impl FnOnce(&mut StyleSheet) -> R) -> R {
    let mut guard = GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut guard)
}

fn js_error(message: impl Display) -> Error {
    Error::from_reason(message.to_string())
}
