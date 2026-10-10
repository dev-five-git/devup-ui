use super::{
    Cleanup, FrozenAuthority, LinkedBatch, authority,
    error::KernelError,
    live::{UpdateEffects, UpdateRequest},
    publication, records,
    scratch::{self, ScratchRequest},
    state::CounterState,
    traversal::Traversal,
};
use crate::{StyleSheet, emission_seed::EmissionContext};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use rustc_hash::FxHashSet;

pub(super) struct Preparation<'a> {
    pub(super) base: &'a LinkedBatch,
    pub(super) styles: &'a FxHashSet<ExtractStyleValue>,
    pub(super) request: UpdateRequest<'a>,
}

pub(super) fn prepare(
    sheet: &mut StyleSheet,
    input: Preparation<'_>,
) -> Result<(CounterState, UpdateEffects), KernelError> {
    css::atom_hoist::freeze_atom_plan();
    let plan = css::atom_hoist::atom_plan();
    if sheet.atom_plan.is_some() && sheet.atom_plan != plan {
        return Err(KernelError::Authority);
    }
    let single_css = input.request.single_css || css::file_map::is_global(input.request.raw_source);
    let bucket = if single_css {
        String::new()
    } else {
        css::file_map::canonical(input.request.raw_source)
    };
    if !single_css {
        let _ordinal = css::file_map::get_file_num_by_filename(&bucket);
    }
    let placement = EmissionContext {
        source_file: input.request.raw_source.into(),
        bucket: bucket.clone(),
        single_css,
        hoisted: !single_css
            && css::atom_hoist::is_atom_hoist()
            && css::atom_hoist::is_hoisted_bucket(input.request.raw_source),
    };
    let traversal = Traversal::run(sheet, input.styles, &placement)?;
    let incoming = traversal.linked()?;
    let cleanup = Cleanup {
        source: input.request.raw_source.into(),
        bucket,
        single_css,
    };
    let mut scratch = scratch::apply_scratch(
        sheet,
        input.base,
        ScratchRequest::new(&incoming, Some(&cleanup))?.ordered(&traversal.operations),
    )?;
    let mut distinct = Vec::new();
    for candidate in std::mem::take(&mut scratch.candidates) {
        if !distinct.contains(&candidate) {
            distinct.push(candidate);
        }
    }
    scratch.candidates = distinct;
    let mut authority = match &sheet.counter_state {
        Some(state) => state.projection(&authority::classes()),
        None => FrozenAuthority::live(),
    };
    authority.originals.extend(traversal.authority.originals);
    authority.files.extend(traversal.authority.files);
    authority.deliveries.extend(traversal.authority.deliveries);
    for placement in traversal.authority.placements {
        if !authority.placements.contains(&placement) {
            authority.placements.push(placement);
        }
    }
    authority.classes = authority::classes();
    authority.authored.clone_from(&scratch.evidence.authored);
    authority.cleanups.clone_from(&scratch.cleanups);
    authority.phase = scratch.phase.clone();
    authority.retain_references(&scratch.candidates);
    let linked =
        LinkedBatch::link_captured_batch(&scratch.candidates, &scratch.evidence, &authority)?;
    let mut prospective = super::emission::clone_sheet(sheet);
    prospective.properties = scratch.properties.clone();
    prospective.css = scratch
        .css
        .iter()
        .map(|(path, rules)| {
            (
                path.clone(),
                rules
                    .iter()
                    .map(|rule| crate::StyleSheetCss {
                        css: rule.css.clone(),
                    })
                    .collect(),
            )
        })
        .collect();
    prospective.keyframes = scratch.keyframes.clone();
    prospective
        .global_css_files
        .clone_from(&scratch.global_css_files);
    prospective.imports = scratch.imports.clone();
    prospective.font_faces = scratch.font_faces.clone();
    records::coverage(&prospective, &linked.records)?;
    let pending = CounterState::from_captured(scratch.candidates.clone(), authority);
    let effects = UpdateEffects {
        collected: scratch.collected,
        updated_base_style: scratch.updated_base_style,
        default_collected: scratch.cleaned,
    };
    publication::install(sheet, scratch);
    sheet.atom_plan = plan;
    Ok((pending, effects))
}
