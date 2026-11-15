use wasm_bindgen::prelude::*;

use crate::{
    configuration::{
        css_internal, export_canonical_map_internal, export_class_map_internal,
        export_file_map_internal, export_sheet_internal, import_canonical_map_internal,
        import_file_routes_internal, register_shorthands_internal, register_theme_internal,
    },
    driver::DRIVER,
    sheet_checkpoint::import_sheet_checkpoint_internal,
    with_style_sheet,
};

#[wasm_bindgen(js_name = "importSheet")]
#[cfg(not(tarpaulin_include))]
pub fn import_sheet(sheet_object: JsValue) -> Result<(), JsValue> {
    import_sheet_checkpoint_internal(
        serde_wasm_bindgen::from_value(sheet_object).map_err(|e| DRIVER.errors.message(e))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "exportSheet")]
#[cfg(not(tarpaulin_include))]
pub fn export_sheet() -> Result<String, JsValue> {
    export_sheet_internal().map_err(|e| DRIVER.errors.message(e))
}

#[wasm_bindgen(js_name = "importClassMap")]
#[cfg(not(tarpaulin_include))]
pub fn import_class_map(sheet_object: JsValue) -> Result<(), JsValue> {
    crate::import_class_map_internal(
        serde_wasm_bindgen::from_value(sheet_object).map_err(|e| DRIVER.errors.message(e))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "exportClassMap")]
#[cfg(not(tarpaulin_include))]
pub fn export_class_map() -> Result<String, JsValue> {
    export_class_map_internal().map_err(|e| DRIVER.errors.message(e))
}

#[wasm_bindgen(js_name = "importFileMap")]
#[cfg(not(tarpaulin_include))]
pub fn import_file_map(sheet_object: JsValue) -> Result<(), JsValue> {
    crate::import_file_map_internal(
        serde_wasm_bindgen::from_value(sheet_object).map_err(|e| DRIVER.errors.message(e))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "exportFileMap")]
#[cfg(not(tarpaulin_include))]
pub fn export_file_map() -> Result<String, JsValue> {
    export_file_map_internal().map_err(|e| DRIVER.errors.message(e))
}

#[wasm_bindgen(js_name = "importCanonicalMap")]
#[cfg(not(tarpaulin_include))]
pub fn import_canonical_map(map_object: JsValue) -> Result<(), JsValue> {
    import_canonical_map_internal(
        serde_wasm_bindgen::from_value(map_object).map_err(|e| DRIVER.errors.message(e))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "exportCanonicalMap")]
#[cfg(not(tarpaulin_include))]
pub fn export_canonical_map() -> Result<String, JsValue> {
    export_canonical_map_internal().map_err(|e| DRIVER.errors.message(e))
}

/// Import the file -> set-of-route-ids mapping used to decide atom hoisting.
///
/// Accepts a JS object like `{ "src/page.tsx": [0, 3], "src/card.tsx": [0] }`
/// where each value is the set of leaf-route ids whose render closure includes
/// that file. Populated by the build-time pre-pass.
#[wasm_bindgen(js_name = "importFileRoutes")]
#[cfg(not(tarpaulin_include))]
pub fn import_file_routes(map_object: JsValue) -> Result<(), JsValue> {
    import_file_routes_internal(
        serde_wasm_bindgen::from_value(map_object).map_err(|e| DRIVER.errors.message(e))?,
    );
    Ok(())
}

/// Set how the imports of the files being extracted are resolved.
///
/// `(specifier, importer) => ({ path, code })`, or nothing when they cannot be.
/// Imported stylesheets are then evaluated, and imported constants inlined.
#[wasm_bindgen(js_name = "setModuleResolver")]
#[cfg(not(tarpaulin_include))]
pub fn set_module_resolver(resolver: Option<js_sys::Function>) {
    DRIVER.install(resolver);
}

#[wasm_bindgen(js_name = "registerTheme")]
#[cfg(not(tarpaulin_include))]
pub fn register_theme(theme_object: JsValue) -> Result<(), JsValue> {
    register_theme_internal(
        serde_wasm_bindgen::from_value(theme_object).map_err(|e| DRIVER.errors.message(e))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "registerShorthands")]
#[cfg(not(tarpaulin_include))]
pub fn register_shorthands(shorthands: JsValue) -> Result<(), JsValue> {
    register_shorthands_internal(
        serde_wasm_bindgen::from_value(shorthands).map_err(|e| DRIVER.errors.message(e))?,
    );
    Ok(())
}

#[wasm_bindgen(js_name = "getDefaultTheme")]
#[cfg(not(tarpaulin_include))]
pub fn get_default_theme() -> Result<Option<String>, JsValue> {
    Ok(with_style_sheet(|sheet| sheet.theme.get_default_theme()))
}

#[wasm_bindgen(js_name = "getCss")]
#[cfg(not(tarpaulin_include))]
pub fn get_css(file_num: Option<usize>, import_main_css: bool) -> Result<String, JsValue> {
    Ok(css_internal(file_num, import_main_css))
}

#[wasm_bindgen(js_name = "getThemeInterface")]
#[cfg(not(tarpaulin_include))]
pub fn get_theme_interface(
    package_name: &str,
    color_interface_name: &str,
    typography_interface_name: &str,
    length_interface_name: &str,
    shadows_interface_name: &str,
    theme_interface_name: &str,
) -> String {
    with_style_sheet(|sheet| {
        sheet.create_interface(
            package_name,
            color_interface_name,
            typography_interface_name,
            length_interface_name,
            shadows_interface_name,
            theme_interface_name,
        )
    })
}
