use extractor::ExtractOption;
use wasm_bindgen::prelude::*;

use crate::{
    Output, SourceMapMode,
    driver::{Arguments, DRIVER, Source},
};

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen(js_name = "codeExtract", skip_typescript)]
#[allow(clippy::too_many_arguments)]
pub fn code_extract(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: JsValue,
    source_type: JsValue,
    opaque_mdx_defaults: JsValue,
) -> Result<Output, JsValue> {
    DRIVER.extract(
        Source {
            filename,
            code,
            option: ExtractOption {
                package: package.into(),
                css_dir,
                single_css,
                import_main_css: import_main_css_in_code,
                import_aliases: std::collections::HashMap::new(),
            },
            import_main_css_in_css,
            map_mode: SourceMapMode::Generate,
        },
        Arguments {
            aliases: import_aliases,
            source_type,
            opaque_defaults: opaque_mdx_defaults,
        },
    )
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen(js_name = "codeExtractWithoutSourceMap", skip_typescript)]
#[allow(clippy::too_many_arguments)]
pub fn code_extract_without_source_map(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: JsValue,
    source_type: JsValue,
    opaque_mdx_defaults: JsValue,
) -> Result<Output, JsValue> {
    DRIVER.extract(
        Source {
            filename,
            code,
            option: ExtractOption {
                package: package.into(),
                css_dir,
                single_css,
                import_main_css: import_main_css_in_code,
                import_aliases: std::collections::HashMap::new(),
            },
            import_main_css_in_css,
            map_mode: SourceMapMode::Skip,
        },
        Arguments {
            aliases: import_aliases,
            source_type,
            opaque_defaults: opaque_mdx_defaults,
        },
    )
}

#[cfg(not(tarpaulin_include))]
#[wasm_bindgen(js_name = "hasDevupUI")]
#[must_use]
pub fn has_devup_ui_wasm(
    filename: &str,
    code: &str,
    package: &str,
    import_aliases: JsValue,
) -> bool {
    DRIVER.relevant(filename, code, package, import_aliases)
}

#[wasm_bindgen(typescript_custom_section)]
const EXTRACTION_TYPES: &str = r#"
export function codeExtract(filename: string, code: string, package: string, css_dir: string, single_css: boolean, import_main_css_in_code: boolean, import_main_css_in_css: boolean, import_aliases: Record<string, string | null>, sourceType?: 'compiled-mdx', opaqueMdxDefaults?: readonly string[]): Output;
export function codeExtractWithoutSourceMap(filename: string, code: string, package: string, css_dir: string, single_css: boolean, import_main_css_in_code: boolean, import_main_css_in_css: boolean, import_aliases: Record<string, string | null>, sourceType?: 'compiled-mdx', opaqueMdxDefaults?: readonly string[]): Output;
"#;
