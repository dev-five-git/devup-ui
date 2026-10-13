use super::{
    Candidate, FrozenAuthority, LinkedBatch, authority,
    error::KernelError,
    live::receipts::{self, AttemptCheck, Observation},
    phase,
    prepare::Input,
    production::{self, Generated},
    records,
};
use crate::{
    StyleSheet,
    counter_evidence::{CounterEvidence, RecordFootprint},
    emission_seed::EmissionContext,
};
use extractor::extract_style::compiler_receipts::WitnessDisposition;
use extractor::extract_style::{
    ExtractDynamicStyle, extract_style_value::ExtractStyleValue as Value,
};
use production::{application as app, require, same};

pub(super) struct Traversal {
    pub(super) authority: FrozenAuthority,
    pub(super) operations: Vec<RecordFootprint>,
    candidates: Vec<Candidate>,
    pub(super) check: Option<AttemptCheck>,
}

impl Traversal {
    pub(super) fn run(
        sheet: &StyleSheet,
        styles: Input<'_>,
        p: &EmissionContext,
    ) -> Result<Self, KernelError> {
        let mut t = Self {
            candidates: Vec::new(),
            authority: FrozenAuthority::live(),
            operations: Vec::new(),
            check: match styles {
                Input::Constructor(_) => None,
                Input::Compiled(view) => Some(receipts::check(view, &sheet.theme)?),
            },
        };
        let ordered: Vec<_> = match styles {
            Input::Constructor(styles) => {
                let mut ordered: Vec<_> = styles.iter().collect();
                ordered.sort_unstable();
                ordered
            }
            Input::Compiled(view) => view.styles.iter().collect(),
        };
        for style in ordered {
            let generated = match style {
                Value::Static(style) => Generated::Static(style),
                Value::Dynamic(style) => Generated::Dynamic(style),
                Value::Keyframes(style) => Generated::Keyframes(style),
                Value::Css(style) => {
                    t.authored(RecordFootprint::Css {
                        source: style.file.clone(),
                        css: style.css.clone(),
                    });
                    continue;
                }
                Value::Import(style) => {
                    t.authored(RecordFootprint::Import {
                        source: style.file.clone(),
                        url: style.url.clone(),
                    });
                    continue;
                }
                Value::FontFace(style) => {
                    t.authored(RecordFootprint::FontFace {
                        source: style.file.clone(),
                        properties: style.properties.clone(),
                    });
                    continue;
                }
                Value::Typography(_) => continue,
            };
            let candidates = match styles {
                Input::Constructor(_) => {
                    vec![production::candidate(generated, &sheet.theme, p)?]
                }
                Input::Compiled(view) => {
                    let graph = view.associations;
                    let final_ = graph
                        .emissions
                        .iter()
                        .find(|f| same(&f.operand, style))
                        .ok_or(KernelError::Authority)?;
                    let receipt = receipts::witness(view, final_.receipt)?;
                    require(receipt.disposition == WitnessDisposition::Emission)?;
                    let check = t.check.as_mut().ok_or(KernelError::Authority)?;
                    let real = check.producer(receipt, &sheet.theme, p)?;
                    let expansion = real.0.proof.emission.expansion.clone();
                    let mut aliases = vec![real];
                    for alias in graph
                        .operands
                        .iter()
                        .filter(|a| a.survivor == final_.survivor)
                    {
                        let (c, d) = app::apply(
                            app::acquired(&alias.operand, &receipt.produced)?,
                            &sheet.theme,
                            p,
                        )?;
                        require(c.proof.emission.expansion == expansion)?;
                        check.keep(
                            Observation {
                                operand: alias.operand.clone(),
                                produced: receipt.produced.clone(),
                                expected: c.clone(),
                                genuine: false,
                            },
                            d.clone(),
                        )?;
                        aliases.push((c, d));
                    }
                    aliases
                }
            };
            let first = candidates.first().ok_or(KernelError::Authority)?;
            for record in first.0.proof.materialized()? {
                phase::retire(&mut t.candidates, &mut t.authority.authored, record);
                t.operations.push(record.clone());
            }
            for (c, d) in candidates {
                t.authority.capture(&c, d)?;
                if !t.candidates.contains(&c) {
                    t.candidates.push(c);
                }
            }
        }
        t.operations.extend(
            records::registrations(&t.operations)
                .into_iter()
                .map(|source| RecordFootprint::GlobalCssOwner { source }),
        );
        t.authority.classes = authority::classes();
        Ok(t)
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

pub(super) fn order_only(a: &Value, b: &Value) -> bool {
    production::same_request(a, b, true)
}

pub(super) fn identifier_only(a: &ExtractDynamicStyle, b: &ExtractDynamicStyle) -> bool {
    app::dynamic_core(a) == app::dynamic_core(b) && a.style_order() == b.style_order()
}
