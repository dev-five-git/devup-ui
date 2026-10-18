//! Rule-free reservation through the real extractor and sheet allocation path.

use bimap::BiHashMap;
use css::{file_map, slot_reservation::ScratchSlots};
use extractor::reservation_state::ScratchExtractionState;
use extractor::{ExtractOption, ModuleResolver, extract_with_modules, extract_without_source_map};
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

/// Reserve `[filename, code, cssDir]` rows using the current resolver and configuration.
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
    reserve_class_slots_internal(
        &inputs,
        option,
        resolver
            .as_ref()
            .map(|resolver| resolver as &ModuleResolver),
    );
    Ok(())
}

/// New live class and file slots since this build's first reservation; repeats do not count.
#[must_use]
#[wasm_bindgen(js_name = "getUnreservedClassSlotCount")]
pub fn get_unreserved_class_slot_count() -> usize {
    css::slot_reservation::unreserved_slot_count()
}

#[cfg(test)]
mod file_slots_tests;
#[cfg(test)]
mod tests;
