use super::{Candidate, FrozenAuthority, legacy};
use crate::{
    counter_evidence::{Materialization, ReplayError},
    emission_seed::{EmissionInput, NumericSite},
};
use css::{
    Site,
    allocation_input::{AllocationContext, AllocationFile, LegacyInput, NameMode},
    sparse_site::SourceFile,
};

pub(super) fn check(candidate: &Candidate, authority: &FrozenAuthority) -> Result<(), ReplayError> {
    let emission = &candidate.proof.emission;
    let placement = &emission.seed.placement;
    let lineage = &candidate.lineage;
    if !authority.placements.contains(placement)
        || !authority.originals.contains_key(&placement.source_file)
        || !authority
            .originals
            .values()
            .any(|original| *original == lineage.parent)
    {
        return Err(ReplayError::Allocation);
    }
    let input = legacy::input(&emission.seed.body, authority.config.mode);
    let shared = placement.single_css
        || matches!(&input, LegacyInput::Declaration(declaration) if declaration.order == Some(0));
    let delivery = if shared {
        None
    } else {
        let delivery = authority
            .delivery(placement)
            .ok_or(ReplayError::Allocation)?;
        if !authority.files.contains_key(&delivery.canonical) {
            return Err(ReplayError::Allocation);
        }
        let mut delivery = delivery.clone();
        delivery.hoisted = authority.config.mode == NameMode::AtomHoist
            && matches!(&input, LegacyInput::Declaration(_))
            && delivery.hoisted;
        Some(delivery)
    };
    let expected_bucket = if placement.single_css {
        ""
    } else {
        authority
            .delivery(placement)
            .ok_or(ReplayError::Allocation)?
            .canonical
            .as_str()
    };
    let expected_hoisted = authority.config.mode == NameMode::AtomHoist
        && !placement.single_css
        && matches!(&input, LegacyInput::Declaration(_))
        && !matches!(&input, LegacyInput::Declaration(declaration) if declaration.order == Some(0))
        && authority
            .delivery(placement)
            .is_some_and(|delivery| delivery.hoisted);
    if placement.bucket != expected_bucket || placement.hoisted != expected_hoisted {
        return Err(ReplayError::Allocation);
    }
    let context = AllocationContext {
        config: authority.config.clone(),
        file: (!shared).then_some(AllocationFile::Original(lineage.parent)),
        delivery,
    };
    authority.envelope(&candidate.proof.allocation, &input, &context)?;
    match &emission.seed.body {
        EmissionInput::Dynamic {
            declaration,
            variable,
            site,
            ..
        } => {
            if !lineage.children.is_empty() {
                return Err(ReplayError::Allocation);
            }
            match (site, &lineage.variable) {
                (Some(site), None) => {
                    if !authority
                        .originals
                        .values()
                        .any(|original| *original == site.source)
                        || site_name(site, &authority.config.prefix) != *variable
                    {
                        return Err(ReplayError::Allocation);
                    }
                }
                (None, Some(variable_evidence)) => {
                    if variable_evidence.original != lineage.parent
                        || !authority
                            .originals
                            .values()
                            .any(|original| *original == variable_evidence.original)
                    {
                        return Err(ReplayError::Allocation);
                    }
                    let input = legacy::variable(declaration, authority.config.mode);
                    let context = AllocationContext {
                        config: authority.config.clone(),
                        file: None,
                        delivery: None,
                    };
                    authority.envelope(&variable_evidence.evidence, &input, &context)?;
                    if variable_evidence.evidence.allocation.name != *variable {
                        return Err(ReplayError::Allocation);
                    }
                }
                (Some(_), Some(_)) | (None, None) => return Err(ReplayError::Allocation),
            }
        }
        EmissionInput::Keyframes { steps } => {
            if lineage.variable.is_some()
                || lineage.children.len() != steps.values().map(Vec::len).sum::<usize>()
                || lineage.children.iter().any(|child| {
                    !authority
                        .originals
                        .values()
                        .any(|original| original == child)
                })
            {
                return Err(ReplayError::Allocation);
            }
        }
        EmissionInput::Static(declaration) => {
            if declaration.property == "typography"
                || !lineage.children.is_empty()
                || lineage.variable.is_some()
            {
                return Err(ReplayError::Expansion);
            }
        }
        EmissionInput::Typography(declaration) => {
            if declaration.property != "typography"
                || !lineage.children.is_empty()
                || lineage.variable.is_some()
            {
                return Err(ReplayError::Expansion);
            }
        }
    }
    if let Materialization::AfterGlobalCleanup { source, bucket } = &emission.materialization
        && !authority
            .cleanups
            .iter()
            .any(|cleanup| &cleanup.source == source && &cleanup.bucket == bucket)
    {
        return Err(ReplayError::Cleanup);
    }
    candidate.proof.materialized()?;
    Ok(())
}

pub(super) fn site_name(site: &NumericSite, prefix: &str) -> String {
    Site {
        file: SourceFile::D9(site.source),
        at: site.at,
        role: site.role,
    }
    .variable_name(prefix)
}
