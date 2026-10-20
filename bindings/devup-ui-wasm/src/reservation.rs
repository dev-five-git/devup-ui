//! Rule-free reservation through the real extractor and sheet allocation path.

use bimap::BiHashMap;
use css::{
    file_map,
    slot_reservation::{RowSlots, ScratchSlots},
};
use extractor::reservation_state::ScratchExtractionState;
use extractor::{ExtractOption, ExtractSourceType, ModuleResolver, extract_with_source_type};
use sheet::StyleSheet;
use wasm_bindgen::prelude::*;

use crate::with_style_sheet_mut;
mod rows;
#[cfg(not(tarpaulin_include))]
use crate::{MODULE_RESOLVER, call_module_resolver, import_aliases_from_js, js_error};

/// Prepared physical input, keyed exactly as its later real transform.
pub struct ReservationInput {
    pub filename: String,
    pub code: String,
    pub css_dir: String,
    pub source_type: Option<ExtractSourceType>,
}

struct ScratchSheet {
    live: StyleSheet,
    files: BiHashMap<String, usize>,
}

impl Default for ScratchSheet {
    fn default() -> Self {
        let files = file_map::get_file_map();
        let live = with_style_sheet_mut(|sheet| {
            let mut live = std::mem::take(sheet);
            sheet.theme = std::mem::take(&mut live.theme);
            live
        });
        Self { live, files }
    }
}

impl Drop for ScratchSheet {
    fn drop(&mut self) {
        with_style_sheet_mut(|sheet| {
            std::mem::swap(&mut self.live.theme, &mut sheet.theme);
            *sheet = std::mem::take(&mut self.live);
        });
        file_map::set_file_map(std::mem::take(&mut self.files));
    }
}

/// Reserve in supplied physical-path order; suppress per-input errors, emitting nothing.
pub fn reserve_class_slots_internal(
    inputs: &[ReservationInput],
    option: ExtractOption,
    resolver: Option<&ModuleResolver>,
) {
    reserve_class_slots_with(inputs, option, |input, option| {
        extract_with_source_type(
            &input.filename,
            &input.code,
            option,
            false,
            resolver,
            input.source_type,
        )
    });
}

fn reserve_class_slots_with<E>(
    inputs: &[ReservationInput],
    option: ExtractOption,
    mut extract_row: impl FnMut(&ReservationInput, ExtractOption) -> Result<extractor::ExtractOutput, E>,
) {
    let _atom_mode = css::atom_hoist::AtomModeGuard::new(option.single_css);
    let slots = ScratchSlots::default();
    let files: Vec<String> = inputs.iter().map(|input| input.filename.clone()).collect();
    file_map::seed_file_numbers(&files);
    for input in inputs {
        let _ = file_map::get_file_num_by_filename(&input.filename);
    }
    let _sheet = ScratchSheet::default();
    let extraction = ScratchExtractionState::default();
    for input in inputs {
        let file_option = ExtractOption {
            css_dir: input.css_dir.clone(),
            ..option.clone()
        };
        let row = RowSlots::default();
        if let Ok(output) = extract_row(input, file_option) {
            let bucket = file_map::canonical(&input.filename);
            let global = option.single_css || file_map::is_global(&input.filename);
            with_style_sheet_mut(|sheet| {
                sheet.rm_global_css(&input.filename, global);
                sheet.update_styles_in_bucket(&output.styles, &bucket, global);
            });
            row.commit();
        } else {
            drop(row);
            extraction.discard_failed_row();
            css::slot_reservation::record_suppressed_row();
        }
    }
    slots.commit();
}

type FallibleResolver<'a> =
    dyn Fn(&str, &str) -> Result<Option<extractor::ModuleResolution>, String> + 'a;

/// Consume the first host fault before the common driver can accept a row's output.
fn extract_scratch_row(
    input: &ReservationInput,
    option: ExtractOption,
    resolve: &FallibleResolver<'_>,
) -> Result<extractor::ExtractOutput, String> {
    let fault = std::cell::RefCell::new(None);
    let resolver = |specifier: &str, importer: &str| {
        if fault.borrow().is_some() {
            return None;
        }
        match resolve(specifier, importer) {
            Ok(module) => module,
            Err(cause) => {
                *fault.borrow_mut() = Some(format!(
                    "{importer}:1:1: module resolver cannot use `{specifier}` at build time: {cause}"
                ));
                None
            }
        }
    };
    let extracted = extract_with_source_type(
        &input.filename,
        &input.code,
        option,
        false,
        Some(&resolver),
        input.source_type,
    );
    crate::checked_extraction(extracted, Some(&fault))
}

