use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::SourceType;

use crate::module_loader::{
    ModuleLoader,
    demand::{Demand, Producer},
};
use crate::ordinary_ve::selection::Selection;
use crate::ordinary_ve::selection::demand::{View, select_reads};
use crate::vanilla_extract::Stylesheet;
use crate::{ExtractOption, ImportAlias, ResolvedModule};

pub(in crate::module_loader::demand) fn program<'a>(
    arena: &'a Allocator,
    source: &'a str,
) -> Program<'a> {
    let parsed = Parser::new(arena, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{:?}", parsed.diagnostics);
    parsed.program
}

pub(in crate::module_loader::demand) fn semantic<'a>(program: &'a Program<'a>) -> Semantic<'a> {
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .with_check_syntax_error(true)
        .build(program);
    assert_eq!(built.diagnostics.len(), 0, "{:?}", built.diagnostics);
    built.semantic
}

pub(in crate::module_loader::demand) fn stylesheet<'a>(
    filename: &'a str,
    source: &'a str,
) -> Stylesheet<'a> {
    Stylesheet {
        filename,
        code: source,
        source,
        edits: &[],
    }
}

pub(super) fn binds(selection: &Selection, name: &str) -> bool {
    selection
        .units
        .iter()
        .flat_map(|unit| &unit.bindings)
        .any(|binding| binding.name == name)
}

pub(super) fn filenames(producers: &[Producer]) -> Vec<&str> {
    producers
        .iter()
        .map(|producer| producer.filename.as_str())
        .collect()
}

pub(super) fn backedge(
    source: &'static str,
    helper: &'static str,
) -> impl Fn(&str, &str) -> Option<ResolvedModule> {
    move |specifier, importer| {
        let (path, code) = match (specifier, importer) {
            ("./helper", "/entry.ts") => ("/helper.ts", helper),
            ("./entry", "/helper.ts") => ("/entry.ts", source),
            _ => return None,
        };
        Some(ResolvedModule {
            path: path.into(),
            code: code.into(),
        })
    }
}

pub(in crate::module_loader::demand) fn option() -> ExtractOption {
    ExtractOption {
        single_css: true,
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".into(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    }
}

pub(in crate::module_loader::demand) fn consumers(
    loader: &mut ModuleLoader<'_>,
    stylesheet: Stylesheet<'_>,
) -> (Result<bool, String>, View) {
    let allocator = Allocator::default();
    let program = program(&allocator, stylesheet.code);
    let semantic = semantic(&program);
    let plan = crate::imported_constants::consumer::plan(&program, &semantic, loader.option());
    assert_ne!(plan.reads.len(), 0);
    let initial = select_reads(
        (&program, &semantic),
        (&Demand::default(), &plan.reads),
        (
            stylesheet.filename,
            "@vanilla-extract/css",
            loader.resolver(),
        ),
    );
    (loader.select_consumers(stylesheet, &plan), initial)
}
