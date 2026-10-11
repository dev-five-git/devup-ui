use crate::extract_style::{
    ExtractKeyframes,
    extract_static_style::{ExtractStaticStyle, ThemeTokenResolution},
};
use css::style_selector::{AtRule, AtRuleKind, StyleSelector};
use std::{collections::BTreeMap, hash::Hash};

#[derive(Hash)]
enum ReferenceSelector<'a> {
    At {
        kind: AtRuleKind,
        query: &'a str,
        selector: &'a Option<String>,
        outer: &'a Vec<AtRule>,
        file: &'a Option<String>,
    },
    Selector(&'a str),
    Global(&'a str, &'a str),
}

#[derive(Hash)]
pub(super) struct ReferenceMember<'a> {
    property: &'a str,
    value: &'a str,
    level: u8,
    selector: Option<ReferenceSelector<'a>>,
    style_order: Option<u8>,
    layer: &'a Option<String>,
    resolution: ThemeTokenResolution,
}

pub(super) fn reference(style: &ExtractStaticStyle) -> ReferenceMember<'_> {
    ReferenceMember {
        property: &style.property,
        value: &style.value,
        level: style.level,
        selector: style.selector.as_ref().map(|selector| match selector {
            StyleSelector::At {
                kind,
                query,
                selector,
                outer,
                file,
            } => ReferenceSelector::At {
                kind: *kind,
                query,
                selector,
                outer,
                file,
            },
            StyleSelector::Selector(text) => ReferenceSelector::Selector(text),
            StyleSelector::Global(text, file) => ReferenceSelector::Global(text, file),
        }),
        style_order: style.style_order,
        layer: &style.layer,
        resolution: style.theme_token_resolution,
    }
}

pub(super) fn reference_map(
    frames: &ExtractKeyframes,
) -> BTreeMap<String, Vec<ReferenceMember<'_>>> {
    frames
        .keyframes
        .iter()
        .map(|(step, styles)| (step.clone(), styles.iter().map(reference).collect()))
        .collect()
}
