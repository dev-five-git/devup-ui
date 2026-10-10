use super::BatchPhase;
use crate::{
    StyleSheet,
    counter_evidence::{CounterEvidence, RecordFootprint, ReplayError},
};
use css::style_selector::StyleSelector;
use rustc_hash::FxHashSet;
use std::collections::BTreeSet;

pub(super) fn expected(
    evidence: &CounterEvidence,
    phase: &BatchPhase,
) -> Result<Vec<RecordFootprint>, ReplayError> {
    let mut result = Vec::new();
    for proof in evidence
        .counters
        .values()
        .flat_map(|slots| slots.values().flatten())
        .chain(&evidence.baseline)
    {
        for record in proof.materialized()? {
            if !result.contains(record) {
                result.push(record.clone());
            }
        }
    }
    for record in &evidence.authored {
        if let RecordFootprint::Property { record, .. } = record
            && record.owner_reset
        {
            return Err(ReplayError::Expansion);
        }
        if !result.contains(record) {
            result.push(record.clone());
        }
    }
    let owners = match phase {
        BatchPhase::Fresh => registrations(&result),
        BatchPhase::Retained(owners) => {
            if !owners.is_subset(&registrations(&result))
                || result.iter().any(|record| {
                    matches!(record,
                RecordFootprint::GlobalCssOwner { source } if !owners.contains(source))
                })
            {
                return Err(ReplayError::Expansion);
            }
            owners.clone()
        }
    };
    result.retain(|record| !matches!(record, RecordFootprint::GlobalCssOwner { .. }));
    result.extend(
        owners
            .into_iter()
            .map(|source| RecordFootprint::GlobalCssOwner { source }),
    );
    for (index, record) in result.iter().enumerate() {
        if let RecordFootprint::Keyframes { bucket, name, .. } = record
            && result[..index].iter().any(|prior| matches!(prior, RecordFootprint::Keyframes { bucket: b, name: n, .. } if b == bucket && n == name))
        { return Err(ReplayError::Expansion); }
    }
    Ok(result)
}

pub(super) fn registrations(records: &[RecordFootprint]) -> BTreeSet<String> {
    records
        .iter()
        .filter_map(|footprint| match footprint {
            RecordFootprint::Property { record, .. } => match &record.selector {
                Some(
                    StyleSelector::Global(_, owner)
                    | StyleSelector::At {
                        file: Some(owner), ..
                    },
                ) => Some(owner.clone()),
                Some(StyleSelector::Selector(_) | StyleSelector::At { file: None, .. }) | None => {
                    None
                }
            },
            RecordFootprint::Css { source, .. }
            | RecordFootprint::Import { source, .. }
            | RecordFootprint::FontFace { source, .. }
            | RecordFootprint::GlobalCssOwner { source } => Some(source.clone()),
            RecordFootprint::Keyframes { .. } => None,
        })
        .collect()
}

pub(super) fn capture(sheet: &StyleSheet) -> Vec<RecordFootprint> {
    let mut result = Vec::new();
    for (bucket, orders) in &sheet.properties {
        for (order, levels) in orders {
            for (level, properties) in levels {
                result.extend(properties.iter().map(|record| RecordFootprint::Property {
                    bucket: bucket.clone(),
                    order: *order,
                    level: *level,
                    record: record.clone(),
                }));
            }
        }
    }
    for (bucket, frames) in &sheet.keyframes {
        result.extend(frames.iter().map(|(name, steps)| {
            RecordFootprint::Keyframes {
                bucket: bucket.clone(),
                name: name.clone(),
                steps: steps
                    .iter()
                    .map(|(step, members)| (step.clone(), members.clone()))
                    .collect(),
            }
        }));
    }
    for (source, css) in &sheet.css {
        result.extend(css.iter().map(|css| RecordFootprint::Css {
            source: source.clone(),
            css: css.css.clone(),
        }));
    }
    for (source, urls) in &sheet.imports {
        result.extend(urls.iter().map(|url| RecordFootprint::Import {
            source: source.clone(),
            url: url.clone(),
        }));
    }
    for (source, fonts) in &sheet.font_faces {
        result.extend(fonts.iter().map(|properties| RecordFootprint::FontFace {
            source: source.clone(),
            properties: properties.clone(),
        }));
    }
    result.extend(
        sheet
            .global_css_files
            .iter()
            .map(|source| RecordFootprint::GlobalCssOwner {
                source: source.clone(),
            }),
    );
    result
}

pub(super) fn coverage(
    sheet: &StyleSheet,
    expected: &[RecordFootprint],
) -> Result<(), ReplayError> {
    let actual: FxHashSet<_> = capture(sheet).into_iter().collect();
    let expected: FxHashSet<_> = expected.iter().cloned().collect();
    if actual != expected {
        return Err(ReplayError::Expansion);
    }
    Ok(())
}
