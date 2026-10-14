use std::collections::{BTreeMap, BTreeSet};

use css::{
    Naming,
    content_name::{AtomContent, ContentName},
};
use sheet::{StyleSheet, StyleSheetProperty};

use crate::cache_descriptor::{Cursor, name_matches};

type FrameSteps = Vec<(String, Vec<(String, String)>)>;

fn frames(descriptor: &[u8]) -> Option<FrameSteps> {
    let mut cursor = Cursor(descriptor);
    if cursor.take(2)? != [1, 3] {
        return None;
    }
    let count = cursor.count()?;
    let mut steps = Vec::new();
    for _ in 0..count {
        let step = cursor.text()?.to_string();
        let count = cursor.count()?;
        let mut properties = Vec::new();
        for _ in 0..count {
            properties.push((cursor.text()?.to_string(), cursor.text()?.to_string()));
        }
        steps.push((step, properties));
    }
    cursor.0.is_empty().then_some(steps)
}

pub(crate) fn validate_frames(sheet: &StyleSheet) -> Result<(), String> {
    for frames_map in sheet.keyframes.values() {
        for (name, steps) in frames_map {
            if let Some(claim) = sheet.names.get(name) {
                let valid = frames(&claim.descriptor).is_some_and(|decoded| {
                    let content = ContentName::keyframes(&decoded);
                    name_matches(&content, name)
                        && decoded.into_iter().collect::<BTreeMap<_, _>>() == *steps
                });
                if !valid {
                    return Err(format!(
                        "cached keyframes `{name}` have a corrupt exact frame proof"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn declarations(value: &str) -> Option<BTreeSet<(u8, String, String)>> {
    let bytes = crate::cache_descriptor::hex_bytes(value.strip_prefix('t')?)?;
    let mut cursor = Cursor(&bytes);
    let count = cursor.count()?;
    let mut result = BTreeSet::new();
    for _ in 0..count {
        result.insert((
            cursor.byte()?,
            cursor.text()?.to_string(),
            cursor.text()?.to_string(),
        ));
    }
    (cursor.0.is_empty() && result.len() == count).then_some(result)
}

pub(crate) fn typography(
    property: &StyleSheetProperty,
    group: (&BTreeMap<u8, rustc_hash::FxHashSet<StyleSheetProperty>>, u8),
    descriptor: &[u8],
) -> Option<()> {
    let mut cursor = Cursor(descriptor);
    if cursor.take(2)? != [1, 1] {
        return None;
    }
    let naming = match cursor.byte()? {
        0 => Naming::Own,
        1 => Naming::Risky,
        _ => return None,
    };
    if cursor.text()? != "typography" || cursor.byte()? != 1 {
        return None;
    }
    let value = cursor.text()?;
    let level = cursor.byte()?;
    let order = cursor.byte()?;
    if order != group.1 {
        return None;
    }
    let content = AtomContent {
        property: "typography",
        value: Some(value),
        naming,
        level,
        order,
        selector: property.selector.as_ref(),
        layer: property.layer.as_deref()?.strip_suffix(".t"),
        dynamic: false,
    }
    .content();
    if content.descriptor != descriptor || !name_matches(&content, &property.class_name) {
        return None;
    }
    let actual = group
        .0
        .iter()
        .flat_map(|(level, properties)| {
            properties
                .iter()
                .filter(|other| other.class_name == property.class_name && other.typography)
                .map(|other| (*level, other.property.clone(), other.value.clone()))
        })
        .collect();
    (declarations(value)? == actual).then_some(())
}

pub(crate) fn counter_typography(
    property: &StyleSheetProperty,
    group: (&BTreeMap<u8, rustc_hash::FxHashSet<StyleSheetProperty>>, u8),
    key: &str,
) -> Option<()> {
    let lossless = String::from_utf8(crate::cache_descriptor::hex_bytes(key)?).ok()?;
    let tail = lossless.strip_prefix("typography-v")?;
    let value = tail.split('-').next()?;
    let suffix = tail.strip_prefix(value)?;
    let level = match suffix.strip_prefix("-l") {
        Some(level) => level.split('-').next()?.parse().ok()?,
        None => 0,
    };
    let content = AtomContent {
        property: "typography",
        value: Some(value),
        naming: Naming::Own,
        level,
        order: group.1,
        selector: property.selector.as_ref(),
        layer: property.layer.as_deref()?.strip_suffix(".t"),
        dynamic: false,
    }
    .content();
    let actual = group
        .0
        .iter()
        .flat_map(|(level, properties)| {
            properties
                .iter()
                .filter(|other| other.class_name == property.class_name && other.typography)
                .map(|other| (*level, other.property.clone(), other.value.clone()))
        })
        .collect();
    (content.lossless == lossless && declarations(value)? == actual).then_some(())
}
