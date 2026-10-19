//! Reservation follows the WASM scratch-state guards and allocation order.

use bimap::BiHashMap;
use css::{file_map, slot_reservation::ScratchSlots};
use extractor::reservation_state::ScratchExtractionState;
use extractor::{ExtractOption, ModuleResolver, extract_with_modules, extract_without_source_map};
use napi::{Env, Result};
use napi_derive::napi;
use serde_json::Value;
use sheet::StyleSheet;

use crate::extraction::import_aliases_from_js;
use crate::{js_error, resolver, with_style_sheet_mut};

struct ReservationInput {
    filename: String,
    code: String,
    css_dir: String,
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

#[expect(
    clippy::needless_pass_by_value,
    reason = "Preserve ownership and drop timing of the measured reservation options"
)]
fn reserve_class_slots_internal(
    inputs: &[ReservationInput],
    option: ExtractOption,
    resolver: Option<&ModuleResolver>,
) {
    let _atom_mode = css::atom_hoist::AtomModeGuard::new(option.single_css);
    let slots = ScratchSlots::default();
    let files: Vec<String> = inputs.iter().map(|input| input.filename.clone()).collect();
    file_map::seed_file_numbers(&files);
    for input in inputs {
        let _ = file_map::get_file_num_by_filename(&input.filename);
    }
    let _sheet = ScratchSheet::default();
    let _extraction = ScratchExtractionState::default();
    for input in inputs {
        let file_option = ExtractOption {
            css_dir: input.css_dir.clone(),
            ..option.clone()
        };
        let extracted = match resolver {
            Some(resolver) => {
                extract_with_modules(&input.filename, &input.code, file_option, false, resolver)
            }
            None => extract_without_source_map(&input.filename, &input.code, file_option),
        };
        // Per-file extraction errors are intentionally suppressed by the WASM API.
        if let Ok(output) = extracted {
            let bucket = file_map::canonical(&input.filename);
            let global = option.single_css || file_map::is_global(&input.filename);
            with_style_sheet_mut(|sheet| {
                sheet.rm_global_css(&input.filename, global);
                sheet.update_styles_in_bucket(&output.styles, &bucket, global);
            });
        }
    }
    slots.commit();
}

/// Reserve class slots using scratch extraction state.
///
/// # Errors
/// Returns an error for malformed rows or aliases; per-file extraction errors are ignored.
#[napi(js_name = "reserveClassSlots")]
pub fn reserve_class_slots(
    env: Env,
    rows: Value,
    package: String,
    single_css: bool,
    import_main_css_in_code: bool,
    _import_main_css_in_css: bool,
    import_aliases: Value,
) -> Result<()> {
    let rows: Vec<Vec<String>> = serde_json::from_value(rows).map_err(js_error)?;
    let inputs: Vec<ReservationInput> = rows
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            let [filename, code, css_dir]: [String; 3] =
                row.try_into().map_err(|row: Vec<String>| {
                    js_error(format!(
                        "reservation row {index} must contain exactly 3 strings, got {} fields",
                        row.len(),
                    ))
                })?;
            Ok(ReservationInput {
                filename,
                code,
                css_dir,
            })
        })
        .collect::<Result<_>>()?;
    let option = ExtractOption {
        package,
        single_css,
        import_main_css: import_main_css_in_code,
        import_aliases: import_aliases_from_js(import_aliases)?,
        ..ExtractOption::default()
    };
    let resolver = resolver::snapshot(env);
    reserve_class_slots_internal(
        &inputs,
        option,
        resolver.as_ref().map(|value| value as &ModuleResolver),
    );
    Ok(())
}

/// Return the number of class slots allocated outside reservation.
///
/// # Errors
/// Returns an error if the count exceeds the JavaScript ABI's unsigned 32-bit range.
#[napi(js_name = "getUnreservedClassSlotCount")]
pub fn get_unreserved_class_slot_count() -> Result<u32> {
    u32::try_from(css::slot_reservation::unreserved_slot_count()).map_err(js_error)
}
