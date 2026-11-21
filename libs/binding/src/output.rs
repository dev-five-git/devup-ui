use css::file_map::{canonical, is_global};
use extractor::{
    ExtractOption, ExtractOutput, ExtractTarget, ModuleResolver, NativePlatform, NativeTarget,
    extract, extract_with_modules, extract_without_source_map,
};

use crate::css_cache::invalidate_css;
use crate::lazy_css::{LazyCss, settle};
use crate::native_theme::native_theme;
use crate::{with_style_sheet, with_style_sheet_untracked};

/// Whether extraction generates a source map.
#[derive(Clone, Copy)]
pub enum SourceMapMode {
    /// Generate source maps using the existing extractor path.
    Generate,
    /// Skip source-map generation.
    Skip,
}

/// CSS emission settings shared by real transforms and output test seams.
#[derive(Clone, Copy)]
pub struct CssOptions {
    /// Emit all atoms to the global sheet.
    pub single_css: bool,
    /// Import the main stylesheet from emitted file CSS.
    pub import_main_css: bool,
}

/// Host-neutral extraction routing and CSS import policy.
pub struct ExtractionMode<'a> {
    /// Source-map generation policy.
    pub source_map: SourceMapMode,
    /// Borrow the host resolver only for this synchronous extraction.
    pub resolver: Option<&'a ModuleResolver>,
    /// Refuse publication after extraction, before matching its result.
    pub resolver_check: Option<&'a dyn Fn() -> Result<(), String>>,
    /// Import the main stylesheet from emitted file CSS.
    pub import_main_css_in_css: bool,
}

/// Materialized transform result, independent of host handles and getters.
pub struct Output {
    /// Transformed source.
    pub code: String,
    /// Generated source map, when requested and available.
    pub map: Option<String>,
    /// Emitted stylesheet filename.
    pub css_file: Option<String>,
    /// Whether shared base CSS changed.
    pub updated_base_style: bool,
    /// Changed CSS, serialized on its first read; absent when the stylesheet is unchanged.
    pub css: Option<LazyCss>,
    /// Files read through the module resolver.
    pub dependencies: Vec<String>,
    /// The component-selector markers the source gives and selects by.
    pub component_markers: extractor::ComponentMarkers,
}

impl Output {
    /// Commit extracted styles to their existing bucket, deferring the changed CSS.
    pub fn new(output: ExtractOutput, filename: String, options: CssOptions) -> Self {
        // Use the bucket identity (single-importer collapse) so the sheet's CSS
        // naming + property bucket + emitted chunk match the canonical class names
        // the extractor already baked into `code`. Identity when no map is loaded.
        // Global (shared-chunk) files are treated like single-css: emitted into the
        // global bucket (devup-ui.css) with prefix-less naming.
        let _atom_mode = css::atom_hoist::AtomModeGuard::new(options.single_css);
        let canonical_filename = canonical(&filename);
        let global = options.single_css || is_global(&filename);
        with_style_sheet_untracked(|sheet| {
            settle(sheet);
            let planned = sheet.atom_plan.is_some();
            // Global CSS belongs to the raw SOURCE file, never its collapsed bucket.
            let default_collected = sheet.rm_global_css(&filename, global);
            let (collected, updated_base_style) =
                sheet.update_styles_in_bucket(&output.styles, &canonical_filename, global);
            // The commit reports every style, global rule, import and font face it
            // added or removed, but not the atom plan its first commit freezes into
            // the sheet. A source extracted again changes neither: what was
            // serialized before still holds.
            if collected
                || updated_base_style
                || default_collected
                || planned != sheet.atom_plan.is_some()
            {
                invalidate_css();
            }
            Self {
                code: output.code,
                map: output.map,
                css_file: output.css_file,
                dependencies: output.dependencies,
                component_markers: output.component_markers,
                updated_base_style: updated_base_style || default_collected,
                css: {
                    if !collected && !default_collected {
                        None
                    } else {
                        Some(LazyCss::defer(
                            if global {
                                None
                            } else {
                                Some(&canonical_filename)
                            },
                            options.import_main_css,
                        ))
                    }
                },
            }
        })
    }
}

/// Extract a `(filename, code)` source with an optional borrowed host resolver.
pub fn code_extract(
    source: (&str, &str),
    option: ExtractOption,
    mode: ExtractionMode<'_>,
) -> Result<Output, String> {
    let (filename, code) = source;
    let css = CssOptions {
        single_css: option.single_css,
        import_main_css: mode.import_main_css_in_css,
    };
    let extracted = match (mode.resolver, mode.source_map) {
        (Some(resolver), source_map) => extract_with_modules(
            filename,
            code,
            option,
            matches!(source_map, SourceMapMode::Generate),
            resolver,
        ),
        (None, SourceMapMode::Generate) => extract(filename, code, option),
        (None, SourceMapMode::Skip) => extract_without_source_map(filename, code, option),
    };
    if let Some(check) = mode.resolver_check {
        check()?;
    }
    match extracted {
        Ok(output) => Ok(Output::new(output, filename.to_string(), css)),
        Err(error) => Err(error.to_string()),
    }
}

/// Compile a `(filename, code)` source for React Native on `platform`
/// (`ios`, `android` or `web`): its code and source map, with no CSS. Theme
/// tokens resolve in the registered theme.
///
/// `runtime` lets styles import the
/// React Native runtime, and `reanimated` tells that the app has
/// react-native-reanimated, which animations and transitions run on
pub fn code_extract_native(
    source: (&str, &str),
    package: &str,
    platform: &str,
    (runtime, reanimated): (bool, bool),
) -> Result<Output, String> {
    let (filename, code) = source;
    let platform = platform.parse::<NativePlatform>().map_err(|_| {
        format!("unknown React Native platform `{platform}`: use `ios`, `android` or `web`")
    })?;
    let option = ExtractOption {
        package: package.to_string(),
        target: ExtractTarget::Native(NativeTarget {
            platform,
            runtime,
            reanimated,
            theme: std::sync::Arc::new(with_style_sheet(|sheet| native_theme(&sheet.theme))),
        }),
        ..ExtractOption::default()
    };
    let output = extract(filename, code, option).map_err(|error| error.to_string())?;
    Ok(Output {
        code: output.code,
        map: output.map,
        css_file: None,
        updated_base_style: false,
        css: None,
        dependencies: output.dependencies,
        component_markers: output.component_markers,
    })
}
