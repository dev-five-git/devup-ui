use std::collections::{BTreeMap, HashMap};

use css::class_map::{set_class_map, with_class_map};
use css::file_map::{set_canonical_map, set_file_map, with_canonical_map, with_file_map};
use napi::Result;
use napi_derive::napi;
use serde_json::Value;
use sheet::StyleSheet;

use crate::{js_error, resolver, with_style_sheet, with_style_sheet_mut};

/// Restore the style sheet from a snapshot value.
///
/// # Errors
/// Returns an error if the value is not a valid sheet snapshot.
#[napi(js_name = "importSheet")]
pub fn import_sheet(value: Value) -> Result<()> {
    let sheet: StyleSheet = serde_json::from_value(value).map_err(js_error)?;
    css::atom_hoist::restore_atom_plan(sheet.atom_plan.clone());
    with_style_sheet_mut(|global_sheet| *global_sheet = sheet);
    Ok(())
}

/// Serialize the style sheet snapshot.
///
/// # Errors
/// Returns an error if snapshot serialization fails.
#[napi(js_name = "exportSheet")]
pub fn export_sheet() -> Result<String> {
    with_style_sheet(|sheet| serde_json::to_string(&sheet.export_snapshot())).map_err(js_error)
}

/// Restore the class-name mapping.
///
/// # Errors
/// Returns an error if the value is not a valid class mapping.
#[napi(js_name = "importClassMap")]
pub fn import_class_map(value: Value) -> Result<()> {
    set_class_map(serde_json::from_value(value).map_err(js_error)?);
    Ok(())
}

/// Serialize the class-name mapping in sorted order.
///
/// # Errors
/// Returns an error if mapping serialization fails.
#[napi(js_name = "exportClassMap")]
pub fn export_class_map() -> Result<String> {
    with_class_map(|map| {
        let sorted: BTreeMap<&String, BTreeMap<&String, &usize>> = map
            .iter()
            .map(|(file, classes)| (file, classes.iter().collect()))
            .collect();
        serde_json::to_string(&sorted)
    })
    .map_err(js_error)
}

/// Restore the file-number mapping.
///
/// # Errors
/// Returns an error if the value is not a valid file mapping.
#[napi(js_name = "importFileMap")]
pub fn import_file_map(value: Value) -> Result<()> {
    set_file_map(serde_json::from_value(value).map_err(js_error)?);
    Ok(())
}

/// Serialize the file-number mapping in sorted order.
///
/// # Errors
/// Returns an error if mapping serialization fails.
#[napi(js_name = "exportFileMap")]
pub fn export_file_map() -> Result<String> {
    with_file_map(|map| {
        let sorted: BTreeMap<&String, &usize> = map.iter().collect();
        serde_json::to_string(&sorted)
    })
    .map_err(js_error)
}

/// Restore the canonical filename mapping.
///
/// # Errors
/// Returns an error if the value is not a valid canonical mapping.
#[napi(js_name = "importCanonicalMap")]
pub fn import_canonical_map(value: Value) -> Result<()> {
    set_canonical_map(serde_json::from_value(value).map_err(js_error)?);
    Ok(())
}

/// Serialize the canonical filename mapping in sorted order.
///
/// # Errors
/// Returns an error if mapping serialization fails.
#[napi(js_name = "exportCanonicalMap")]
pub fn export_canonical_map() -> Result<String> {
    with_canonical_map(|map| {
        let sorted: BTreeMap<&String, &String> = map.iter().collect();
        serde_json::to_string(&sorted)
    })
    .map_err(js_error)
}

/// Seed file numbers in sorted path order.
#[napi(js_name = "seedFileMap")]
pub fn seed_file_map(files: Vec<String>) {
    css::file_map::seed_file_numbers(&files);
}

/// Reset sheet, naming, reservation, route and resolver build state.
#[napi(js_name = "resetBuildState")]
pub fn reset_build_state() {
    css::slot_reservation::reset();
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::file_routes::set_file_routes(HashMap::new());
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    css::set_prefix(None);
    with_style_sheet_mut(|sheet| *sheet = StyleSheet::default());
    resolver::clear();
}
