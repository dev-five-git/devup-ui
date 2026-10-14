use super::LinkedBatch;
use crate::{StyleSheet, StyleSheetCss, counter_evidence::RecordFootprint};
use css::allocation_input::NameMode;

pub(super) fn clone_sheet(sheet: &StyleSheet) -> StyleSheet {
    StyleSheet {
        properties: sheet.properties.clone(),
        css: sheet
            .css
            .iter()
            .map(|(source, rules)| {
                (
                    source.clone(),
                    rules
                        .iter()
                        .map(|rule| StyleSheetCss {
                            css: rule.css.clone(),
                        })
                        .collect(),
                )
            })
            .collect(),
        keyframes: sheet.keyframes.clone(),
        global_css_files: sheet.global_css_files.clone(),
        imports: sheet.imports.clone(),
        font_faces: sheet.font_faces.clone(),
        ..StyleSheet::default()
    }
}

pub(super) fn apply(sheet: &mut StyleSheet, incoming: &LinkedBatch) -> (bool, bool) {
    apply_ordered(sheet, &incoming.records, incoming.config.mode)
}

pub(super) fn apply_ordered(
    sheet: &mut StyleSheet,
    operations: &[RecordFootprint],
    mode: NameMode,
) -> (bool, bool) {
    let mut collected = false;
    let mut base = false;
    for footprint in operations {
        let added = insert(sheet, footprint);
        match footprint {
            RecordFootprint::Property {
                bucket,
                order,
                record,
                ..
            } => {
                collected |= added;
                base |= added
                    && (*order == 0
                        || (mode == NameMode::AtomHoist
                            && (record.hoisted || bucket.is_empty() || *order != 255)));
            }
            RecordFootprint::Keyframes { bucket, .. } => {
                collected |= added;
                base |= added && mode == NameMode::AtomHoist && bucket.is_empty();
            }
            RecordFootprint::Css { .. } => base |= added,
            RecordFootprint::Import { .. } | RecordFootprint::FontFace { .. } => {
                base |= added && mode == NameMode::AtomHoist;
            }
            RecordFootprint::GlobalCssOwner { .. } => {}
        }
    }
    (collected, base)
}

pub(super) fn insert(sheet: &mut StyleSheet, footprint: &RecordFootprint) -> bool {
    match footprint {
        RecordFootprint::Property {
            bucket,
            order,
            level,
            record,
        } => sheet
            .properties
            .entry(bucket.clone())
            .or_default()
            .entry(*order)
            .or_default()
            .entry(*level)
            .or_default()
            .insert(record.clone()),
        RecordFootprint::Keyframes {
            bucket,
            name,
            steps,
        } => sheet.add_keyframes_raw(name, steps.iter().cloned().collect(), Some(bucket)),
        RecordFootprint::Css { source, css } => sheet
            .css
            .entry(source.clone())
            .or_default()
            .insert(StyleSheetCss { css: css.clone() }),
        RecordFootprint::Import { source, url } => sheet
            .imports
            .entry(source.clone())
            .or_default()
            .insert(url.clone()),
        RecordFootprint::FontFace { source, properties } => sheet
            .font_faces
            .entry(source.clone())
            .or_default()
            .insert(properties.clone()),
        RecordFootprint::GlobalCssOwner { source } => sheet.global_css_files.insert(source.clone()),
    }
}
