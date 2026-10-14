use super::counter_keyframe_input::{LegacyMember, legacy_map};
use super::counter_producer_test_support::{address, cleanup, produced, reset, scope};
use super::extract_static_style::{ExtractStaticStyle, ThemeTokenResolution};
use super::policy_test_support::trace;
use super::{CounterProducerError, ExtractKeyframes, ProducerPolicy};
use css::{
    CounterOwner, Naming,
    allocation_input::{LegacyInput, NameMode},
    class_map::get_class_map,
    style_selector::{AtRule, AtRuleKind, StyleSelector},
};
use rstest::rstest;
use serial_test::serial;
use std::{
    collections::BTreeMap,
    hash::{DefaultHasher, Hash, Hasher},
};

#[path = "counter_keyframe_test_reference.rs"]
mod reference;
use reference::{ReferenceMember, reference, reference_map};

#[derive(Clone, Copy)]
enum Field {
    Property,
    Value,
    Level,
    Selector,
    Order,
    Layer,
    Resolution,
}

#[rstest]
#[case(Field::Property)]
#[case(Field::Value)]
#[case(Field::Level)]
#[case(Field::Selector)]
#[case(Field::Order)]
#[case(Field::Layer)]
#[case(Field::Resolution)]
fn seven_field_trace_matches_baseline_when_each_legacy_field_changes(#[case] field: Field) {
    // Given: a raw member with nonempty fields and a separate derived baseline reference.
    let mut style = ExtractStaticStyle::new("color", "$tone", 1, Some("hover".into()));
    style.layer = Some("ui".into());
    let before = trace(&LegacyMember(&style));
    // When: precisely one legacy field changes.
    match field {
        Field::Property => style.property = "background".into(),
        Field::Value => style.value = "red".into(),
        Field::Level => style.level = 2,
        Field::Selector => {
            style.selector = Some(StyleSelector::Global("&:hover".into(), "raw".into()));
        }
        Field::Order => style.style_order = Some(255),
        Field::Layer => style.layer = Some("other".into()),
        Field::Resolution => style.theme_token_resolution = ThemeTokenResolution::FirstValue,
    }
    // Then: each field affects the exact seven-call trace, not a tuple or new IR hash.
    assert_ne!(trace(&LegacyMember(&style)), before);
    assert_eq!(trace(&LegacyMember(&style)), trace(&reference(&style)));
}

#[rstest]
#[case(None)]
#[case(Some(StyleSelector::Selector(" &:hover ".into())))]
#[case(Some(StyleSelector::Global("&:hover".into(), "raw-owner".into())))]
#[case(Some(StyleSelector::At { kind: AtRuleKind::Container, query: "(width>1px)".into(), selector: Some("&".into()), outer: vec![AtRule { kind: AtRuleKind::Supports, query: "(display:grid)".into() }], file: Some("raw-owner".into()) }))]
fn selector_hash_trace_matches_baseline_when_all_selector_domains_are_used(
    #[case] selector: Option<StyleSelector>,
) {
    // Given: full raw selector fields including outer rules and cleanup filenames.
    let style = ExtractStaticStyle::new_basic("color", "red", 0, selector);
    // When: the borrowed legacy member hashes its seven fields.
    let actual = trace(&LegacyMember(&style));
    // Then: an independently derived enum reproduces discriminants and field framing.
    assert_eq!(actual, trace(&reference(&style)));
}

#[test]
#[serial]
fn container_hash_matches_baseline_when_steps_have_duplicates_and_empty_vectors() {
    // Given: ordered map keys, empty vectors, duplicate members and raw None orders.
    reset(NameMode::Counter);
    let _scope = scope("a");
    let style = ExtractStaticStyle::new("color", "red", 0, None);
    let frames = ExtractKeyframes {
        keyframes: BTreeMap::from([
            ("to".into(), vec![style.clone(), style]),
            ("from".into(), vec![]),
        ]),
        ..ExtractKeyframes::default()
    };
    let before = get_class_map();
    let reference = reference_map(&frames);
    let mut hasher = DefaultHasher::new();
    reference.hash(&mut hasher);
    // When: pure map hashing and the single actual request are evaluated.
    assert_eq!(trace(&legacy_map(&frames)), trace(&reference));
    assert_eq!(get_class_map(), before);
    let receipt = produced(frames.counter_produce(Some("delivery")));
    // Then: standard map/vector framing yields one decimal keyframe request only.
    assert_eq!(
        receipt.input,
        LegacyInput::Keyframes(hasher.finish().to_string())
    );
    address(&receipt, "D9-0", &format!("k-{}", hasher.finish()), 0);
    assert_eq!(
        get_class_map()
            .values()
            .map(std::collections::HashMap::len)
            .sum::<usize>(),
        1
    );
    cleanup();
}

