use super::{
    Candidate, FrozenAuthority, LinkedBatch, authority,
    error::KernelError,
    phase,
    production::{self, Generated},
    records,
};
use crate::{
    StyleSheet,
    counter_evidence::{CounterEvidence, RecordFootprint},
    emission_seed::EmissionContext,
};
use extractor::extract_style::extract_style_value::ExtractStyleValue;
use rustc_hash::FxHashSet;

pub(super) struct Traversal {
    pub(super) authority: FrozenAuthority,
    pub(super) operations: Vec<RecordFootprint>,
    candidates: Vec<Candidate>,
}

impl Traversal {
    pub(super) fn run(
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
            let generated = match style {
                ExtractStyleValue::Static(style) => Generated::Static(style),
                ExtractStyleValue::Dynamic(style) => Generated::Dynamic(style),
                ExtractStyleValue::Keyframes(style) => Generated::Keyframes(style),
                ExtractStyleValue::Css(style) => {
                    result.authored(RecordFootprint::Css {
                        source: style.file.clone(),
                        css: style.css.clone(),
                    });
                    continue;
                }
                ExtractStyleValue::Import(style) => {
                    result.authored(RecordFootprint::Import {
                        source: style.file.clone(),
                        url: style.url.clone(),
                    });
                    continue;
                }
                ExtractStyleValue::FontFace(style) => {
                    result.authored(RecordFootprint::FontFace {
                        source: style.file.clone(),
                        properties: style.properties.clone(),
                    });
                    continue;
                }
                ExtractStyleValue::Typography(_) => continue,
            };
            let (candidate, delivery) = production::candidate(generated, &sheet.theme, placement)?;
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

    fn authored(&mut self, record: RecordFootprint) {
        self.operations.push(record.clone());
        if !self.authority.authored.contains(&record) {
            self.authority.authored.push(record);
        }
    }

    pub(super) fn linked(&self) -> Result<LinkedBatch, KernelError> {
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
