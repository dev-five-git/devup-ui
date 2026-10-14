use super::production::{application::placement, original, require};
use super::{
    Cleanup, FrozenAuthority, LinkedBatch, authority, capture,
    error::KernelError,
    live::{UpdateEffects, UpdateRequest, receipts::AttemptCheck},
    publication, records,
    scratch::{self, ScratchRequest},
    state::CounterState,
    traversal::Traversal,
};
use crate::StyleSheet;
use extractor::extract_style::extract_style_value::ExtractStyleValue as Value;
use extractor::extract_style::{
    compiler_associations::{Graph, InvocationId, ScopeId},
    compiler_receipts::ReceiptBatchView,
};
use rustc_hash::FxHashSet;

#[derive(Clone, Copy)]
pub(super) enum Input<'a> {
    Constructor(&'a FxHashSet<Value>),
    Compiled(&'a ReceiptBatchView<'a>),
}

pub(super) struct Preparation<'a, I> {
    pub(super) base: &'a LinkedBatch,
    pub(super) styles: I,
    pub(super) request: UpdateRequest<'a>,
}

pub(super) fn prepare<'a>(
    sheet: &mut StyleSheet,
    input: Preparation<'a, impl Into<Input<'a>>>,
) -> Result<(CounterState, UpdateEffects, Option<AttemptCheck>), KernelError> {
    css::atom_hoist::freeze_atom_plan();
    let plan = css::atom_hoist::atom_plan();
    if sheet.atom_plan.is_some() && sheet.atom_plan != plan {
        return Err(KernelError::Authority);
    }
    let single_css = input.request.single_css || css::file_map::is_global(input.request.raw_source);
    let placement = placement(
        input.request.raw_source,
        (!single_css).then_some(input.request.raw_source),
    );
    if !placement.single_css {
        let _ordinal = css::file_map::get_file_num_by_filename(&placement.bucket);
    }
    let traversal = Traversal::run(sheet, input.styles.into(), &placement)?;
    let incoming = traversal.linked()?;
    let cleanup = Cleanup {
        source: input.request.raw_source.into(),
        bucket: placement.bucket,
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
    let pending = CounterState::from_captured(scratch.candidates.clone(), authority);
    let effects = UpdateEffects {
        collected: scratch.collected,
        updated_base_style: scratch.updated_base_style,
        default_collected: scratch.cleaned,
    };
    publication::install(sheet, scratch);
    records::coverage(sheet, &linked.records)?;
    sheet.atom_plan = plan;
    Ok((pending, effects, traversal.check))
}

pub(super) fn route(raw: &str) -> (String, bool, bool) {
    let canonical = css::file_map::canonical(raw);
    let hoisted = css::atom_hoist::is_atom_hoist()
        && css::atom_hoist::atom_plan().is_some_and(|plan| plan.contains(&canonical));
    (canonical, css::file_map::is_global(raw), hoisted)
}

type Call<'a> = Result<(&'a str, Option<&'a str>), KernelError>;
pub(super) fn frame(g: &Graph, id: InvocationId, scope: Option<ScopeId>) -> Call<'_> {
    let raw = g
        .invocations
        .iter()
        .find(|i| i.id == id)
        .ok_or(KernelError::Authority)?;
    let route = match scope {
        None => None,
        Some(scope) => g
            .frames
            .iter()
            .find(|f| f.id == scope && f.invocation == id)
            .map(|f| f.route.as_deref())
            .ok_or(KernelError::Authority)?,
    };
    Ok((&raw.raw, route))
}

pub(super) fn references(
    value: &Value,
    retained: &mut std::collections::BTreeMap<String, u32>,
) -> Result<(), KernelError> {
    use Value as V;
    let originals = css::file_map::get_original_ids();
    let mut retain = |id| -> Result<(), KernelError> {
        require(originals.values().any(|actual| *actual == id))?;
        retained.extend(
            originals
                .iter()
                .filter(|(_, actual)| **actual == id)
                .map(|(raw, id)| (raw.clone(), *id)),
        );
        Ok(())
    };
    match value {
        V::Static(s) => retain(original(s.producer_policy())?),
        V::Dynamic(s) => {
            retain(original(s.producer_policy())?)?;
            capture::numeric_site(s.site())?.map_or(Ok(()), |site| retain(site.source))
        }
        V::Keyframes(s) => {
            retain(original(s.producer_policy())?)?;
            for member in s.keyframes.values().flatten() {
                retain(original(member.producer_policy())?)?;
            }
            Ok(())
        }
        V::Typography(_) | V::Css(_) | V::Import(_) | V::FontFace(_) => Ok(()),
    }
}
