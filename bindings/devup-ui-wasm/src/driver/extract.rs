use std::{cell::RefCell, fmt::Display};

use extractor::ExtractOption;

use super::{Driver, fallible_resolver};
use crate::{Output, SourceMapMode};

/// Inputs shared with the plain extraction API; raw host values are parsed here, in order.
pub(crate) struct Source<'a> {
    pub filename: &'a str,
    pub code: &'a str,
    pub option: ExtractOption,
    pub import_main_css_in_css: bool,
    pub map_mode: SourceMapMode,
}

pub(crate) struct Arguments<V> {
    pub aliases: V,
    pub source_type: V,
    pub opaque_defaults: V,
}

impl<V: Clone, F: Clone, S, C: Display, E: Into<V>, T> Driver<V, F, S, C, E, T> {
    pub(crate) fn extract(&self, source: Source<'_>, args: Arguments<V>) -> Result<Output, V> {
        let _scope = self.scope();
        let source_type = self
            .resolver
            .source_type(&args.source_type)
            .map_err(|error| {
                self.errors
                    .message(format!("{}:1:1: {error}", source.filename))
            })?;
        let aliases = self.aliases(args.aliases)?;
        let defaults = if (self.resolver.values.is_undefined)(&args.opaque_defaults)
            || (self.resolver.values.is_null)(&args.opaque_defaults)
        {
            Vec::new()
        } else {
            (self.conversion.strings)(args.opaque_defaults)
                .map_err(|error| self.errors.message(error))?
        };
        let fault = RefCell::new(None);
        let installed = self.installed.with_borrow(Clone::clone);
        let resolver = installed.map(|resolver| {
            fallible_resolver(&fault, move |specifier, importer| {
                self.resolver.call(&resolver, specifier, importer)
            })
        });
        crate::code_extract_internal_impl(
            source.filename,
            source.code,
            &source.option.package,
            source.option.css_dir,
            source.option.single_css,
            source.option.import_main_css,
            source.import_main_css_in_css,
            aliases,
            source.map_mode,
            resolver.as_deref(),
            Some(&fault),
            source_type,
            &defaults,
        )
        .map_err(|message| self.errors.extraction(&message))
    }

    pub(crate) fn relevant(&self, filename: &str, code: &str, package: &str, aliases: V) -> bool {
        let _scope = self.scope();
        let aliases = self.aliases(aliases).unwrap_or_default();
        let fault = RefCell::new(None);
        let installed = self.installed.with_borrow(Clone::clone);
        let resolver = installed.map(|resolver| {
            fallible_resolver(&fault, move |specifier, importer| {
                self.resolver.call(&resolver, specifier, importer)
            })
        });
        let relevant =
            extractor::has_devup_ui_with(filename, code, package, &aliases, resolver.as_deref());
        relevant || fault.borrow().is_some()
    }
}