#[test]
fn legacy_trace_excludes_metadata_when_new_authority_and_naming_change() {
    // Given: a raw member's seven-field trace.
    let mut style = ExtractStaticStyle::new("color", "$tone", 0, None);
    let before = trace(&LegacyMember(&style));
    // When: every new metadata dimension changes without touching legacy fields.
    style.naming = Naming::Risky;
    style.counter_owner = CounterOwner::D9(99);
    style.producer_policy = ProducerPolicy::CounterOriginal(98);
    style.origin = css::style_origin::Origin(
        Some(Box::new(css::style_origin::StyleOrigin {
            file: "raw".into(),
            line: 1,
            column: 2,
            expression: "tone".into(),
        })),
        Some(Box::new(css::style_origin::RealLocation::ModuleExport {
            file: "bucket".into(),
            binding: Some("tone".into()),
        })),
    );
    // Then: the internal legacy map hash cannot inherit policy or content identity.
    assert_eq!(trace(&LegacyMember(&style)), before);
}

#[rstest]
#[case(NameMode::Counter)]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
#[serial]
fn current_child_rejects_before_request_when_parent_has_counter_authority(#[case] mode: NameMode) {
    // Given: a counter parent with a manually mixed Current child.
    reset(mode);
    let current = ExtractStaticStyle::new("color", "red", 0, None);
    let _scope = scope("a");
    let mut frames = ExtractKeyframes::default();
    frames.keyframes.insert("to".into(), vec![current]);
    let before = get_class_map();
    // When: the new boundary inspects all children before capture/allocation.
    let error = frames.counter_produce(None).err();
    // Then: wrong child policy cannot reserve even shared/debug/atom names.
    assert_eq!(error, Some(CounterProducerError::WrongPolicy));
    assert_eq!(get_class_map(), before);
    cleanup();
}

#[test]
#[serial]
fn debug_keyframes_use_decimal_legacy_hash_when_no_counter_is_reserved() {
    // Given: raw themed fields whose effective lookup can later change.
    reset(NameMode::Debug);
    let _scope = scope("a");
    let mut frames = ExtractKeyframes::default();
    frames.keyframes.insert(
        "to".into(),
        vec![
            ExtractStaticStyle::new("width", "$space", 0, None)
                .with_theme_token_resolution(ThemeTokenResolution::FirstValue),
        ],
    );
    let before = trace(&legacy_map(&frames));
    css::theme_tokens::set_theme_token_values(
        BTreeMap::from([("space".into(), "24px".into())]),
        BTreeMap::new(),
    );
    let mut hasher = DefaultHasher::new();
    reference_map(&frames).hash(&mut hasher);
    // When: debug production projects the raw legacy map, not resolved members.
    let receipt = produced(frames.counter_produce(Some("delivery")));
    // Then: debug remains decimal k- and effective lookup cannot alter its hash trace.
    assert_eq!(
        receipt.input,
        LegacyInput::Keyframes(hasher.finish().to_string())
    );
    assert_eq!(receipt.allocation.name, format!("pk-{}", hasher.finish()));
    assert_eq!(trace(&legacy_map(&frames)), before);
    assert_eq!(get_class_map().len(), 0);
    cleanup();
}

#[test]
#[serial]
fn atom_input_is_old_hex_content_when_keyframes_delivery_is_hoisted() {
    // Given: a hoisted declaration bucket and a counter child from a different original.
    reset(NameMode::AtomHoist);
    css::debug::set_debug(true);
    css::atom_hoist::restore_atom_plan(Some(std::collections::BTreeSet::from(["root".into()])));
    let mut frames = {
        let _scope = scope("a");
        ExtractKeyframes::default()
    };
    let child = {
        let _scope = scope("b");
        ExtractStaticStyle::new("color", "red", 0, None)
    };
    frames.keyframes.insert("to".into(), vec![child]);
    // When: the one keyframe name is produced from the old step/field string.
    let receipt = produced(frames.counter_produce(Some("root")));
    // Then: atom k1 stays local (never h) and receives hex content, not a decimal digest.
    assert_eq!(receipt.original, 0);
    assert_eq!(
        receipt.input,
        LegacyInput::Keyframes("746f{636f6c6f72:726564;}".into())
    );
    assert_eq!(
        receipt.allocation.name,
        format!(
            "pk1-l-726f6f74-{}",
            css::atom_name::hex("746f{636f6c6f72:726564;}")
        )
    );
    assert_eq!(get_class_map().len(), 0);
    cleanup();
}

#[test]
#[serial]
fn empty_keyframes_retain_original_when_deferred_beyond_scope_exit() {
    // Given: two empty frame parents cannot infer authority from any children.
    reset(NameMode::Counter);
    let a = {
        let _scope = scope("a");
        ExtractKeyframes::default()
    };
    let b = {
        let _scope = scope("b");
        ExtractKeyframes::default()
    };
    let mut reference = DefaultHasher::new();
    BTreeMap::<String, Vec<ReferenceMember<'_>>>::new().hash(&mut reference);
    // When: deferred records request private keyframe names.
    let receipts = [a, b].map(|frames| produced(frames.counter_produce(Some("delivery"))));
    // Then: empty map hash is unchanged while original namespaces remain distinct.
    assert_eq!(
        receipts[0].input,
        LegacyInput::Keyframes(reference.finish().to_string())
    );
    assert_eq!(receipts[0].allocation.name, "pa-a");
    assert_eq!(receipts[1].allocation.name, "pb-a");
    cleanup();
}
