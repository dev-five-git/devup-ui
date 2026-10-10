use super::counter_fixture_support::{address, fixture, produced, state};
use extractor::extract_style::{
    ExtractKeyframes, ProducerPolicy,
    extract_static_style::{ExtractStaticStyle, ThemeTokenResolution},
};
use std::{
    collections::{BTreeMap, HashMap},
    hash::{DefaultHasher, Hash, Hasher},
};

#[derive(Hash)]
struct ReferenceMember {
    property: &'static str,
    value: &'static str,
    level: u8,
    selector: Option<css::style_selector::StyleSelector>,
    order: Option<u8>,
    layer: Option<String>,
    resolution: ThemeTokenResolution,
}

fn reference(value: &'static str) -> ReferenceMember {
    ReferenceMember {
        property: "opacity",
        value,
        level: 0,
        selector: None,
        order: None,
        layer: None,
        resolution: ThemeTokenResolution::CssVariable,
    }
}

#[test]
#[serial_test::serial]
fn ordered_children_retain_lineage_when_keyframes_make_one_legacy_hash_request() {
    // Given: authentic parent/child originals and an independent legacy input fixture.
    let _state = state();
    let mut frames = fixture("parent", ExtractKeyframes::default);
    let first = fixture("first", || ExtractStaticStyle::new("opacity", "0", 0, None));
    let second = fixture("second", || {
        ExtractStaticStyle::new("opacity", "1", 0, None)
    });
    frames
        .keyframes
        .insert("from".into(), vec![second.clone(), first.clone(), second]);
    frames.keyframes.insert("to".into(), vec![first]);
    let mut hasher = DefaultHasher::new();
    BTreeMap::from([
        ("from", vec![reference("1"), reference("0"), reference("1")]),
        ("to", vec![reference("0")]),
    ])
    .hash(&mut hasher);
    let input = hasher.finish().to_string();
    let key = format!("k-{input}");
    let before = HashMap::from([("empty".into(), HashMap::new())]);
    css::class_map::set_class_map(before);
    // When: a clone produces the one parent request after all constructor scopes end.
    let deferred = frames.clone();
    let receipt = produced(deferred.counter_produce(Some("delivery")));
    // Then: decimal legacy hash is the request, not a visible hash-name fallback.
    assert_eq!(frames.producer_policy(), ProducerPolicy::CounterOriginal(0));
    assert_eq!(
        frames
            .keyframes
            .values()
            .flatten()
            .map(ExtractStaticStyle::producer_policy)
            .collect::<Vec<_>>(),
        vec![
            ProducerPolicy::CounterOriginal(2),
            ProducerPolicy::CounterOriginal(1),
            ProducerPolicy::CounterOriginal(2),
            ProducerPolicy::CounterOriginal(1)
        ]
    );
    assert_eq!(receipt.original, 0);
    assert_eq!(
        receipt.input,
        css::allocation_input::LegacyInput::Keyframes(input)
    );
    address(&receipt, ("D9-0", &key, 0, "pa-a"));
    assert_eq!(
        css::class_map::get_class_map(),
        HashMap::from([
            ("empty".into(), HashMap::new()),
            ("D9-0".into(), HashMap::from([(key, 0)])),
        ])
    );
    assert_eq!(
        css::file_map::get_original_ids(),
        BTreeMap::from([
            ("parent".into(), 0),
            ("first".into(), 1),
            ("second".into(), 2),
        ])
    );
}

#[test]
#[serial_test::serial]
fn empty_keyframe_parent_retains_original_when_deferred_without_children() {
    // Given: no child record can supply the parent's retained policy.
    let _state = state();
    fixture("plain", || ());
    let frames = fixture("empty", ExtractKeyframes::default);
    let mut hasher = DefaultHasher::new();
    BTreeMap::<&str, Vec<ReferenceMember>>::new().hash(&mut hasher);
    let input = hasher.finish().to_string();
    let key = format!("k-{input}");
    // When: the real empty constructor's cloned parent produces after TLS exit.
    let deferred = frames.clone();
    let receipt = fixture("later", move || {
        produced(deferred.counter_produce(Some("delivery")))
    });
    // Then: original one owns one exact request; no child or hash fallback is invented.
    assert_eq!(frames.producer_policy(), ProducerPolicy::CounterOriginal(1));
    assert_eq!(frames.keyframes, BTreeMap::new());
    assert_eq!(receipt.original, 1);
    assert_eq!(
        receipt.input,
        css::allocation_input::LegacyInput::Keyframes(input)
    );
    address(&receipt, ("D9-1", &key, 0, "pb-a"));
    assert_eq!(
        css::class_map::get_class_map(),
        HashMap::from([("D9-1".into(), HashMap::from([(key, 0)])),])
    );
}
