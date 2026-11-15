use std::collections::HashMap;

use bimap::BiHashMap;
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

/// Import the matching sheet export, including its global atom plan.
///
/// # Errors
/// Returns the serde parse/type error without changing the installed sheet or plan.
pub fn import_sheet_json_internal(json: &str) -> Result<(), serde_json::Error> {
    crate::sheet_checkpoint::import_sheet_checkpoint_internal(serde_json::from_str(json)?);
    Ok(())
}

/// Import the matching shared allocator export without a JavaScript object traversal.
///
/// # Errors
/// Returns the serde parse/type error without changing the installed class map.
pub fn import_class_map_json_internal(json: &str) -> Result<(), serde_json::Error> {
    let classes: HashMap<String, HashMap<String, usize>> = serde_json::from_str(json)?;
    crate::import_class_map_internal(classes);
    Ok(())
}

/// Import the matching file-number export without a JavaScript object traversal.
///
/// # Errors
/// Returns the serde parse/type error without changing the installed file map.
pub fn import_file_map_json_internal(json: &str) -> Result<(), serde_json::Error> {
    let files: BiHashMap<String, usize> = serde_json::from_str(json)?;
    crate::import_file_map_internal(files);
    Ok(())
}

/// Import the matching canonical-bucket export without a JavaScript object traversal.
///
/// # Errors
/// Returns the serde parse/type error without changing the installed canonical map.
pub fn import_canonical_map_json_internal(json: &str) -> Result<(), serde_json::Error> {
    let canonical: HashMap<String, String> = serde_json::from_str(json)?;
    crate::configuration_state::replaced();
    crate::import_canonical_map_internal(canonical);
    Ok(())
}

/// Import matching opaque reservation bookkeeping after its allocator maps.
///
/// # Errors
/// Returns the serde parse/type error without changing reservation bookkeeping.
pub fn import_reservation_state_json_internal(json: &str) -> Result<(), serde_json::Error> {
    let state: css::slot_reservation::Snapshot = serde_json::from_str(json)?;
    crate::reservation::import_reservation_state_internal(state);
    Ok(())
}

/// Decode a matching Rust sheet export at the WASM platform boundary.
///
/// # Errors
/// Returns a JavaScript Error for malformed JSON or an incompatible sheet type.
#[cfg(not(tarpaulin_include))]
#[wasm_bindgen(js_name = importSheetJson)]
pub fn import_sheet_json(json: &str) -> Result<(), JsValue> {
    import_sheet_json_internal(json)
        .map_err(|error| crate::driver::message_error(error, js_sys::Error::new))
}

/// Decode a matching Rust class-map export at the WASM platform boundary.
///
/// # Errors
/// Returns a JavaScript Error for malformed JSON or an incompatible class-map type.
#[cfg(not(tarpaulin_include))]
#[wasm_bindgen(js_name = importClassMapJson)]
pub fn import_class_map_json(json: &str) -> Result<(), JsValue> {
    import_class_map_json_internal(json)
        .map_err(|error| crate::driver::message_error(error, js_sys::Error::new))
}

/// Decode a matching Rust file-map export at the WASM platform boundary.
///
/// # Errors
/// Returns a JavaScript Error for malformed JSON or an incompatible file-map type.
#[cfg(not(tarpaulin_include))]
#[wasm_bindgen(js_name = importFileMapJson)]
pub fn import_file_map_json(json: &str) -> Result<(), JsValue> {
    import_file_map_json_internal(json)
        .map_err(|error| crate::driver::message_error(error, js_sys::Error::new))
}

/// Decode a matching Rust canonical-map export at the WASM platform boundary.
///
/// # Errors
/// Returns a JavaScript Error for malformed JSON or an incompatible canonical-map type.
#[cfg(not(tarpaulin_include))]
#[wasm_bindgen(js_name = importCanonicalMapJson)]
pub fn import_canonical_map_json(json: &str) -> Result<(), JsValue> {
    import_canonical_map_json_internal(json)
        .map_err(|error| crate::driver::message_error(error, js_sys::Error::new))
}

/// Decode a matching Rust reservation export at the WASM platform boundary.
///
/// # Errors
/// Returns a JavaScript Error for malformed JSON or incompatible reservation bookkeeping.
#[cfg(not(tarpaulin_include))]
#[wasm_bindgen(js_name = importReservationStateJson)]
pub fn import_reservation_state_json(json: &str) -> Result<(), JsValue> {
    import_reservation_state_json_internal(json)
        .map_err(|error| crate::driver::message_error(error, js_sys::Error::new))
}
