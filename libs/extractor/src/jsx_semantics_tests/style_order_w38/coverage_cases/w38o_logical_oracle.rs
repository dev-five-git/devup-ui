use super::w38o_logical_source::{Fixture, Observation, declarations, fixture};
use super::*;
use crate::extract_style::extract_static_style::ExtractStaticStyle;

pub(super) struct Expected<'a> {
    pub inventory: &'a [&'a str],
    pub selected: &'a [&'a str],
    pub trace: &'a [&'a str],
    pub order: Option<u8>,
}

fn token(color: &str, order: Option<u8>) -> String {
    format!("color-0-{color}--{}-a", order.unwrap_or(255))
}

fn authored_inventory(colors: &[&str], order: Option<u8>) -> Vec<ExtractStyleValue> {
    let mut inventory: Vec<_> = colors
        .iter()
        .map(|color| {
            let mut style = ExtractStaticStyle::new("color", color, 0, None);
            style.style_order = order;
            ExtractStyleValue::Static(style)
        })
        .collect();
    inventory.sort_unstable();
    inventory
}

pub(super) fn verify(actual: &Observation, expected: Expected<'_>) {
    let inventory = authored_inventory(expected.inventory, expected.order);
    let mut actual_inventory: Vec<_> = actual.output.styles.iter().cloned().collect();
    actual_inventory.sort_unstable();
    assert_eq!(actual_inventory, inventory);
    let selected: Vec<_> = expected
        .selected
        .iter()
        .map(|color| token(color, expected.order))
        .collect();
    let mut selected_tokens: Vec<_> = selected.iter().map(String::as_str).collect();
    selected_tokens.sort_unstable();
    let actual_tokens = super::w38n_mixin_source::tokens(&actual.evaluated.element);
    assert_eq!(actual_tokens, selected_tokens);
    assert_eq!(actual.evaluated.trace, serde_json::json!(expected.trace));
    let selected_inventory = authored_inventory(expected.selected, expected.order);
    assert_eq!(
        declarations(&actual_inventory, &actual_tokens),
        selected_inventory,
    );
    let literal_body = expected
        .inventory
        .iter()
        .map(|color| {
            let order = expected
                .order
                .map_or_else(String::new, |value| format!("styleOrder:{value},"));
            format!("css({{{order}color:'{color}'}})")
        })
        .collect::<Vec<_>>()
        .join(",");
    let literal = compile_emotion(&fixture(&Fixture {
        outer_producer: "",
        returned_expression: &format!("[{literal_body}]"),
    }))
    .required("independent literal inventory must compile");
    let mut literal_inventory: Vec<_> = literal.styles.iter().cloned().collect();
    literal_inventory.sort_unstable();
    assert_eq!(literal_inventory, inventory);
    assert_eq!(
        declarations(&literal_inventory, &actual_tokens),
        selected_inventory
    );
}
