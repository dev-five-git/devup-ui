use super::{Unit, retained_css};
use crate::vanilla_extract::Stylesheet;
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::SourceType;
use rustc_hash::FxHashMap;
use std::{collections::BTreeSet, rc::Rc};
mod analysis;
mod path;
mod reads;
mod requests;
pub(super) mod script;
use analysis::Module;
pub(crate) use path::Demand;

#[derive(Clone)]
pub(crate) struct Producer {
    pub filename: String,
    pub source: String,
    pub namespace: String,
    pub bindings: Vec<crate::ordinary_ve::selection::plan::Binding>,
    pub native: Vec<crate::ordinary_ve::selection::plan::Binding>,
    pub reserved: BTreeSet<String>,
}

#[derive(Default)]
pub(super) struct Frozen {
    pub units: FxHashMap<String, Rc<Unit>>,
    pub producers: Vec<Producer>,
    pub environments: Vec<Producer>,
    pub entry: Option<crate::ordinary_ve::selection::Selection>,
    pub source: String,
}

impl Frozen {
    pub fn rewrites(&self, filename: &str, semantic: &Semantic<'_>) -> Vec<(u32, u32, String)> {
        self.environments
            .iter()
            .find(|producer| producer.filename == filename)
            .map_or_else(Vec::new, |producer| reads::rewrite(semantic, producer))
    }
    pub fn exported(&self, filename: &str, local: &str) -> String {
        self.environments
            .iter()
            .find(|producer| producer.filename == filename)
            .map_or_else(
                || local.to_string(),
                |producer| format!("{}$read({local})", producer.namespace),
            )
    }
    pub fn discover(
        stylesheet: Stylesheet<'_>,
        selected: bool,
        loader: &super::ModuleLoader<'_>,
    ) -> Result<Self, String> {
        let allocator = Allocator::default();
        let parsed = Parser::new(
            &allocator,
            stylesheet.code,
            SourceType::from_path(stylesheet.filename).unwrap_or_default(),
        )
        .parse();
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(&parsed.program)
            .semantic;
        let arena = Allocator::default();
        let mut modules: FxHashMap<String, Module<'_>> = FxHashMap::default();
        let mut pending: Vec<_> = requests::entry(&parsed.program, &semantic, selected)
            .into_iter()
            .map(|(specifier, demand, site)| {
                (
                    stylesheet.filename.to_string(),
                    specifier,
                    demand,
                    crate::ordinary_ve::execution::policy::place(stylesheet, site.start),
                )
            })
            .collect();
        while let Some((importer, specifier, demand, place)) = pending.pop() {
            if specifier == loader.option.package || specifier == "@vanilla-extract/css" {
                continue;
            }
            let resolver = loader.resolver.ok_or_else(|| {
                super::LoadError::NoResolver.describe(&specifier, &importer, || place.clone())
            })?;
            let module = resolver(&specifier, &importer).ok_or_else(|| {
                super::LoadError::Unresolved.describe(&specifier, &importer, || place.clone())
            })?;
            if retained_css::is_css(&module.path) {
                continue;
            }
            let record = modules.entry(module.path.clone()).or_insert_with(|| {
                let source = if module.path == stylesheet.filename {
                    stylesheet.code
                } else {
                    &module.code
                };
                let code = arena.alloc_str(source);
                let parsed = Parser::new(
                    &arena,
                    code,
                    SourceType::from_path(&module.path).unwrap_or_default(),
                )
                .parse();
                let program = arena.alloc(parsed.program);
                let semantic = SemanticBuilder::new()
                    .with_build_nodes(true)
                    .build(program)
                    .semantic;
                Module {
                    code,
                    program,
                    semantic,
                    demand: Demand::default(),
                }
            });
            if !record.demand.merge(&demand) {
                continue;
            }
            let view = record.view(&module.path, loader.option);
            pending.extend(view.forwarded.into_iter().map(|(specifier, demand, site)| {
                (
                    module.path.clone(),
                    specifier,
                    demand,
                    crate::locate(
                        &module.path,
                        record.code,
                        usize::try_from(site.start).unwrap_or(0),
                    ),
                )
            }));
        }
        let mut frozen = Self::default();
        let mut modules: Vec<_> = modules.into_iter().collect();
        modules.sort_by(|left, right| left.0.cmp(&right.0));
        let original_sources = std::iter::once(stylesheet.code)
            .chain(modules.iter().map(|(_, module)| module.code))
            .collect::<Vec<_>>()
            .join("\n");
        frozen.source.clone_from(&original_sources);
        for (filename, module) in modules {
            if filename == stylesheet.filename {
                frozen.entry = Some(module.view(&filename, loader.option).selection);
                continue;
            }
            let stylesheet = Stylesheet {
                filename: &filename,
                code: module.code,
                source: module.code,
                edits: &[],
            };
            super::validate(stylesheet)?;
            let view = module.view(&filename, loader.option);
            let native = view
                .selection
                .imports
                .iter()
                .any(|import| import.native.is_some());
            let commonjs = !module
                .program
                .body
                .iter()
                .any(Statement::is_module_declaration)
                && module
                    .semantic
                    .scoping()
                    .root_unresolved_references()
                    .keys()
                    .any(|name| matches!(name.as_str(), "module" | "exports" | "require"));
            if commonjs {
                continue;
            }
            if crate::utils::is_vanilla_extract_file(&filename) && !native {
                continue;
            }
            let mut rendered = view.render(stylesheet, loader.option)?;
            let namespace = crate::fresh_name::fresh_name(
                &format!("__ve_owner_{}__", frozen.environments.len()),
                &original_sources,
            );
            let producer = Producer {
                filename: filename.clone(),
                source: module.code.to_string(),
                namespace: namespace.clone(),
                bindings: rendered.bindings,
                native: rendered.native,
                reserved: view.selection.reserved_names.clone(),
            };
            if native {
                crate::ordinary_ve::execution::policy::check(stylesheet, &view.selection)?;
                let reads = producer
                    .bindings
                    .iter()
                    .map(|binding| binding.name.clone())
                    .collect::<Vec<_>>()
                    .join(",");
                rendered
                    .mapped
                    .synthesize(0, &format!("{namespace}$finish([{reads}]);\n"));
                frozen.producers.push(producer.clone());
            }
            frozen.environments.push(producer);
            for statement in &module.program.body {
                if let Statement::ImportDeclaration(import) = statement
                    && import
                        .specifiers
                        .as_ref()
                        .is_none_or(|specifiers| specifiers.is_empty())
                    && retained_css::is_css(import.source.value.as_str())
                {
                    rendered.mapped.copy(module.code, import.span);
                    rendered.mapped.synthesize(import.span.end, "\n");
                }
            }
            frozen.units.insert(
                filename.clone(),
                Unit::selected(stylesheet, &rendered.mapped)?,
            );
        }
        Ok(frozen)
    }
}
