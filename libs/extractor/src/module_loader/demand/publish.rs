use oxc_ast::ast::Statement;

use super::{Frozen, Producer, Unit, analysis::Module, retained_css};
use crate::{imported_constants::consumer::ReadPlan, vanilla_extract::Stylesheet};

impl Frozen {
    pub(super) fn publish(
        &mut self,
        modules: Vec<(String, Module<'_>)>,
        input: (Stylesheet<'_>, Option<&ReadPlan>, &str),
        loader: &super::super::ModuleLoader<'_>,
    ) -> Result<(), String> {
        let (entry, consumer, original_sources) = input;
        for (filename, module) in modules {
            if filename == entry.filename {
                if let Some(plan) = consumer {
                    let mut view = crate::ordinary_ve::selection::demand::select_reads(
                        (module.program, &module.semantic),
                        (&module.demand, &plan.reads),
                        (entry.filename, "@vanilla-extract/css", loader.resolver),
                    );
                    view.audit((module.program, &module.semantic), loader.option);
                    self.entry_view = Some(view);
                } else {
                    self.entry = Some(
                        module
                            .view(&filename, loader.option, loader.resolver)
                            .selection,
                    );
                }
                continue;
            }
            let stylesheet = Stylesheet {
                filename: &filename,
                code: module.code,
                source: module.code,
                edits: &[],
            };
            super::super::validate(stylesheet)?;
            let view = if consumer.is_some() {
                module.original_view(&filename, loader.option, loader.resolver)
            } else {
                module.view(&filename, loader.option, loader.resolver)
            };
            crate::ordinary_ve::execution::policy::check(stylesheet, &view.selection)?;
            let native = !view.selection.roots.is_empty();
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
            if commonjs && consumer.is_none() {
                continue;
            }
            if consumer.is_none()
                && crate::utils::is_vanilla_extract_file(&filename)
                && !native
                && !view.native_carrier
            {
                continue;
            }
            let mut rendered = view.render(stylesheet, loader.option)?;
            let namespace = crate::fresh_name::fresh_name(
                &format!("__ve_owner_{}__", self.environments.len()),
                original_sources,
            );
            let mut checks = if consumer.is_some() {
                crate::ordinary_ve::execution::policy::observations(stylesheet, &view.selection)?
            } else {
                Vec::new()
            };
            if commonjs && consumer.is_some() {
                checks.extend(
                    crate::ordinary_ve::execution::policy::commonjs_observations(
                        stylesheet,
                        &view.selection,
                        (module.program, &module.semantic),
                    ),
                );
            }
            let producer = Producer {
                filename: filename.clone(),
                source: module.code.to_string(),
                namespace: namespace.clone(),
                bindings: rendered.bindings,
                native: rendered.native,
                reserved: view.selection.reserved_names.clone(),
                checks,
            };
            if !producer.checks.is_empty() {
                let mut reads: Vec<_> = producer
                    .checks
                    .iter()
                    .map(|check| check.read.clone())
                    .collect();
                reads.sort();
                reads.dedup();
                rendered
                    .mapped
                    .synthesize(0, &format!("{namespace}$audit([{}]);\n", reads.join(",")));
            }
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
                self.producers.push(producer.clone());
            }
            self.environments.push(producer);
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
            self.units.insert(
                filename.clone(),
                Unit::selected(stylesheet, &rendered.mapped)?,
            );
        }
        Ok(())
    }
}
