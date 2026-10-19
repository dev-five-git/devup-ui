use std::collections::HashMap;

use extractor::{
    ExtractOption, ImportAlias, ModuleResolver, extract, extract_with_modules,
    extract_without_source_map,
};
use napi::{Env, Result};
use napi_derive::napi;
use serde_json::Value;

use crate::output::{Emission, Output};
use crate::{js_error, resolver};

pub(crate) fn import_aliases_from_js(value: Value) -> Result<HashMap<String, ImportAlias>> {
    let aliases: HashMap<String, Option<String>> =
        serde_json::from_value(value).map_err(js_error)?;
    Ok(aliases
        .into_iter()
        .map(|(key, value)| {
            let alias = match value {
                Some(name) => ImportAlias::DefaultToNamed(name),
                None => ImportAlias::NamedToNamed,
            };
            (key, alias)
        })
        .collect())
}

#[derive(Clone, Copy)]
enum SourceMapMode {
    Generate,
    Skip,
}

// The positional arguments intentionally reproduce the existing WASM ABI.
#[expect(clippy::too_many_arguments, reason = "Exact WASM extraction ABI")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Preserve the measured napi-owned String conversion through the extraction bridge"
)]
fn code_extract_js(
    env: Env,
    filename: String,
    code: String,
    package: String,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: Value,
    source_map: SourceMapMode,
) -> Result<Output> {
    let import_aliases = import_aliases_from_js(import_aliases)?;
    let resolver = resolver::snapshot(env);
    let option = ExtractOption {
        package,
        css_dir,
        single_css,
        import_main_css: import_main_css_in_code,
        import_aliases,
    };
    let extracted = match (
        resolver.as_ref().map(|value| value as &ModuleResolver),
        source_map,
    ) {
        (Some(resolver), mode) => extract_with_modules(
            &filename,
            &code,
            option,
            matches!(mode, SourceMapMode::Generate),
            resolver,
        ),
        (None, SourceMapMode::Generate) => extract(&filename, &code, option),
        (None, SourceMapMode::Skip) => extract_without_source_map(&filename, &code, option),
    };
    match extracted {
        Ok(output) => Ok(Output::new(
            output,
            &filename,
            Emission {
                single_css,
                import_main_css: import_main_css_in_css,
            },
        )),
        Err(error) => Err(js_error(error)),
    }
}

/// Extract styles and transformed code with a source map.
///
/// # Errors
/// Returns an error if aliases are malformed or extraction fails.
#[napi(js_name = "codeExtract")]
pub fn code_extract(
    env: Env,
    filename: String,
    code: String,
    package: String,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: Value,
) -> Result<Output> {
    code_extract_js(
        env,
        filename,
        code,
        package,
        css_dir,
        single_css,
        import_main_css_in_code,
        import_main_css_in_css,
        import_aliases,
        SourceMapMode::Generate,
    )
}

/// Extract styles and transformed code without a source map.
///
/// # Errors
/// Returns an error if aliases are malformed or extraction fails.
#[napi(js_name = "codeExtractWithoutSourceMap")]
pub fn code_extract_without_source_map(
    env: Env,
    filename: String,
    code: String,
    package: String,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: Value,
) -> Result<Output> {
    code_extract_js(
        env,
        filename,
        code,
        package,
        css_dir,
        single_css,
        import_main_css_in_code,
        import_main_css_in_css,
        import_aliases,
        SourceMapMode::Skip,
    )
}

/// Detect whether a source uses the configured Devup UI package.
#[must_use]
#[napi(js_name = "hasDevupUI")]
pub fn has_devup_ui(filename: String, code: String, package: String) -> bool {
    extractor::has_devup_ui(&filename, &code, &package)
}
