use std::collections::{BTreeMap, HashMap};

use css::{
    class_map::with_class_map,
    file_map::{set_canonical_map, with_canonical_map, with_file_map},
};
use sheet::StyleSheet;
use wasm_bindgen::prelude::*;

use crate::configuration_state::{Field, applied, replaced};
use crate::{MODULE_RESOLVER, with_style_sheet, with_style_sheet_mut};

#[wasm_bindgen(js_name = "setDebug")]
pub fn set_debug(debug: bool) {
    if applied(Field::Debug, css::debug::is_debug() != debug) {
        css::debug::set_debug(debug);
    }
}

#[wasm_bindgen(js_name = "isDebug")]
#[must_use]
pub fn is_debug() -> bool {
    css::debug::is_debug()
}

/// Set the CSS class name prefix
///
/// # Example (Vite Config)
/// ```javascript
/// import init, { setPrefix, codeExtract } from 'devup-ui-wasm';
///
/// export default {
///   plugins: [
///     {
///       name: 'devup-ui',
///       apply: 'pre',
///       async configResolved() {
///         await init();
///         setPrefix('du-'); // Set prefix to 'du-'
///       },
///       // ... other plugin code
///     }
///   ]
/// }
/// ```
///
/// # Example (Next.js Plugin)
/// ```typescript
/// import init, { setPrefix } from 'devup-ui-wasm';
///
/// const withDevupUI = (nextConfig) => {
///   return {
///     ...nextConfig,
///     webpack: (config, options) => {
///       if (!options.isServer && !global.devupUIInitialized) {
///         init().then(() => {
///           setPrefix('du-');
///           global.devupUIInitialized = true;
///         });
///       }
///       return config;
///     }
///   };
/// };
/// ```
#[wasm_bindgen(js_name = "setPrefix")]
pub fn set_prefix(prefix: Option<String>) {
    if applied(Field::Prefix, css::get_prefix() != prefix) {
        css::set_prefix(prefix);
    }
}

#[wasm_bindgen(js_name = "getPrefix")]
#[must_use]
pub fn get_prefix() -> Option<String> {
    css::get_prefix()
}

pub fn import_sheet_internal(sheet: StyleSheet) {
    crate::sheet_checkpoint::import_sheet_checkpoint_internal(sheet.into());
}

pub fn export_sheet_internal() -> Result<String, String> {
    with_style_sheet(crate::sheet_checkpoint::export).map_err(|e| e.to_string())
}

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

pub fn export_file_map_internal() -> Result<String, String> {
    with_file_map(|map| {
        let sorted: BTreeMap<&String, &usize> = map.iter().collect();
        serde_json::to_string(&sorted)
    })
    .map_err(|e| e.to_string())
}

pub fn import_canonical_map_internal(map: HashMap<String, String>) {
    if applied(
        Field::Canonical,
        with_canonical_map(|current| current != &map),
    ) {
        set_canonical_map(map);
    }
}

pub fn export_canonical_map_internal() -> Result<String, String> {
    with_canonical_map(|map| {
        let sorted: BTreeMap<&String, &String> = map.iter().collect();
        serde_json::to_string(&sorted)
    })
    .map_err(|e| e.to_string())
}

/// Number every file in `files` now, in path order, keeping numbers files
/// already hold, so class prefixes depend on the paths and not on the order
/// workers reach files in.
#[wasm_bindgen(js_name = "seedFileMap")]
pub fn seed_file_map(files: Vec<String>) {
    css::file_map::seed_file_numbers(&files);
}

pub fn reset_build_state_internal() {
    replaced();
    css::slot_reservation::reset();
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::file_routes::set_file_routes(HashMap::new());
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    css::set_prefix(None);
    with_style_sheet_mut(|sheet| *sheet = StyleSheet::default());
    MODULE_RESOLVER.with_borrow_mut(|current| *current = None);
}

#[wasm_bindgen(js_name = "resetBuildState")]
pub fn reset_build_state() {
    reset_build_state_internal();
}

/// Set the atom-level hoist threshold.
///
/// When set to `Some(n)`, buckets with predeclared reach of `>= n` routes emit
/// their atoms into shared CSS. Eligibility freezes before the first source;
/// unknown or late reach stays local until `resetBuildState`.
/// `None` (the default) preserves ordinary per-file naming and placement.
///
/// Set this before `codeExtract` so extraction and emission share one plan.
/// Pair with `importFileRoutes` to provide the file -> routes mapping.
#[wasm_bindgen(js_name = "setAtomHoist")]
pub fn set_atom_hoist(threshold: Option<usize>) {
    let threshold = threshold.filter(|value| *value != 0);
    if applied(
        Field::Threshold,
        css::atom_hoist::atom_hoist_threshold() != threshold,
    ) {
        css::atom_hoist::set_atom_hoist(threshold);
    }
}

pub fn import_file_routes_internal(map: HashMap<String, std::collections::HashSet<u32>>) {
    if applied(
        Field::Routes,
        css::file_routes::with_file_routes(|current| current != &map),
    ) {
        css::file_routes::set_file_routes(map);
    }
}

pub fn register_theme_internal(theme: sheet::theme::Theme) {
    if applied(Field::Theme, with_style_sheet(|sheet| sheet.theme != theme)) {
        with_style_sheet_mut(|sheet| sheet.set_theme(theme));
    }
}

pub fn register_shorthands_internal(shorthands: BTreeMap<String, Vec<String>>) {
    applied(
        Field::Shorthands,
        css::replace_custom_shorthands(shorthands),
    );
}

pub fn import_class_map_internal(map: HashMap<String, HashMap<String, usize>>) {
    replaced();
    css::class_map::set_class_map(map);
}

pub fn import_file_map_internal(map: bimap::BiHashMap<String, usize>) {
    replaced();
    css::file_map::set_file_map(map);
}

pub(crate) fn css_internal(file_num: Option<usize>, import_main_css: bool) -> String {
    with_style_sheet(|sheet| match file_num {
        Some(file_num) => with_file_map(|map| {
            sheet.create_css(
                map.get_by_right(&file_num).map(String::as_str),
                import_main_css,
            )
        }),
        None => sheet.create_css(None, import_main_css),
    })
}
