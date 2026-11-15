#[cfg(not(tarpaulin_include))]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = _ownerImporter, skip_typescript)]
pub fn owner_importer() -> Option<String> {
    super::importer()
}
