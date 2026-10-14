use css::content_hash::FingerprintBits;
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use rustc_hash::FxHashSet;

use crate::{
    StyleSheet,
    name_registry::{self, NameError, NameRegistry},
};

impl StyleSheet {
    pub fn preflight_styles(
        &self,
        styles: &FxHashSet<ExtractStyleValue>,
        filename: &str,
        single_css: bool,
    ) -> Result<NameRegistry, NameError> {
        self.preflight_styles_with_bits(styles, (filename, single_css), FingerprintBits::PRODUCTION)
    }

    /// Check exact source claims before atoms, even when their CSS is identical.
    pub fn preflight_styles_with_bits(
        &self,
        styles: &FxHashSet<ExtractStyleValue>,
        source: (&str, bool),
        bits: FingerprintBits,
    ) -> Result<NameRegistry, NameError> {
        let (filename, single_css) = source;
        let scope = if single_css { None } else { Some(filename) };
        let mut styles: Vec<_> = styles.iter().collect();
        styles.sort_unstable();
        let sources = styles
            .iter()
            .filter_map(|style| name_registry::source_claim(style, filename, bits));
        let atoms = styles
            .iter()
            .filter_map(|style| name_registry::claim(style, (filename, scope), bits));
        let scopes = styles
            .iter()
            .filter_map(|style| name_registry::scope_claim(style, (filename, scope), bits));
        name_registry::preflight(&self.names, sources.chain(scopes).chain(atoms))
    }
}
