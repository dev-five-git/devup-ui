use super::{
    Cleanup, FrozenAuthority, LinkedBatch,
    authority::{self, Retained},
    error::KernelError,
    live::{KernelEvidence, UpdateEffects, UpdateRequest},
    phase, production, publication, records,
    scratch::{self, ScratchRequest},
};
use crate::{
    StyleSheet,
    counter_evidence::{CounterEvidence, RecordFootprint},
    emission_seed::EmissionContext,
};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use rustc_hash::FxHashSet;

struct Traversal {
    candidates: Vec<super::Candidate>,
    authority: FrozenAuthority,
    operations: Vec<RecordFootprint>,
}

impl Traversal {
    fn run(
        sheet: &StyleSheet,
        styles: &FxHashSet<ExtractStyleValue>,
        placement: &EmissionContext,
    ) -> Result<Self, KernelError> {
        let mut result = Self {
            candidates: Vec::new(),
            authority: FrozenAuthority::live(),
            operations: Vec::new(),
        };
        let mut ordered: Vec<_> = styles.iter().collect();
        ordered.sort_unstable();
        for style in ordered {
            let Some((candidate, delivery)) =
                production::candidate(style, &sheet.theme, placement)?
            else {
                let authored = match style {
                    ExtractStyleValue::Css(style) => Some(RecordFootprint::Css {
                        source: style.file.clone(),
                        css: style.css.clone(),
                    }),
                    ExtractStyleValue::Import(style) => Some(RecordFootprint::Import {
                        source: style.file.clone(),
                        url: style.url.clone(),
                    }),
                    ExtractStyleValue::FontFace(style) => Some(RecordFootprint::FontFace {
                        source: style.file.clone(),
                        properties: style.properties.clone(),
                    }),
                    ExtractStyleValue::Typography(_) => None,
                    ExtractStyleValue::Static(_)
                    | ExtractStyleValue::Dynamic(_)
                    | ExtractStyleValue::Keyframes(_) => return Err(KernelError::Coverage),
                };
                if let Some(record) = authored {
                    result.operations.push(record.clone());
                    if !result.authority.authored.contains(&record) {
                        result.authority.authored.push(record);
                    }
                }
                continue;
            };
            result.authority.capture(&candidate, delivery)?;
            for record in candidate.proof.materialized()? {
                phase::retire(
                    &mut result.candidates,
                    &mut result.authority.authored,
                    record,
                );
                result.operations.push(record.clone());
            }
            if !result.candidates.contains(&candidate) {
                result.candidates.push(candidate);
            }
        }
        result.operations.extend(
            records::registrations(&result.operations)
                .into_iter()
                .map(|source| RecordFootprint::GlobalCssOwner { source }),
        );
        result.authority.classes = authority::classes();
        Ok(result)
    }

    fn linked(&self) -> Result<LinkedBatch, KernelError> {
        let mut evidence = CounterEvidence {
            authored: self.authority.authored.clone(),
            ..CounterEvidence::default()
        };
        for candidate in &self.candidates {
            evidence.insert(candidate.proof.clone())?;
        }
        Ok(LinkedBatch::link_captured_batch(
            &self.candidates,
            &evidence,
            &self.authority,
        )?)
    }
}

pub(super) struct Preparation<'a> {
    pub(super) base: &'a LinkedBatch,
    pub(super) evidence: &'a KernelEvidence,
    pub(super) styles: &'a FxHashSet<ExtractStyleValue>,
    pub(super) request: UpdateRequest<'a>,
}

pub(super) fn prepare(
    sheet: &mut StyleSheet,
    input: Preparation<'_>,
) -> Result<(KernelEvidence, UpdateEffects), KernelError> {
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
    let mut authority = match &input.evidence.retained {
        Some(retained) => retained.authority.refresh()?,
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
    let pending = KernelEvidence {
        retained: Some(Retained {
            candidates: scratch.candidates.clone(),
            evidence: scratch.evidence.clone(),
            authority,
            plan: plan.clone(),
        }),
    };
    let effects = UpdateEffects {
        collected: scratch.collected,
        updated_base_style: scratch.updated_base_style,
        default_collected: scratch.cleaned,
    };
    publication::install(sheet, scratch);
    sheet.atom_plan = plan;
    Ok((pending, effects))
}
