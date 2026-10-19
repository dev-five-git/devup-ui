use css::file_map::{canonical, is_global};
use extractor::ExtractOutput;
use napi::Result;
use napi::bindgen_prelude::{Either, Undefined};
use napi_derive::napi;

use crate::{js_error, with_style_sheet_mut};

/// The sheet-emission options, separate from the extractor's code-import option.
#[derive(Clone, Copy)]
pub(crate) struct Emission {
    pub single_css: bool,
    pub import_main_css: bool,
}

struct OutputData {
    code: String,
    map: Option<String>,
    css_file: Option<String>,
    updated_base_style: bool,
    css: Option<String>,
    dependencies: Vec<String>,
}

/// A WASM-compatible extraction result with explicit early release.
#[napi]
pub struct Output {
    data: Option<OutputData>,
}

impl Output {
    pub(crate) fn new(output: ExtractOutput, filename: &str, emission: Emission) -> Self {
        let _atom_mode = css::atom_hoist::AtomModeGuard::new(emission.single_css);
        let canonical_filename = canonical(filename);
        let global = emission.single_css || is_global(filename);
        with_style_sheet_mut(|sheet| {
            // Global CSS remains keyed by the raw source, not the collapsed bucket.
            let default_collected = sheet.rm_global_css(filename, global);
            let (collected, updated_base_style) =
                sheet.update_styles_in_bucket(&output.styles, &canonical_filename, global);
            Self {
                data: Some(OutputData {
                    code: output.code,
                    map: output.map,
                    css_file: output.css_file,
                    dependencies: output.dependencies,
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
                            emission.import_main_css,
                        ))
                    },
                }),
            }
        })
    }

    fn data(&self) -> Result<&OutputData> {
        self.data
            .as_ref()
            .ok_or_else(|| js_error("Output has been freed"))
    }
}

pub(crate) fn optional_string(value: Option<String>) -> Either<String, Undefined> {
    match value {
        Some(value) => Either::A(value),
        None => Either::B(()),
    }
}

#[napi]
impl Output {
    /// Release the retained strings without waiting for JavaScript GC.
    #[napi]
    pub fn free(&mut self) {
        self.data = None;
    }

    /// Return the transformed source code.
    ///
    /// # Errors
    /// Returns an error if this output has been freed.
    #[napi(getter)]
    pub fn code(&self) -> Result<String> {
        Ok(self.data()?.code.clone())
    }

    /// Return the CSS filename, or JavaScript undefined.
    ///
    /// # Errors
    /// Returns an error if this output has been freed.
    #[napi(getter, js_name = "cssFile")]
    pub fn css_file(&self) -> Result<Either<String, Undefined>> {
        Ok(optional_string(self.data()?.css_file.clone()))
    }

    /// Return the source map, or JavaScript undefined.
    ///
    /// # Errors
    /// Returns an error if this output has been freed.
    #[napi(getter)]
    pub fn map(&self) -> Result<Either<String, Undefined>> {
        Ok(optional_string(self.data()?.map.clone()))
    }

    /// Return whether extraction changed the base styles.
    ///
    /// # Errors
    /// Returns an error if this output has been freed.
    #[napi(getter, js_name = "updatedBaseStyle")]
    pub fn updated_base_style(&self) -> Result<bool> {
        Ok(self.data()?.updated_base_style)
    }

    /// Return emitted CSS, or JavaScript undefined.
    ///
    /// # Errors
    /// Returns an error if this output has been freed.
    #[napi(getter)]
    pub fn css(&self) -> Result<Either<String, Undefined>> {
        Ok(optional_string(self.data()?.css.clone()))
    }

    /// Return the resolved module dependencies.
    ///
    /// # Errors
    /// Returns an error if this output has been freed.
    #[napi(getter)]
    pub fn dependencies(&self) -> Result<Vec<String>> {
        Ok(self.data()?.dependencies.clone())
    }
}
