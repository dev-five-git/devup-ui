use css::file_map::{canonical, is_global};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use rustc_hash::FxHashSet;
use wasm_bindgen::prelude::*;

use crate::with_style_sheet_mut;

#[wasm_bindgen]
pub struct Output {
    pub(crate) code: String,
    pub(crate) map: Option<String>,
    pub(crate) css_file: Option<String>,
    pub(crate) updated_base_style: bool,
    pub(crate) css: Option<String>,
    pub(crate) dependencies: Vec<String>,
    pub(crate) deferred_defaults: String,
}

pub(crate) fn deferred_defaults(output: &extractor::ExtractOutput) -> String {
    serde_json::Value::Array(output.deferred_defaults.iter().map(|record| serde_json::json!({
        "extraction": record.extraction, "request": record.request, "exports": record.exports,
        "line": record.line, "column": record.column,
        "demand": match record.demand { extractor::RenderDemand::RenderOnly => "render-only" },
        "attributes": record.attributes,
        "cutImporter": record.cut_importer, "cutRequest": record.cut_request, "cutExport": record.cut_export,
    })).collect()).to_string()
}

#[wasm_bindgen]
impl Output {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        code: String,
        styles: FxHashSet<ExtractStyleValue>,
        map: Option<String>,
        single_css: bool,
        filename: String,
        css_file: Option<String>,
        import_main_css: bool,
        dependencies: Vec<String>,
    ) -> Self {
        // Bucket identity matches classes already baked into code; globalCss remains per source.
        let _atom_mode = css::atom_hoist::AtomModeGuard::new(single_css);
        let canonical_filename = canonical(&filename);
        let global = single_css || is_global(&filename);
        with_style_sheet_mut(|sheet| {
            let default_collected = sheet.rm_global_css(&filename, global);
            let (collected, updated_base_style) =
                sheet.update_styles_in_bucket(&styles, &canonical_filename, global);
            Self {
                code,
                map,
                css_file,
                dependencies,
                deferred_defaults: "[]".to_string(),
                updated_base_style: updated_base_style || default_collected,
                css: if !collected && !default_collected {
                    None
                } else {
                    Some(sheet.create_css(
                        if global {
                            None
                        } else {
                            Some(&canonical_filename)
                        },
                        import_main_css,
                    ))
                },
            }
        })
    }

    /// Get the code
    #[wasm_bindgen(getter, js_name = "code")]
    #[must_use]
    pub fn code(&self) -> String {
        self.code.clone()
    }

    #[wasm_bindgen(getter, js_name = "cssFile")]
    #[must_use]
    pub fn css_file(&self) -> Option<String> {
        self.css_file.clone()
    }

    #[wasm_bindgen(getter, js_name = "map")]
    #[must_use]
    pub fn map(&self) -> Option<String> {
        self.map.clone()
    }

    #[wasm_bindgen(getter, js_name = "updatedBaseStyle")]
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn updated_base_style(&self) -> bool {
        self.updated_base_style
    }

    /// Get the css
    #[wasm_bindgen(getter, js_name = "css")]
    #[must_use]
    pub fn css(&self) -> Option<String> {
        self.css.clone()
    }

    /// Files read through the module resolver, for the bundler to watch
    #[wasm_bindgen(getter, js_name = "dependencies")]
    #[must_use]
    pub fn dependencies(&self) -> Vec<String> {
        self.dependencies.clone()
    }

    /// Serialized semantic obligations; acceptance belongs to the native verifier.
    #[wasm_bindgen(getter, js_name = "deferredDefaults")]
    #[must_use]
    pub fn deferred_defaults(&self) -> String {
        self.deferred_defaults.clone()
    }
}
