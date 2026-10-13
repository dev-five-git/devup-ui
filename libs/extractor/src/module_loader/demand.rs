use super::{Unit, retained_css};
use crate::vanilla_extract::Stylesheet;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::{Semantic, SemanticBuilder};
use oxc_span::SourceType;
use rustc_hash::FxHashMap;
use std::{collections::BTreeSet, rc::Rc};
mod analysis;
mod path;
mod publish;
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
    pub checks: Vec<crate::vanilla_extract::capture::MutationCheck>,
}

#[derive(Default)]
pub(super) struct Frozen {
    pub units: FxHashMap<String, Rc<Unit>>,
    pub producers: Vec<Producer>,
    pub environments: Vec<Producer>,
    pub entry: Option<crate::ordinary_ve::selection::Selection>,
    pub entry_view: Option<crate::ordinary_ve::selection::demand::View>,
    pub source: String,
    pub dependencies: BTreeSet<String>,
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
        input: (bool, Option<&crate::imported_constants::consumer::ReadPlan>),
        loader: &super::ModuleLoader<'_>,
    ) -> Result<Self, String> {
        let (selected, consumer) = input;
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
        let entry_selection = crate::ordinary_ve::selection::select_resolved(
            (&parsed.program, &semantic),
            (stylesheet.filename, "@vanilla-extract/css"),
            loader.resolver,
        );
        let entry_view = consumer.map(|plan| {
            let mut view = crate::ordinary_ve::selection::demand::select_reads(
                (&parsed.program, &semantic),
                (&Demand::default(), &plan.reads),
                (stylesheet.filename, "@vanilla-extract/css", loader.resolver),
            );
            view.audit((&parsed.program, &semantic), loader.option);
            view
        });
        let mut dependencies = entry_selection.dependencies.clone();
        let mut failures = Vec::new();
        let mut pending: Vec<_> =
            requests::entry((&parsed.program, &semantic), selected, &entry_selection)
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
        if let Some(view) = &entry_view {
            pending.extend(
                view.forwarded
                    .iter()
                    .cloned()
                    .map(|(specifier, demand, site)| {
                        (
                            stylesheet.filename.to_string(),
                            specifier,
                            demand,
                            crate::ordinary_ve::execution::policy::place(stylesheet, site.start),
                        )
                    }),
            );
        }
        while let Some((importer, specifier, demand, place)) = pending.pop() {
            if specifier == loader.option.package || specifier == "@vanilla-extract/css" {
                continue;
            }
            let module = match loader.resolver {
                Some(resolver) => resolver(&specifier, &importer).ok_or_else(|| {
                    super::LoadError::Unresolved.describe(&specifier, &importer, || place.clone())
                }),
                None => {
                    Err(super::LoadError::NoResolver
                        .describe(&specifier, &importer, || place.clone()))
                }
            };
            let module = match module {
                Ok(module) => module,
                Err(error) if consumer.is_some() => {
                    failures.push(error);
                    continue;
                }
                Err(error) => return Err(error),
            };
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
            let view = if module.path == stylesheet.filename
                && let Some(plan) = consumer
            {
                let mut view = crate::ordinary_ve::selection::demand::select_reads(
                    (record.program, &record.semantic),
                    (&record.demand, &plan.reads),
                    (stylesheet.filename, "@vanilla-extract/css", loader.resolver),
                );
                view.audit((record.program, &record.semantic), loader.option);
                view
            } else if consumer.is_some() {
                record.original_view(&module.path, loader.option, loader.resolver)
            } else {
                record.view(&module.path, loader.option, loader.resolver)
            };
            dependencies.extend(view.selection.dependencies.iter().cloned());
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
        if consumer.is_some()
            && entry_selection.roots.is_empty()
            && !modules.iter().any(|(filename, module)| {
                !module
                    .original_view(filename, loader.option, loader.resolver)
                    .selection
                    .roots
                    .is_empty()
            })
        {
            return Ok(Self::default());
        }
        if let Some(error) = failures.into_iter().next() {
            return Err(error);
        }
        let mut frozen = Self {
            entry: selected.then_some(entry_selection),
            entry_view,
            dependencies,
            ..Self::default()
        };
        let mut modules: Vec<_> = modules.into_iter().collect();
        modules.sort_by(|left, right| left.0.cmp(&right.0));
        let original_sources = std::iter::once(stylesheet.code)
            .chain(modules.iter().map(|(_, module)| module.code))
            .collect::<Vec<_>>()
            .join("\n");
        frozen.source.clone_from(&original_sources);
        frozen.publish(modules, (stylesheet, consumer, &original_sources), loader)?;
        Ok(frozen)
    }
}

#[cfg(test)]
mod loader_coverage_tests;
#[cfg(test)]
mod request_read_coverage_tests;
