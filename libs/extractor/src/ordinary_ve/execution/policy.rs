use super::{Selection, Stylesheet};
use crate::import_alias_visit::source_offset;
use crate::mutations::Use;
use oxc_span::GetSpan;

pub(crate) fn place(stylesheet: Stylesheet<'_>, at: u32) -> String {
    let offset = stylesheet.edits.iter().fold(
        usize::try_from(at).unwrap_or(stylesheet.code.len()),
        |offset, edits| source_offset(edits, offset),
    );
    crate::locate(stylesheet.filename, stylesheet.source, offset)
}

pub(crate) fn check(stylesheet: Stylesheet<'_>, selection: &Selection) -> Result<(), String> {
    if let Some(escape) = selection.escapes.first() {
        return Err(format!(
            "{}: {}. Fix: {}",
            place(stylesheet, escape.span.start),
            escape.cause(),
            escape.fix()
        ));
    }
    for unit in &selection.units {
        let erased = match unit.kind {
            super::super::selection::plan::UnitKind::Declarator { erased, .. }
            | super::super::selection::plan::UnitKind::Function { erased } => erased,
            super::super::selection::plan::UnitKind::Statement => false,
        };
        if erased {
            let site = selection
                .helper_calls
                .iter()
                .find(|call| call.callable == unit.node)
                .map_or(unit.span.start, |call| call.span.start);
            return Err(format!(
                "{}: a required native input is erased by TypeScript. Fix: supply an initialized runtime value",
                place(stylesheet, site)
            ));
        }
    }
    Ok(())
}

pub(super) fn observations(
    stylesheet: Stylesheet<'_>,
    selection: &Selection,
) -> Result<Vec<crate::vanilla_extract::capture::MutationCheck>, String> {
    let mut checks = Vec::new();
    let aliases = super::aliases::Aliases::new(stylesheet, selection);
    for mutation in &selection.mutations {
        let (mut at, path) = match &mutation.usage {
            Use::Changes { at, .. } => (*at, Vec::new()),
            Use::Escapes {
                into: Some(into), ..
            } if into.is_empty() => continue,
            Use::Calls { at, path } | Use::Escapes { at, path, .. } => (*at, path.clone()),
        };
        if selection
            .units
            .iter()
            .any(|unit| (unit.span.start..unit.span.end).contains(&at))
        {
            continue;
        }
        let binding = selection.units.iter().flat_map(|unit| &unit.bindings)
            .chain(selection.imports.iter().map(|import| &import.binding))
            .find(|binding| binding.symbol == mutation.symbol)
            .ok_or_else(|| format!("{}: required mutation input has no selected binding. Fix: report this extraction error with the original module", place(stylesheet, at)))?;
        if let Use::Escapes {
            into: Some(into), ..
        } = &mutation.usage
        {
            match aliases.classify(&binding.name, into) {
                super::aliases::Alias::Readonly => continue,
                super::aliases::Alias::Mutable(site) => at = site,
                super::aliases::Alias::Other => {}
            }
        }
        checks.push(crate::vanilla_extract::capture::MutationCheck {
            read: binding.name.clone(),
            path,
            place: place(stylesheet, at),
        });
    }
    Ok(checks)
}

pub(super) fn writes(
    stylesheet: Stylesheet<'_>,
    selection: &Selection,
    semantic: &oxc_semantic::Semantic<'_>,
) -> Result<(), String> {
    let scoping = semantic.scoping();
    for binding in selection.units.iter().flat_map(|unit| &unit.bindings) {
        let Some(symbol) = scoping.get_root_binding(binding.name.as_str().into()) else {
            continue;
        };
        for reference in scoping.get_resolved_reference_ids(symbol) {
            let data = scoping.get_reference(*reference);
            let span = semantic.nodes().kind(data.node_id()).span();
            if data.is_write()
                && !selection
                    .units
                    .iter()
                    .any(|unit| unit.span.contains_inclusive(span))
            {
                return Err(format!(
                    "{}: required binding `{}` may be changed outside its selected initialization slice. Fix: keep the exact write inside its initializer or use immutable input data",
                    place(stylesheet, span.start),
                    binding.name,
                ));
            }
        }
    }
    Ok(())
}
