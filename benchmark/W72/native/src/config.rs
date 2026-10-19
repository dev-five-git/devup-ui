use css::file_map::with_file_map;
use napi::Result;
use napi::bindgen_prelude::{Either, Undefined};
use napi_derive::napi;
use serde_json::Value;

use crate::output::optional_string;
use crate::{js_error, with_style_sheet, with_style_sheet_mut};

/// Set extraction debug mode.
#[napi(js_name = "setDebug")]
pub fn set_debug(debug: bool) {
    css::debug::set_debug(debug);
}

/// Return whether extraction debug mode is enabled.
#[must_use]
#[napi(js_name = "isDebug")]
pub fn is_debug() -> bool {
    css::debug::is_debug()
}

/// Set or clear the generated class-name prefix.
#[napi(js_name = "setPrefix")]
pub fn set_prefix(prefix: Option<String>) {
    css::set_prefix(prefix);
}

/// Return the generated class-name prefix, or JavaScript undefined.
#[must_use]
#[napi(js_name = "getPrefix")]
pub fn get_prefix() -> Either<String, Undefined> {
    optional_string(css::get_prefix())
}

/// Set or clear the atom-hoisting threshold.
///
/// # Errors
/// Returns an error if the threshold does not fit the platform's index type.
#[napi(js_name = "setAtomHoist")]
pub fn set_atom_hoist(threshold: Option<u32>) -> Result<()> {
    let threshold = threshold
        .map(usize::try_from)
        .transpose()
        .map_err(js_error)?;
    css::atom_hoist::set_atom_hoist(threshold);
    Ok(())
}

/// Import the file-to-route mapping.
///
/// # Errors
/// Returns an error if the value is not a valid route mapping.
#[napi(js_name = "importFileRoutes")]
pub fn import_file_routes(value: Value) -> Result<()> {
    css::file_routes::set_file_routes(serde_json::from_value(value).map_err(js_error)?);
    Ok(())
}

/// Replace the sheet's theme.
///
/// # Errors
/// Returns an error if the value is not a valid theme.
#[napi(js_name = "registerTheme")]
pub fn register_theme(value: Value) -> Result<()> {
    let theme: sheet::theme::Theme = serde_json::from_value(value).map_err(js_error)?;
    with_style_sheet_mut(|sheet| sheet.set_theme(theme));
    Ok(())
}

/// Replace the custom shorthand mapping.
///
/// # Errors
/// Returns an error if the value is not a valid shorthand mapping.
#[napi(js_name = "registerShorthands")]
pub fn register_shorthands(value: Value) -> Result<()> {
    css::set_custom_shorthands(serde_json::from_value(value).map_err(js_error)?);
    Ok(())
}

/// Return the default theme name, or JavaScript undefined.
#[must_use]
#[napi(js_name = "getDefaultTheme")]
pub fn get_default_theme() -> Either<String, Undefined> {
    optional_string(with_style_sheet(|sheet| sheet.theme.get_default_theme()))
}

/// Generate CSS for a file number or the entire sheet.
///
/// # Errors
/// Returns an error if the file number does not fit the platform's index type.
#[napi(js_name = "getCss")]
pub fn get_css(file_num: Option<u32>, import_main_css: bool) -> Result<String> {
    let file_num = file_num
        .map(usize::try_from)
        .transpose()
        .map_err(js_error)?;
    Ok(with_style_sheet(|sheet| {
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
    }))
}

/// Generate theme declarations using the supplied interface names.
#[must_use]
#[napi(js_name = "getThemeInterface")]
pub fn get_theme_interface(
    package_name: String,
    color_interface_name: String,
    typography_interface_name: String,
    length_interface_name: String,
    shadows_interface_name: String,
    theme_interface_name: String,
) -> String {
    with_style_sheet(|sheet| {
        sheet.create_interface(
            &package_name,
            &color_interface_name,
            &typography_interface_name,
            &length_interface_name,
            &shadows_interface_name,
            &theme_interface_name,
        )
    })
}
