//! Rule-free reservation through the real extractor and sheet allocation path.

#[cfg(test)]
use css::file_map;
use extractor::{ExtractOption, ModuleResolver};
use wasm_bindgen::prelude::*;

#[cfg(test)]
use crate::with_style_sheet_mut;
pub use devup_ui_binding_core::{ReservationInput, reserve_class_slots_internal};
mod rows;
#[cfg(not(tarpaulin_include))]
use crate::{MODULE_RESOLVER, boundary, call_module_resolver, import_aliases_from_js};

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
    let rows: Vec<Vec<String>> = boundary::decode(rows, 0)?;
    let inputs = rows::parse_rows(rows).map_err(|_| boundary::fail(0))?;
    let option = ExtractOption {
        package: package.to_string(),
        single_css,
        import_main_css: import_main_css_in_code,
        import_aliases: import_aliases_from_js(import_aliases, 5)?,
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
    devup_ui_binding_core::get_unreserved_class_slot_count()
}

#[cfg(test)]
mod file_slots_tests;
#[cfg(test)]
mod tests;
