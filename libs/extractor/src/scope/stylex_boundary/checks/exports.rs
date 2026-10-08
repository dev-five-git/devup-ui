use oxc_ast::ast::{ExportAllDeclaration, ExportFromDeclaration, ExportNamedDeclaration};
use oxc_ast_visit::Visit;

use super::{Check, StylexSource};

impl Check<'_> {
    pub(super) fn named_export(&mut self, export: &ExportNamedDeclaration<'_>) {
        if export.export_kind.is_type() {
            return;
        }
        for specifier in &export.specifiers {
            if !specifier.export_kind.is_type() {
                self.visit_module_export_name(&specifier.local);
            }
        }
    }

    pub(super) fn source_export(&mut self, export: &ExportFromDeclaration<'_>) {
        if export.export_kind.is_type() {
            return;
        }
        let source = StylexSource::classify(export.source.value.as_str(), self.package);
        for specifier in &export.specifiers {
            if !specifier.export_kind.is_type()
                && (source.is_api_module()
                    || source == StylexSource::TypesOnly
                    || source == StylexSource::Root && specifier.local.name() == "stylex")
            {
                self.reject(specifier.span.start, &specifier.local.name(), "API namespace/function re-exports cannot run at runtime; export the compiled result instead");
            }
        }
    }

    pub(super) fn star_export(&mut self, export: &ExportAllDeclaration<'_>) {
        if !export.export_kind.is_type()
            && matches!(
                StylexSource::classify(export.source.value.as_str(), self.package),
                StylexSource::Upstream | StylexSource::Dedicated | StylexSource::TypesOnly
            )
        {
            self.reject(export.span.start, export.source.value.as_str(), "star re-exports expose compile-only APIs; export compiled results or explicit runtime-only root exports");
        }
    }
}
