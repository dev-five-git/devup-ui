use super::w38n_mixin_source::tokens;
use super::w38o_logical_source::{Fixture, Observation, declarations, fixture};
use super::*;
use crate::extract_style::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};

pub(super) type Declaration = (&'static str, &'static str, u8);
pub(super) const R2: Declaration = ("color", "red", 2);
pub(super) const B3: Declaration = ("color", "blue", 3);
pub(super) const K1: Declaration = ("background", "black", 1);
pub(super) const W1: Declaration = ("background", "white", 1);
pub(super) const INVENTORY: &[Declaration] = &[R2, B3, K1];
pub(super) const CONSTRUCTION: &[&str] = &["left", "right"];

pub(super) struct Expected<'a> {
    pub inventory: &'a [Declaration],
    pub selected: &'a [Declaration],
    pub trace: &'a [&'a str],
}

fn authored_inventory(input: &[Declaration]) -> Vec<ExtractStyleValue> {
    let mut inventory: Vec<_> = input
        .iter()
        .map(|&(property, value, order)| {
            ExtractStyleValue::Static(ExtractStaticStyle {
                level: 0,
                selector: None,
                style_order: Some(order),
                layer: None,
                theme_token_resolution: ThemeTokenResolution::CssVariable,
                ..ExtractStaticStyle::new(property, value, 0, None)
            })
        })
        .collect();
    inventory.sort_unstable();
    inventory
}

pub(super) fn verify(actual: &Observation, expected: Expected<'_>) {
    let inventory = authored_inventory(expected.inventory);
    let mut actual_inventory: Vec<_> = actual.output.styles.iter().cloned().collect();
    actual_inventory.sort_unstable();
    assert_eq!(actual_inventory, inventory);

    // Sorting preserves multiplicity; it does not stand in for stylesheet cascade order.
    let mut selected_tokens: Vec<_> = expected
        .selected
        .iter()
        .map(|(property, value, order)| format!("{property}-0-{value}--{order}-a"))
        .collect();
    selected_tokens.sort_unstable();
    let actual_tokens = tokens(&actual.evaluated.element);
    assert_eq!(actual_tokens, selected_tokens);
    assert_eq!(actual.evaluated.trace, serde_json::json!(expected.trace));
    let selected_inventory = authored_inventory(expected.selected);
    assert_eq!(
        declarations(&actual_inventory, &actual_tokens),
        selected_inventory,
    );

    let literal_body = expected
        .inventory
        .iter()
        .map(|(property, value, order)| format!("css({{styleOrder:{order},{property}:'{value}'}})"))
        .collect::<Vec<_>>()
        .join(",");
    let reference = compile_emotion(&fixture(&Fixture {
        outer_producer: "",
        returned_expression: &format!("[{literal_body}]"),
    }))
    .required("independent saved finite inventory must compile");
    let mut reference_inventory: Vec<_> = reference.styles.iter().cloned().collect();
    reference_inventory.sort_unstable();
    assert_eq!(reference_inventory, inventory);
    assert_eq!(
        declarations(&reference_inventory, &actual_tokens),
        selected_inventory,
    );
}
