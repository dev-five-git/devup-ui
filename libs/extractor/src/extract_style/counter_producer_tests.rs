use super::counter_producer_test_support::{
    address, cleanup, produced, reset, scope, static_style,
};
use super::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};
use super::{CounterProducerError, ExtractDynamicStyle, ExtractKeyframes};
use crate::sparse_sites::SiteScope;
use css::{
    Naming,
    allocation_input::{LegacyDeclaration, LegacyInput, NameMode},
    class_map::get_class_map,
};
use rstest::rstest;
use serial_test::serial;
use std::collections::{BTreeMap, HashMap};

#[rstest]
#[case(None, Some("delivery"), "D9-0", "color-0-red--255-a", "pa-a")]
#[case(Some(1), Some("delivery"), "D9-0", "color-0-red--1-a", "pa-a")]
#[case(Some(255), Some("delivery"), "D9-0", "color-0-red--255-a", "pa-a")]
#[case(Some(0), Some("delivery"), "", "color-0-red--0", "pa")]
#[case(None, None, "", "color-0-red--255", "pa")]
#[serial]
fn static_retains_original_when_orders_choose_private_or_shared(
    #[case] order: Option<u8>,
    #[case] filename: Option<&str>,
    #[case] namespace: &str,
    #[case] key: &str,
    #[case] name: &str,
) {
    // Given: a deferred numeric record, with unrelated delivery numbering.
    reset(NameMode::Counter);
    let mut style = static_style("original");
    style.style_order = order;
    let _delivery = css::file_map::get_file_num_by_filename("unrelated");
    // When: its real dormant request is produced after scope exit.
    let receipt = produced(style.counter_produce(filename));
    // Then: original authority and actual legacy address are independent of delivery.
    assert_eq!(receipt.original, 0);
    address(&receipt, namespace, key, 0);
    assert_eq!(receipt.allocation.name, name);
    cleanup();
}

#[rstest]
#[case("margin", "1px 1px 1px 1px", "1px 1px 1px 1px")]
#[case("font-family", "'Roboto', sans-serif", "Roboto,sans-serif")]
#[case("content", "'a b'", "'a b'")]
#[case("typography", "heading", "heading")]
#[case("width", "$missing", "$missing")]
#[serial]
fn static_uses_authored_projection_when_emission_can_expand(
    #[case] property: &str,
    #[case] value: &str,
    #[case] expected: &str,
) {
    // Given: normalized raw IR; content and typography must not use emission identity.
    reset(NameMode::Counter);
    let _scope = scope("original");
    let style = ExtractStaticStyle::new(property, value, 0, None);
    // When: the adapter prepares the exact legacy naming input.
    let receipt = produced(style.counter_produce(None));
    // Then: naming contains the legacy resolved/multi-value projection only.
    assert_eq!(
        receipt.input,
        LegacyInput::Declaration(LegacyDeclaration {
            property: property.into(),
            level: 0,
            value: Some(expected.into()),
            selector: None,
            order: None,
        })
    );
    cleanup();
}

#[test]
#[serial]
fn first_value_is_captured_when_theme_lookup_differs_from_authored_token() {
    // Given: an authored token with a concrete first lookup and layered selector.
    reset(NameMode::Counter);
    css::theme_tokens::set_theme_token_values(
        BTreeMap::from([("space".into(), "24px".into())]),
        BTreeMap::new(),
    );
    let _scope = scope("original");
    let style = ExtractStaticStyle::new_with_layer(
        "width",
        "$space",
        2,
        Some("hover".into()),
        Some("ui".into()),
    )
    .with_theme_token_resolution(ThemeTokenResolution::FirstValue);
    // When: one request captures the currently resolved first value.
    let receipt = produced(style.counter_produce(Some("delivery")));
    // Then: its retained input is literal, not a CSS variable or raw token.
    let LegacyInput::Declaration(input) = receipt.input else {
        panic!("declaration")
    };
    assert_eq!(input.value, Some("24px".into()));
    assert_eq!(input.selector, Some("&:hover@layer ui".into()));
    cleanup();
}

