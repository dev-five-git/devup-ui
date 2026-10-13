use super::{allocation, original, require};
use crate::counter_kernel::{Candidate, Lineage, VariableLineage, capture, error::KernelError};
use crate::{
    counter_evidence::{EmissionWitness, ExpansionProof, Materialization},
    emission_seed::{EmissionContext, EmissionInput, EmissionSeed, NumericSite},
    theme::Theme,
};
use css::{allocation_input::CapturedDelivery, sparse_site::SourceFile};
use extractor::extract_style::{
    CounterProducerError, ExtractDynamicStyle, ExtractKeyframes, ProducedAllocation,
    ProducedDynamic, ProducerPolicy,
    compiler_receipts::{ProducedData, ReceiptBatchView as View, ReceiptId, ReceiptWitness},
    extract_static_style::ExtractStaticStyle,
    extract_style_value::ExtractStyleValue as Value,
};

/// Borrowed real acquisition paired with the independently retained operand.
pub(in crate::counter_kernel) enum Acquisition<'a> {
    Static(&'a ExtractStaticStyle, &'a ProducedAllocation),
    Dynamic(&'a ExtractDynamicStyle, &'a ProducedDynamic),
    Keyframes(&'a ExtractKeyframes, &'a ProducedAllocation),
}

impl Acquisition<'_> {
    const fn policy(&self) -> ProducerPolicy {
        match self {
            Self::Static(style, _) => style.producer_policy(),
            Self::Dynamic(style, _) => style.producer_policy(),
            Self::Keyframes(style, _) => style.producer_policy(),
        }
    }
}

pub(in crate::counter_kernel) fn placement(raw: &str, naming: Option<&str>) -> EmissionContext {
    let (bucket, hoisted) = naming.map_or_else(
        || (String::new(), false),
        |name| {
            let (bucket, _, hoisted) = crate::counter_kernel::prepare::route(name);
            (bucket, hoisted)
        },
    );
    EmissionContext {
        source_file: raw.into(),
        bucket,
        single_css: naming.is_none(),
        hoisted,
    }
}

pub(in crate::counter_kernel) fn static_core(s: &ExtractStaticStyle) -> impl PartialEq + '_ {
    (
        s.property(),
        s.value(),
        s.level(),
        s.selector(),
        s.layer(),
        s.theme_token_resolution,
        s.naming,
        s.producer_policy(),
    )
}

pub(in crate::counter_kernel) fn dynamic_core(s: &ExtractDynamicStyle) -> impl PartialEq + '_ {
    (
        s.property(),
        s.level(),
        s.selector(),
        s.layer(),
        s.important(),
        s.naming(),
        s.site(),
        s.producer_policy(),
    )
}

pub(in crate::counter_kernel) fn acquired<'a>(
    value: &'a Value,
    produced: &'a ProducedData,
) -> Result<Acquisition<'a>, KernelError> {
    match (value, produced) {
        (Value::Static(s), ProducedData::Allocation(r)) => Ok(Acquisition::Static(s, r)),
        (Value::Dynamic(s), ProducedData::Dynamic(r)) => Ok(Acquisition::Dynamic(s, r)),
        (Value::Keyframes(s), ProducedData::Allocation(r)) => Ok(Acquisition::Keyframes(s, r)),
        (
            Value::Static(_)
            | Value::Dynamic(_)
            | Value::Keyframes(_)
            | Value::Typography(_)
            | Value::Css(_)
            | Value::Import(_)
            | Value::FontFace(_),
            ProducedData::Allocation(_) | ProducedData::Dynamic(_),
        ) => Err(KernelError::Authority),
    }
}

pub(in crate::counter_kernel) fn witness<'a>(
    v: &'a View<'_>,
    id: ReceiptId,
) -> Result<&'a ReceiptWitness, KernelError> {
    v.witnesses
        .iter()
        .find(|w| w.id == id)
        .ok_or(KernelError::Authority)
}

/// Check genuine producer originals before discarding them during evidence conversion.
pub(in crate::counter_kernel) fn producer(
    acquisition: Acquisition<'_>,
    theme: &Theme,
    placement: &EmissionContext,
) -> Result<(Candidate, CapturedDelivery), KernelError> {
    let parent = original(acquisition.policy())?;
    require(acquisition.class().original == parent)?;
    match &acquisition {
        Acquisition::Dynamic(style, receipt) => {
            require(
                receipt.identifier == style.identifier()
                    && receipt
                        .variable_allocation
                        .as_ref()
                        .is_none_or(|variable| variable.original == parent),
            )?;
        }
        Acquisition::Static(_, _) | Acquisition::Keyframes(_, _) => {}
    }
    apply(acquisition, theme, placement)
}

/// Independently capture an operand without reacquiring or inventing alias receipts.
/// Callers must link the resulting candidate against independently captured authority.
pub(in crate::counter_kernel) fn apply(
    acquisition: Acquisition<'_>,
    theme: &Theme,
    placement: &EmissionContext,
) -> Result<(Candidate, CapturedDelivery), KernelError> {
    let parent = original(acquisition.policy())?;
    let receipt = acquisition.class();
    let (body, children, variable) = match acquisition {
        Acquisition::Static(style, _) => {
            let declaration = capture::declaration(style, theme)?;
            let body = if style.property == "typography" {
                EmissionInput::Typography(declaration)
            } else {
                EmissionInput::Static(declaration)
            };
            (body, Vec::new(), None)
        }
        Acquisition::Dynamic(style, acquired) => {
            require(
                acquired.site.as_ref() == style.site() && acquired.important == style.important(),
            )?;
            let site = style
                .site()
                .map(|site| match site.file {
                    SourceFile::D9(source) => Ok(NumericSite {
                        source,
                        at: site.at,
                        role: site.role,
                    }),
                    SourceFile::Unnumbered(_) => {
                        Err(KernelError::Producer(CounterProducerError::UnnumberedSite))
                    }
                })
                .transpose()?;
            let variable = match (&site, &acquired.variable_allocation) {
                (Some(_), None) => None,
                (None, Some(variable)) => Some(VariableLineage {
                    original: parent,
                    evidence: allocation(variable.clone()),
                }),
                (Some(_), Some(_)) | (None, None) => return Err(KernelError::Authority),
            };
            (
                EmissionInput::Dynamic {
                    declaration: capture::dynamic(style),
                    variable: acquired.variable.clone(),
                    site,
                    important: style.important(),
                },
                Vec::new(),
                variable,
            )
        }
        Acquisition::Keyframes(frames, _) => {
            let mut children = Vec::new();
            let steps = frames
                .keyframes
                .iter()
                .map(|(step, members)| {
                    let declarations = members
                        .iter()
                        .map(|member| {
                            children.push(original(member.producer_policy())?);
                            capture::declaration(member, theme)
                        })
                        .collect::<Result<Vec<_>, KernelError>>()?;
                    Ok((step.clone(), declarations))
                })
                .collect::<Result<_, KernelError>>()?;
            (EmissionInput::Keyframes { steps }, children, None)
        }
    };
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
        allocation(receipt.clone()),
        EmissionWitness {
            seed,
            name,
            expansion,
            materialization: Materialization::Complete,
        },
    )?;
    let lineage = Lineage {
        parent,
        children,
        variable,
    };
    Ok((Candidate { proof, lineage }, delivery))
}
