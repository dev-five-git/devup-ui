//! Pure Output materialization from the kernel's borrowed prospective sheet.
use crate::Output;
use sheet::{StyleSheet, counter_kernel::UpdateEffects};

/// Owned compilation metadata; contains no naming authority.
pub struct OutputMetadata {
    pub code: String,
    pub map: Option<String>,
    pub css_file: Option<String>,
    pub dependencies: Vec<String>,
}
/// Actual kernel finish inputs; no second update is performed.
pub struct ProspectiveSheet<'a> {
    pub sheet: &'a StyleSheet,
    pub effects: UpdateEffects,
}
/// Existing raw-source routing options used by Output rendering.
pub struct OutputRoute<'a> {
    pub raw_source: &'a str,
    pub single_css: bool,
    pub import_main_css: bool,
}
impl Output {
    /// Render actual prepared effects without allocating names or cleaning twice.
    #[must_use]
    pub fn from_prospective(
        metadata: OutputMetadata,
        prepared: ProspectiveSheet<'_>,
        route: OutputRoute<'_>,
    ) -> Self {
        let global = route.single_css || css::file_map::is_global(route.raw_source);
        let canonical = css::file_map::canonical(route.raw_source);
        let effects = prepared.effects;
        Self {
            code: metadata.code,
            map: metadata.map,
            css_file: metadata.css_file,
            dependencies: metadata.dependencies,
            updated_base_style: effects.updated_base_style || effects.default_collected,
            css: if effects.collected || effects.default_collected {
                Some(prepared.sheet.create_css(
                    if global { None } else { Some(&canonical) },
                    route.import_main_css,
                ))
            } else {
                None
            },
        }
    }
}
