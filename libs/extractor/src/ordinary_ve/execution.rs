use oxc_span::Span;

use super::selection::plan::{Binding, Selection};
use crate::vanilla_extract::{CollectedStyles, Stylesheet, StylesheetImports, capture};

mod aliases;
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
    let SelectedModule {
        stylesheet,
        selection,
    } = module;
    let mut loader = crate::module_loader::ModuleLoader::new(resolver, option);
    loader.select_demands(stylesheet, true)?;
    let selection = loader.entry_selection().unwrap_or(selection);
    policy::check(stylesheet, selection)?;
    let mutations = policy::observations(stylesheet, selection)?;
    let selected = source::build(stylesheet, selection, option)?;
    let result = capture::execute(
        capture::Selected {
            stylesheet,
            mapped: &selected.mapped,
            captures: &selected.captures,
            reserved: &selected.reserved,
            mutations: &mutations,
            observer: &selected.observations.helper,
            observations: &selected.observations.sites,
        },
        loader,
    )?;
    let captures = selected
        .identities
        .into_iter()
        .zip(selected.captures)
        .zip(result.captures)
        .map(|(((root, binding), capture), expression)| Captured {
            root,
            binding,
            name: capture.name,
            expression,
        })
        .collect();
    Ok(Executed {
        collected: result.collected,
        imports: result.imports,
        captures,
        emission: result.emission,
    })
}
