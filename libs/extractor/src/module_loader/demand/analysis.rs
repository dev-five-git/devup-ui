use super::Demand;
use oxc_ast::ast::{Program, Statement};
use oxc_semantic::Semantic;

pub(super) struct Module<'a> {
    pub code: &'a str,
    pub program: &'a Program<'a>,
    pub semantic: Semantic<'a>,
    pub demand: Demand,
}

impl Module<'_> {
    pub(super) fn original_view(
        &self,
        filename: &str,
        option: &crate::ExtractOption,
        resolver: Option<&crate::ModuleResolver>,
    ) -> crate::ordinary_ve::selection::demand::View {
        let mut view = crate::ordinary_ve::selection::demand::select_consumer(
            (self.program, &self.semantic),
            &self.demand,
            (filename, "@vanilla-extract/css", resolver),
        );
        view.audit((self.program, &self.semantic), option);
        view
    }

    pub(super) fn view(
        &self,
        filename: &str,
        option: &crate::ExtractOption,
        resolver: Option<&crate::ModuleResolver>,
    ) -> crate::ordinary_ve::selection::demand::View {
        let accepts = |package: &str| {
            self.program.body.iter().any(|statement| {
                matches!(statement,
            Statement::ImportDeclaration(import) if import.source.value == package)
            })
        };
        let package = if crate::utils::is_vanilla_extract_file(filename)
            && accepts(&option.package)
            && !accepts("@vanilla-extract/css")
        {
            option.package.as_str()
        } else {
            "@vanilla-extract/css"
        };
        crate::ordinary_ve::selection::demand::select_resolved(
            (self.program, &self.semantic),
            &self.demand,
            (filename, package, resolver),
        )
    }
}
