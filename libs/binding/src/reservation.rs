//! Rule-free reservation through the real extractor and sheet allocation path.

use bimap::BiHashMap;
use css::{file_map, slot_reservation::ScratchSlots};
use extractor::reservation_state::ScratchExtractionState;
use extractor::{ExtractOption, ModuleResolver, extract_with_modules, extract_without_source_map};
use sheet::StyleSheet;

use crate::with_style_sheet_mut;

/// Prepared physical input, keyed exactly as its later real transform.
pub struct ReservationInput {
    /// Physical source path.
    pub filename: String,
    /// Source contents.
    pub code: String,
    /// CSS output directory for this source.
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
