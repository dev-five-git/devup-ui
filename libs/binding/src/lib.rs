//! Host-neutral binding orchestration; JavaScript values belong to the host adapter.

mod output;
mod reservation;

pub use css::atom_hoist::set_atom_hoist;
pub use css::debug::{is_debug, set_debug};
pub use css::file_routes::set_file_routes as import_file_routes_internal;
pub use css::slot_reservation::unreserved_slot_count as get_unreserved_class_slot_count;
pub use css::{get_prefix, set_custom_shorthands as register_shorthands_internal, set_prefix};
pub use extractor::{has_devup_ui, has_devup_ui_with};
pub use output::{CssOptions, ExtractionMode, Output, SourceMapMode, code_extract};
pub use reservation::{ReservationInput, reserve_class_slots_internal};

use css::class_map::{set_class_map, with_class_map};
use css::file_map::{set_canonical_map, set_file_map, with_canonical_map, with_file_map};
use sheet::StyleSheet;
use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, Mutex};

/// The single stylesheet shared by extraction, reservation and host test seams.
pub static GLOBAL_STYLE_SHEET: LazyLock<Mutex<StyleSheet>> =
    LazyLock::new(|| Mutex::new(StyleSheet::default()));

/// Borrow the live stylesheet, recovering its contents after an unwind.
pub fn with_style_sheet<F, R>(f: F) -> R
where
    F: FnOnce(&StyleSheet) -> R,
{
    let guard = GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&guard)
}

/// Mutably borrow the live stylesheet, recovering its contents after an unwind.
pub fn with_style_sheet_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut StyleSheet) -> R,
{
    let mut guard = GLOBAL_STYLE_SHEET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut guard)
}

/// Restore the sheet and its frozen atom plan together.
pub fn import_sheet_internal(sheet: StyleSheet) {
    css::atom_hoist::restore_atom_plan(sheet.atom_plan.clone());
    with_style_sheet_mut(|global_sheet| *global_sheet = sheet);
}

/// Export the deterministic sheet snapshot without changing live storage.
pub fn export_sheet_internal() -> Result<String, String> {
    with_style_sheet(|sheet| serde_json::to_string(&sheet.export_snapshot()))
        .map_err(|e| e.to_string())
}

/// Restore class allocations decoded by a host adapter.
pub fn import_class_map_internal(map: HashMap<String, HashMap<String, usize>>) {
    set_class_map(map);
}

/// Export class allocations in file and class-key order.
pub fn export_class_map_internal() -> Result<String, String> {
    with_class_map(|map| {
        let sorted: BTreeMap<&String, BTreeMap<&String, &usize>> = map
            .iter()
            .map(|(file, classes)| (file, classes.iter().collect()))
            .collect();
        serde_json::to_string(&sorted)
    })
    .map_err(|e| e.to_string())
}

/// Restore file numbers decoded by a host adapter.
pub fn import_file_map_internal(map: bimap::BiHashMap<String, usize>) {
    set_file_map(map);
}

/// Export file numbers in path order.
pub fn export_file_map_internal() -> Result<String, String> {
    with_file_map(|map| {
        let sorted: BTreeMap<&String, &usize> = map.iter().collect();
        serde_json::to_string(&sorted)
    })
    .map_err(|e| e.to_string())
}

/// Restore canonical bucket identities decoded by a host adapter.
pub fn import_canonical_map_internal(map: HashMap<String, String>) {
    set_canonical_map(map);
}

/// Export canonical bucket identities in path order.
pub fn export_canonical_map_internal() -> Result<String, String> {
    with_canonical_map(|map| {
        let sorted: BTreeMap<&String, &String> = map.iter().collect();
        serde_json::to_string(&sorted)
    })
    .map_err(|e| e.to_string())
}

/// Seed deterministic file numbers without replacing existing assignments.
pub fn seed_file_map(files: &[String]) {
    css::file_map::seed_file_numbers(files);
}

/// Clear production build state, retaining theme registries, shorthands and debug.
/// The host adapter separately clears its resolver handle.
pub fn reset_build_state_internal() {
    css::slot_reservation::reset();
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::file_routes::set_file_routes(HashMap::new());
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    css::set_prefix(None);
    with_style_sheet_mut(|sheet| *sheet = StyleSheet::default());
}

/// Clear test state at a quiescent boundary, clearing the host resolver before registries.
pub fn reset_state_internal(clear_resolver: impl FnOnce()) {
    with_style_sheet_mut(|sheet| *sheet = StyleSheet::default());
    clear_resolver();
    css::test_state::reset_state_for_testing();
    extractor::test_state::reset_caches_for_testing();
}

/// Register a decoded theme in the live stylesheet.
pub fn register_theme_internal(theme: sheet::theme::Theme) {
    with_style_sheet_mut(|sheet| sheet.set_theme(theme));
}

/// Return the theme used without an explicit variant selector.
pub fn get_default_theme() -> Option<String> {
    with_style_sheet(|sheet| sheet.theme.get_default_theme())
}

/// Materialize CSS for a numbered file or the global sheet.
pub fn get_css(file_num: Option<usize>, import_main_css: bool) -> String {
    with_style_sheet(|sheet| {
        if let Some(file_num) = file_num {
            with_file_map(|map| {
                sheet.create_css(
                    map.get_by_right(&file_num).map(String::as_str),
                    import_main_css,
                )
            })
        } else {
            sheet.create_css(None, import_main_css)
        }
    })
}

/// Generate declarations with color, typography, length, shadow and theme interface names.
pub fn get_theme_interface(package_name: &str, names: [&str; 5]) -> String {
    let [color, typography, length, shadows, theme] = names;
    with_style_sheet(|sheet| {
        sheet.create_interface(package_name, color, typography, length, shadows, theme)
    })
}
