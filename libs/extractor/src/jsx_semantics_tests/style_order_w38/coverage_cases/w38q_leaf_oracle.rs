use super::w38o_logical_source::{Observation, declarations};
use super::w38q_leaf_source::reference;
use super::*;
use crate::extract_style::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};

pub(super) const RED: &str = "color-0-red--2-a";
pub(super) const BLACK: &str = "background-0-black--1-a";

pub(super) struct Expected<'a> {
    pub saved: bool,
    pub classes: &'a str,
    pub external: &'a str,
    pub trace: &'a [&'a str],
}

fn inventory(saved: bool) -> Vec<ExtractStyleValue> {
    let red = ExtractStyleValue::Static(ExtractStaticStyle {
        property: "color".into(),
        value: "red".into(),
        level: 0,
        selector: None,
        style_order: Some(2),
        layer: None,
        theme_token_resolution: ThemeTokenResolution::CssVariable,
    });
    let mut values = vec![red];
    if saved {
        values.push(ExtractStyleValue::Static(ExtractStaticStyle {
            property: "background".into(),
            value: "black".into(),
            level: 0,
            selector: None,
            style_order: Some(1),
            layer: None,
            theme_token_resolution: ThemeTokenResolution::CssVariable,
        }));
    }
    values.sort_unstable();
    values
}

pub(super) fn verify(actual: &Observation, expected: Expected<'_>) {
    // Full typed equality precedes any filtering, including the hidden resolution field.
    let authored = inventory(expected.saved);
    let mut actual_inventory: Vec<_> = actual.output.styles.iter().cloned().collect();
    actual_inventory.sort_unstable();
    assert_eq!(actual_inventory, authored);
    assert_eq!(
        actual.evaluated.element,
        serde_json::json!(expected.classes)
    );
    assert_eq!(actual.evaluated.trace, serde_json::json!(expected.trace));
    let classes = actual.evaluated.element.as_str().required("class string");
    let tokens: Vec<_> = classes.split_whitespace().collect();
    let external: Vec<_> = tokens
        .iter()
        .copied()
        .filter(|token| *token != RED && *token != BLACK)
        .collect();
    let authored_external: Vec<_> = expected.external.split_whitespace().collect();
    assert_eq!(external, authored_external);
    assert_eq!(tokens.iter().filter(|token| **token == RED).count(), 1);
    assert_eq!(
        tokens.iter().filter(|token| **token == BLACK).count(),
        usize::from(expected.saved)
    );
    assert_eq!(declarations(&actual_inventory, &tokens), authored);
    // This reference is compile-only: it has no render or runtime evaluator call.
    let reference = reference(expected.saved);
    let mut reference_inventory: Vec<_> = reference.styles.iter().cloned().collect();
    reference_inventory.sort_unstable();
    assert_eq!(reference_inventory, authored);
    assert_eq!(declarations(&reference_inventory, &tokens), authored);
}