#[rstest]
#[case(Naming::Own)]
#[case(Naming::Risky)]
#[serial]
fn collapsed_delivery_keeps_private_original_when_naming_eligibility_changes(
    #[case] naming: Naming,
) {
    // Given: distinct originals collapsed into one delivery root.
    reset(NameMode::Counter);
    let first = static_style("a").with_naming(naming);
    let second = static_style("b").with_naming(naming);
    css::file_map::set_canonical_map(HashMap::from([
        ("a".into(), "root".into()),
        ("b".into(), "root".into()),
    ]));
    // When: both deferred records request their private classes.
    let receipts = [
        produced(first.counter_produce(Some("a"))),
        produced(second.counter_produce(Some("b"))),
    ];
    // Then: collapsed delivery never aliases original private counters.
    assert_eq!(receipts[0].allocation.name, "pa-a");
    assert_eq!(receipts[1].allocation.name, "pb-a");
    address(&receipts[0], "D9-0", "color-0-red--255-a", 0);
    address(&receipts[1], "D9-1", "color-0-red--255-b", 0);
    cleanup();
}

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
#[serial]
fn wrong_policy_rejects_all_parents_when_shared_and_order_zero(#[case] mode: NameMode) {
    // Given: ordinary numbered scopes remain Current despite numeric owners.
    reset(mode);
    let _scope = SiteScope::enter_numbered(7, "tone", &[]);
    let style = ExtractStaticStyle::new_basic("color", "red", 0, None);
    let dynamic = ExtractDynamicStyle::new("color", 0, "tone", None).at(0);
    let frames = ExtractKeyframes::default();
    let before = get_class_map();
    // When: all parent kinds attempt shared dormant production.
    let errors = [
        style.counter_produce(None).err(),
        dynamic.counter_produce(None).err(),
        frames.counter_produce(None).err(),
    ];
    // Then: policy rejection precedes all reservations in every mode.
    assert_eq!(errors, [Some(CounterProducerError::WrongPolicy); 3]);
    assert_eq!(get_class_map(), before);
    cleanup();
}

#[rstest]
#[case(CounterProducerError::WrongPolicy)]
#[case(CounterProducerError::UnnumberedSite)]
fn errors_are_typed_when_displayed(#[case] error: CounterProducerError) {
    // Given: typed adapter errors.
    let as_error: &dyn std::error::Error = &error;
    // When: the boundary presents a diagnostic.
    let message = as_error.to_string();
    // Then: diagnostics exist without an underlying fabricated cause.
    assert_ne!(message, "");
    assert_eq!(as_error.source().map(ToString::to_string), None);
}

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
#[serial]
fn selector_layer_projection_uses_captured_mode_when_live_mode_later_changes(
    #[case] mode: NameMode,
) {
    // Given: raw selector bytes retained by new_basic, plus an authored layer.
    reset(mode);
    let _scope = scope("a");
    let mut style = ExtractStaticStyle::new_basic(
        "color",
        "red",
        0,
        Some(css::style_selector::StyleSelector::Selector(
            " &:hover ".into(),
        )),
    );
    style.layer = Some("ui".into());
    // When: one adapter invocation captures its selector, prefix and placement.
    let receipt = produced(style.counter_produce(Some("delivery")));
    css::debug::set_debug(true);
    css::atom_hoist::set_atom_hoist(Some(2));
    css::set_prefix(Some("later".into()));
    // Then: frozen ordinary/debug bytes differ from the raw-hex atom selector key.
    let LegacyInput::Declaration(input) = &receipt.input else {
        panic!("class")
    };
    let selector = match mode {
        NameMode::Counter | NameMode::Debug => " &:hover @layer ui".to_string(),
        NameMode::AtomHoist => "s-20263a686f76657220-s-7569".to_string(),
    };
    assert_eq!(input.selector, Some(selector));
    assert_eq!(receipt.context.config.prefix, "p");
    assert_eq!(
        css::counter_names::render_name(&receipt.input, &receipt.context, 0),
        receipt.allocation.name
    );
    cleanup();
}
