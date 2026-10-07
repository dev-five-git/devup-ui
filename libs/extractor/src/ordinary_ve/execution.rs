use oxc_span::Span;

use super::selection::plan::{Binding, Selection};
use crate::vanilla_extract::{CollectedStyles, Stylesheet, StylesheetImports, capture};

mod aliases;
mod consumer;
pub(crate) mod diagnostics;
pub(crate) mod imports;
mod observe;
pub(crate) mod policy;
mod source;
#[cfg(test)]
mod tests;

pub(crate) struct Captured {
    pub root: Span,
    pub binding: Option<Binding>,
    pub name: String,
    pub expression: String,
}

pub(crate) struct Executed {
    pub collected: CollectedStyles,
    pub imports: StylesheetImports,
    pub captures: Vec<Captured>,
    pub emission: capture::Emission,
    pub readback: Vec<(Span, String)>,
}

#[derive(Clone, Copy)]
pub(crate) struct SelectedModule<'a> {
    pub stylesheet: Stylesheet<'a>,
    pub selection: &'a Selection,
}

pub(crate) fn execute(
    module: SelectedModule<'_>,
    option: &crate::ExtractOption,
    resolver: Option<&crate::ModuleResolver>,
) -> Result<Executed, String> {
    run(module, (option, resolver), None)?.ok_or_else(|| {
        format!(
            "{}:1:1: missing selected native transaction. Fix: report this extraction error",
            module.stylesheet.filename
        )
    })
}

pub(crate) fn execute_reads(
    module: SelectedModule<'_>,
    configuration: (&crate::ExtractOption, Option<&crate::ModuleResolver>),
    plan: &crate::imported_constants::consumer::ReadPlan,
) -> Result<Option<Executed>, String> {
    run(module, configuration, Some(plan))
}

fn run(
    module: SelectedModule<'_>,
    configuration: (&crate::ExtractOption, Option<&crate::ModuleResolver>),
    plan: Option<&crate::imported_constants::consumer::ReadPlan>,
) -> Result<Option<Executed>, String> {
    let (option, resolver) = configuration;
    let SelectedModule {
        stylesheet,
        selection,
    } = module;
    let mut loader = crate::module_loader::ModuleLoader::new(resolver, option);
    match plan {
        Some(plan) => {
            if !loader.select_consumers(stylesheet, plan)? {
                return Ok(None);
            }
        }
        None => loader.select_demands(stylesheet, true)?,
    }
    if let Some(plan) = plan
        && let Some((span, message)) = plan.failures.first()
    {
        return Err(format!(
            "{}: {message}",
            policy::place(stylesheet, span.start)
        ));
    }
    let selection = loader.entry_selection().unwrap_or(selection);
    policy::check(stylesheet, selection)?;
    let mutations = policy::observations(stylesheet, selection)?;
    let selected = match (plan, loader.entry_view()) {
        (Some(plan), Some(view)) => source::build_reads(
            SelectedModule {
                stylesheet,
                selection,
            },
            option,
            (view, plan),
        )?,
        (Some(_), None) => {
            return Err(format!(
                "{}:1:1: missing consumer demand view. Fix: report this extraction error",
                stylesheet.filename
            ));
        }
        (None, _) => source::build(stylesheet, selection, option)?,
    };
    let captured = capture::Selected {
        stylesheet,
        mapped: &selected.mapped,
        captures: &selected.captures,
        reserved: &selected.reserved,
        mutations: &mutations,
        observer: &selected.observations.helper,
        observations: &selected.observations.sites,
    };
    let result = match plan {
        Some(plan) => capture::execute_reads(captured, loader, &plan.slots)?,
        None => capture::execute(captured, loader)?,
    };
    let captures = selected
        .identities
        .into_iter()
        .zip(selected.captures)
        .zip(result.captures)
        .filter(|(((root, _), _), _)| plan.is_none_or(|plan| !plan.slots.contains(root)))
        .map(|(((root, binding), capture), expression)| Captured {
            root,
            binding,
            name: capture.name,
            expression,
        })
        .collect();
    Ok(Some(Executed {
        collected: result.collected,
        imports: result.imports,
        captures,
        emission: result.emission,
        readback: result.readback,
    }))
}
