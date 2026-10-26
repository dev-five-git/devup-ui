//! Private native implementation; adapter.cjs alone defines the package ABI.
mod access;
mod de;
mod host;
mod output;
mod value;

pub use host::{Resolved, extract, has_devup_ui, init, reserve, reset, set_resolver};
pub use output::NativeOutput;

use devup_ui_binding_core as core;
use napi::bindgen_prelude::{Either, Undefined, Unknown};
use napi::{Error, Result};
use napi_derive::napi;
use std::collections::{BTreeMap, HashMap, HashSet};
use value::{Access, Wasm32Index, decode, widen_index};

macro_rules! import {
    ($name:ident, $target:path, $kind:ty, $doc:literal) => {
        #[doc = $doc]
        #[napi]
        pub fn $name<'env>(value: Unknown<'env>, access: Access<'env>) -> Result<()> {
            $target(decode::<$kind>(value, &access)?);
            Ok(())
        }
    };
}
import!(
    import_sheet,
    core::import_sheet_internal,
    sheet::StyleSheet,
    "Restore a completely decoded sheet."
);
import!(
    register_theme,
    core::register_theme_internal,
    sheet::theme::Theme,
    "Register a completely decoded theme."
);
import!(register_shorthands, core::register_shorthands_internal, BTreeMap<String, Vec<String>>, "Register typed shorthand targets.");
import!(import_canonical_map, core::import_canonical_map_internal, HashMap<String, String>, "Restore canonical bucket identities.");
import!(import_file_routes, core::import_file_routes_internal, HashMap<String, HashSet<u32>>, "Restore iterable route sets.");
/// Restore wasm32-sized class slots, widening only after complete decoding.
#[napi]
pub fn import_class_map<'env>(value: Unknown<'env>, access: Access<'env>) -> Result<()> {
    let map: HashMap<String, HashMap<String, Wasm32Index>> = decode(value, &access)?;
    core::import_class_map_internal(
        map.into_iter()
            .map(|(file, entries)| {
                (
                    file,
                    entries
                        .into_iter()
                        .map(|(key, index)| (key, index.0))
                        .collect(),
                )
            })
            .collect(),
    );
    Ok(())
}
/// Restore file slots using the same bimap decoding and wasm32 width as WASM.
#[napi]
pub fn import_file_map<'env>(value: Unknown<'env>, access: Access<'env>) -> Result<()> {
    let map: bimap::BiHashMap<String, Wasm32Index> = decode(value, &access)?;
    core::import_file_map_internal(
        map.into_iter()
            .map(|(file, index)| (file, index.0))
            .collect(),
    );
    Ok(())
}
macro_rules! export {
    ($name:ident, $target:path) => {
        #[doc = "Return the unchanged core serialization, without normalization."]
        #[napi]
        pub fn $name() -> Result<String> {
            $target().map_err(Error::from_reason)
        }
    };
}
export!(export_sheet, core::export_sheet_internal);
export!(export_class_map, core::export_class_map_internal);
export!(export_file_map, core::export_file_map_internal);
export!(export_canonical_map, core::export_canonical_map_internal);
/// Return the optional prefix as a string or undefined.
#[napi]
pub fn get_prefix() -> Either<String, Undefined> {
    core::get_prefix().into()
}
/// Install a valid string/nullish prefix; malformed wrapper policy remains separate.
#[napi]
pub fn set_prefix(value: Option<String>) {
    core::set_prefix(value);
}
/// Return the default theme as a string or undefined.
#[napi]
pub fn get_default_theme() -> Either<String, Undefined> {
    core::get_default_theme().into()
}
/// Set debug after facade WASM-i32 conversion.
#[napi]
pub fn set_debug(value: bool) {
    core::set_debug(value);
}
/// Read debug state.
#[napi]
pub fn is_debug() -> bool {
    core::is_debug()
}
/// Set hoisting after facade nullish/unsigned-i32 conversion.
#[napi]
pub fn set_atom_hoist(value: Option<u32>) {
    core::set_atom_hoist(value.map(widen_index));
}
/// Materialize CSS for the facade's converted file number.
#[napi]
pub fn get_css(value: Option<u32>, import_main: bool) -> String {
    core::get_css(value.map(widen_index), import_main)
}
/// Seed the array-like wrapper's copied strings, not arbitrary JS iterables.
#[napi]
pub fn seed_file_map(files: Vec<String>) {
    core::seed_file_map(&files);
}
/// Return the live allocation count as a JS number, never a `BigInt`.
#[napi]
pub fn get_unreserved_class_slot_count() -> u32 {
    let [a, b, c, d, ..] = core::get_unreserved_class_slot_count().to_le_bytes();
    u32::from_le_bytes([a, b, c, d])
}
/// Generate declarations with the existing five interface-name slots.
#[napi]
pub fn get_theme_interface(package: String, names: Vec<String>) -> Result<String> {
    let [color, typography, length, shadows, theme]: [String; 5] = names
        .try_into()
        .map_err(|_| Error::from_reason("invalid private interface-name request"))?;
    Ok(core::get_theme_interface(
        &package,
        [&color, &typography, &length, &shadows, &theme],
    ))
}
