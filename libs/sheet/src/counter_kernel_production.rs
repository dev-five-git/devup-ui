use super::{Candidate, Lineage, VariableLineage, capture, error::KernelError};
use crate::{
    counter_evidence::{AllocationEvidence, EmissionWitness, ExpansionProof, Materialization},
    emission_seed::{EmissionContext, EmissionInput, EmissionSeed, NumericSite},
    theme::Theme,
};
use css::{allocation_input::CapturedDelivery, sparse_site::SourceFile};
use extractor::extract_style::{
    ExtractDynamicStyle, ExtractKeyframes, ProducedAllocation, ProducerPolicy,
    extract_static_style::ExtractStaticStyle,
};

pub(super) enum Generated<'a> {
    Static(&'a ExtractStaticStyle),
    Dynamic(&'a ExtractDynamicStyle),
    Keyframes(&'a ExtractKeyframes),
}

pub(super) fn allocation(receipt: ProducedAllocation) -> AllocationEvidence {
    AllocationEvidence {
        input: receipt.input,
        context: receipt.context,
        allocation: receipt.allocation,
    }
}

pub(super) fn candidate(
    style: Generated<'_>,
    theme: &Theme,
    placement: &EmissionContext,
) -> Result<(Candidate, CapturedDelivery), KernelError> {
    let filename = (!placement.single_css).then_some(placement.source_file.as_str());
    let (body, receipt, children, variable) = match style {
        Generated::Static(style) => {
            let declaration = capture::declaration(style, theme)?;
            let body = if style.property == "typography" {
                EmissionInput::Typography(declaration)
            } else {
                EmissionInput::Static(declaration)
            };
            (body, style.counter_produce(filename)?, Vec::new(), None)
        }
        Generated::Dynamic(style) => {
            let declaration = capture::dynamic(style);
            match style.producer_policy() {
                ProducerPolicy::CounterOriginal(_) => {}
                ProducerPolicy::Current => {
                    return Err(KernelError::Producer(
                        extractor::extract_style::CounterProducerError::WrongPolicy,
                    ));
                }
            }
            let site = style
                .site()
                .map(|site| match site.file {
                    SourceFile::D9(source) => Ok(NumericSite {
                        source,
                        at: site.at,
                        role: site.role,
                    }),
                    SourceFile::Unnumbered(_) => Err(KernelError::Producer(
                        extractor::extract_style::CounterProducerError::UnnumberedSite,
                    )),
                })
                .transpose()?;
            let receipt = style.counter_produce(filename)?;
            let variable = receipt.variable_allocation.map(|receipt| VariableLineage {
                original: receipt.original,
                evidence: allocation(receipt),
            });
            (
                EmissionInput::Dynamic {
                    declaration,
                    variable: receipt.variable,
                    site,
                    important: receipt.important,
                },
                receipt.class,
                Vec::new(),
                variable,
            )
        }
        Generated::Keyframes(frames) => {
            let mut children = Vec::new();
            let steps = frames
                .keyframes
                .iter()
                .map(|(step, members)| {
                    let declarations =
                        members
                            .iter()
                            .map(|member| {
                                match member.producer_policy() {
                                    ProducerPolicy::CounterOriginal(id) => children.push(id),
                                    ProducerPolicy::Current => return Err(KernelError::Producer(
                                        extractor::extract_style::CounterProducerError::WrongPolicy,
                                    )),
                                }
                                capture::declaration(member, theme)
                            })
                            .collect::<Result<Vec<_>, KernelError>>()?;
                    Ok((step.clone(), declarations))
                })
                .collect::<Result<_, KernelError>>()?;
            (
                EmissionInput::Keyframes { steps },
                frames.counter_produce(filename)?,
                children,
                None,
            )
        }
    };
    let parent = receipt.original;
    let mut placement = placement.clone();
    placement.hoisted &= match &body {
        EmissionInput::Static(declaration)
        | EmissionInput::Typography(declaration)
        | EmissionInput::Dynamic { declaration, .. } => declaration.style_order != Some(0),
        EmissionInput::Keyframes { .. } => false,
    };
    let delivery = CapturedDelivery {
        canonical: placement.bucket.clone(),
        hoisted: placement.hoisted,
    };
    let seed = EmissionSeed { placement, body };
    let name = receipt.allocation.name.clone();
    let expansion = seed.replay(&name)?;
    let proof = ExpansionProof::new(
        allocation(receipt),
        EmissionWitness {
            seed,
            name,
            expansion,
            materialization: Materialization::Complete,
        },
    )?;
    Ok((
        Candidate {
            proof,
            lineage: Lineage {
                parent,
                children,
                variable,
            },
        },
        delivery,
    ))
}
