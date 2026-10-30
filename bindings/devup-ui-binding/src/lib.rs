//! Private native implementation; adapter.cjs alone defines the package ABI.
mod access;
mod de;
mod host;
mod output;
mod value;

pub use host::{Resolved, extract, has_devup_ui, init, reserve, reset, set_resolver};
pub use output::NativeOutput;

use de::Wasm32Index;
use devup_ui_binding_core as core;
use napi::bindgen_prelude::{Either, Undefined, Unknown};
use napi::{Env, Error, Result};
use napi_derive::napi;
use std::collections::{BTreeMap, HashMap, HashSet};
use value::{Access, AccessCallback, decode, widen_index};

#[doc = "Restore a completely decoded sheet."]
#[napi]
pub fn import_sheet<'env>(
    env: Env,
    value: Unknown<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<()> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    core::import_sheet_internal(decode::<sheet::StyleSheet>(value, &access, 0)?);
    Ok(())
}
#[doc = "Register a completely decoded theme."]
#[napi]
pub fn register_theme<'env>(
    env: Env,
    value: Unknown<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<()> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    core::register_theme_internal(decode::<sheet::theme::Theme>(value, &access, 0)?);
    Ok(())
}
#[doc = "Register typed shorthand targets."]
#[napi]
pub fn register_shorthands<'env>(
    env: Env,
    value: Unknown<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<()> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    core::register_shorthands_internal(decode::<BTreeMap<String, Vec<String>>>(value, &access, 0)?)
        .map_err(|error| Error::from_reason(error.to_string()))
}
#[doc = "Restore canonical bucket identities."]
#[napi]
pub fn import_canonical_map<'env>(
    env: Env,
    value: Unknown<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<()> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    core::import_canonical_map_internal(decode::<HashMap<String, String>>(value, &access, 0)?);
    Ok(())
}
#[doc = "Restore iterable route sets."]
#[napi]
pub fn import_file_routes<'env>(
    env: Env,
    value: Unknown<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<()> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    core::import_file_routes_internal(decode::<HashMap<String, HashSet<u32>>>(value, &access, 0)?);
    Ok(())
}
/// Restore wasm32-sized class slots, widening only after complete decoding.
#[napi]
pub fn import_class_map<'env>(
    env: Env,
    value: Unknown<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<()> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    let map: HashMap<String, HashMap<String, Wasm32Index>> = decode(value, &access, 0)?;
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
pub fn import_file_map<'env>(
    env: Env,
    value: Unknown<'env>,
    callback: AccessCallback<'env>,
    schema_token: Unknown<'env>,
    access_token: Unknown<'env>,
) -> Result<()> {
    let access = Access {
        env,
        callback,
        schema_token,
        failure_token: access_token,
    };
    let map: bimap::BiHashMap<String, Wasm32Index> = decode(value, &access, 0)?;
    core::import_file_map_internal(
        map.into_iter()
            .map(|(file, index)| (file, index.0))
            .collect(),
    );
    Ok(())
}
#[doc = "Return the unchanged core serialization, without normalization."]
#[napi]
pub fn export_sheet() -> Result<String> {
    core::export_sheet_internal().map_err(Error::from_reason)
}
#[doc = "Return the unchanged core serialization, without normalization."]
#[napi]
pub fn export_class_map() -> Result<String> {
    core::export_class_map_internal().map_err(Error::from_reason)
}
#[doc = "Return the unchanged core serialization, without normalization."]
#[napi]
pub fn export_file_map() -> Result<String> {
    core::export_file_map_internal().map_err(Error::from_reason)
}
#[doc = "Return the unchanged core serialization, without normalization."]
#[napi]
pub fn export_canonical_map() -> Result<String> {
    core::export_canonical_map_internal().map_err(Error::from_reason)
}
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
