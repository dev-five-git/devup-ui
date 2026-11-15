use std::{cell::RefCell, collections::HashMap, fmt::Display};

use extractor::{ExtractOption, ImportAlias, ModuleResolver};

use crate::Output;

#[derive(Clone, Copy)]
pub(crate) enum SourceMapMode {
    Generate,
    Skip,
}

pub(crate) fn checked_extraction<T, E: Display>(
    extracted: Result<T, E>,
    fault: Option<&RefCell<Option<String>>>,
) -> Result<T, String> {
    if let Some(message) = fault.and_then(|fault| fault.borrow_mut().take()) {
        return Err(message);
    }
    extracted.map_err(|error| error.to_string())
}

#[allow(clippy::too_many_arguments)]
pub fn code_extract_internal(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: HashMap<String, ImportAlias>,
    source_type: Option<extractor::ExtractSourceType>,
) -> Result<Output, String> {
    code_extract_internal_impl(
        filename,
        code,
        package,
        css_dir,
        single_css,
        import_main_css_in_code,
        import_main_css_in_css,
        import_aliases,
        SourceMapMode::Generate,
        None,
        None,
        source_type,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn code_extract_without_source_map_internal(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: HashMap<String, ImportAlias>,
    source_type: Option<extractor::ExtractSourceType>,
) -> Result<Output, String> {
    code_extract_internal_impl(
        filename,
        code,
        package,
        css_dir,
        single_css,
        import_main_css_in_code,
        import_main_css_in_css,
        import_aliases,
        SourceMapMode::Skip,
        None,
        None,
        source_type,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn code_extract_with_modules_internal(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: HashMap<String, ImportAlias>,
    source_type: Option<extractor::ExtractSourceType>,
    resolver: &ModuleResolver,
) -> Result<Output, String> {
    code_extract_internal_impl(
        filename,
        code,
        package,
        css_dir,
        single_css,
        import_main_css_in_code,
        import_main_css_in_css,
        import_aliases,
        SourceMapMode::Generate,
        Some(resolver),
        None,
        source_type,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn code_extract_internal_impl(
    filename: &str,
    code: &str,
    package: &str,
    css_dir: String,
    single_css: bool,
    import_main_css_in_code: bool,
    import_main_css_in_css: bool,
    import_aliases: HashMap<String, ImportAlias>,
    source_map: SourceMapMode,
    resolver: Option<&ModuleResolver>,
    resolver_fault: Option<&RefCell<Option<String>>>,
    source_type: Option<extractor::ExtractSourceType>,
    opaque_mdx_defaults: &[String],
) -> Result<Output, String> {
    let option = ExtractOption {
        package: package.to_string(),
        css_dir,
        single_css,
        import_main_css: import_main_css_in_code,
        import_aliases,
    };
    let extracted = extractor::extract_with_source_policy(
        filename,
        code,
        option,
        matches!(source_map, SourceMapMode::Generate),
        resolver,
        source_type,
        opaque_mdx_defaults,
    );
    crate::resident_owner::check_fault().map_err(|fault| fault.to_string())?;
    let output = checked_extraction(extracted, resolver_fault)?;
    let deferred = crate::output::deferred_defaults(&output);
    let mut result = Output::new(
        output.code,
        output.styles,
        output.map,
        single_css,
        filename.to_string(),
        output.css_file,
        import_main_css_in_css,
        output.dependencies,
    );
    result.deferred_defaults = deferred;
    Ok(result)
}