/// Reserve `[filename, code, cssDir, sourceType?]` rows under their real filenames.
#[wasm_bindgen(js_name = "reserveClassSlots")]
#[cfg(not(tarpaulin_include))]
pub fn reserve_class_slots(
    rows: JsValue,
    package: &str,
    single_css: bool,
    import_main_css_in_code: bool,
    _import_main_css_in_css: bool,
    import_aliases: JsValue,
) -> Result<(), JsValue> {
    let rows: Vec<Vec<String>> = serde_wasm_bindgen::from_value(rows).map_err(js_error)?;
    let inputs = rows::parse_rows(rows).map_err(js_error)?;
    let option = ExtractOption {
        package: package.to_string(),
        single_css,
        import_main_css: import_main_css_in_code,
        import_aliases: import_aliases_from_js(import_aliases)?,
        ..ExtractOption::default()
    };
    let resolver = MODULE_RESOLVER.with_borrow(Clone::clone).map(|resolver| {
        move |specifier: &str, importer: &str| call_module_resolver(&resolver, specifier, importer)
    });
    match resolver {
        Some(resolver) => reserve_class_slots_with(&inputs, option, |input, option| {
            extract_scratch_row(input, option, &resolver)
        }),
        None => reserve_class_slots_internal(&inputs, option, None),
    }
    Ok(())
}

/// New live class and file slots since this build's first reservation; repeats do not count.
#[must_use]
#[wasm_bindgen(js_name = "getUnreservedClassSlotCount")]
pub fn get_unreserved_class_slot_count() -> usize {
    css::slot_reservation::unreserved_slot_count()
}

/// Failed rows suppressed by the current reservation batch, separate from live misses.
#[must_use]
#[wasm_bindgen(js_name = "getSuppressedReservationRowCount")]
pub fn get_suppressed_reservation_row_count() -> usize {
    css::slot_reservation::suppressed_row_count()
}

/// Serialize opaque reservation bookkeeping for a synchronous build owner handoff.
pub fn export_reservation_state_internal() -> Result<String, String> {
    serde_json::to_string(&css::slot_reservation::capture()).map_err(|error| error.to_string())
}

/// Restore already parsed reservation bookkeeping without altering allocator maps.
pub fn import_reservation_state_internal(state: css::slot_reservation::Snapshot) {
    css::slot_reservation::restore(state);
}

/// Capture reservation bookkeeping together with the existing sheet and map snapshots.
#[wasm_bindgen(js_name = "exportReservationState")]
#[cfg(not(tarpaulin_include))]
pub fn export_reservation_state() -> Result<String, JsValue> {
    export_reservation_state_internal().map_err(js_error)
}

/// Restore an owner's opaque reservation bookkeeping after resetting the engine.
#[wasm_bindgen(js_name = "importReservationState")]
#[cfg(not(tarpaulin_include))]
pub fn import_reservation_state(state: JsValue) -> Result<(), JsValue> {
    let state = serde_wasm_bindgen::from_value(state).map_err(js_error)?;
    import_reservation_state_internal(state);
    Ok(())
}

#[cfg(test)]
mod cache_rollback_tests;
#[cfg(test)]
mod file_slots_tests;
#[cfg(test)]
mod rollback_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn serialized_owner_restore_retains_phase_and_count() -> Result<(), String> {
        crate::reset_build_state_internal();
        reserve_class_slots_internal(&[], ExtractOption::default(), None);
        let _ = file_map::get_file_num_by_filename("first-live.tsx");
        let saved = export_reservation_state_internal()?;
        crate::reset_build_state_internal();
        let snapshot = serde_json::from_str(&saved).map_err(|error| error.to_string())?;
        import_reservation_state_internal(snapshot);
        let _ = file_map::get_file_num_by_filename("second-live.tsx");
        assert_eq!(get_unreserved_class_slot_count(), 2);
        crate::reset_build_state_internal();
        Ok(())
    }
}
