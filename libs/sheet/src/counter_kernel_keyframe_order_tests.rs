use super::authentic_support::*;
use crate::StyleSheet;
use extractor::extract_style::extract_static_style::ThemeTokenResolution;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Hash)]
struct Member<'a> {
    property: &'a str,
    value: &'a str,
    level: u8,
    selector: Option<css::style_selector::StyleSelector>,
    order: Option<u8>,
    layer: Option<String>,
    resolution: ThemeTokenResolution,
}
fn key(value: &str, layer: Option<&str>) -> String {
    let mut hash = DefaultHasher::new();
    BTreeMap::from([(
        "from",
        vec![Member {
            property: "opacity",
            value,
            level: 0,
            selector: None,
            order: None,
            layer: layer.map(str::to_string),
            resolution: ThemeTokenResolution::CssVariable,
        }],
    )])
    .hash(&mut hash);
    format!("k-{}", hash.finish())
}
fn frames(value: &str, layer: Option<&str>) -> ExtractStyleValue {
    let mut frames = ExtractKeyframes::default();
    frames.keyframes.insert(
        "from".into(),
        vec![ExtractStaticStyle::new_with_layer(
            "opacity",
            value,
            0,
            None,
            layer.map(str::to_string),
        )],
    );
    ExtractStyleValue::Keyframes(frames)
}

#[test]
#[serial_test::serial]
fn sequential_operations_preserve_flags_when_intermediate_replacement_restores_base_payload() {
    // Given
    let _state = state();
    let (first, last) = fixture("a", || (frames("0", None), frames("1", None)));
    css::class_map::set_class_map(HashMap::from([(
        "D9-0".into(),
        HashMap::from([(key("0", None), 7), (key("1", None), 7)]),
    )]));
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    update(&mut sheet, &mut evidence, &styles([last.clone()])).required("initial");
    let original = sheet.keyframes.clone();
    let reservations = css::class_map::get_class_map();
    // When
    let effects =
        update(&mut sheet, &mut evidence, &styles([last, first])).required("ordered replacements");
    // Then
    assert!(effects.collected);
    assert_eq!(sheet.keyframes, original);
    assert_eq!(
        sheet.keyframes["a"]["pa-h"]["from"],
        vec![("opacity".into(), "1".into())]
    );
    assert_eq!(
        evidence
            .retained
            .as_ref()
            .required("retained")
            .candidates
            .len(),
        1
    );
    assert_eq!(css::class_map::get_class_map(), reservations);
}

#[test]
#[serial_test::serial]
fn distinct_witnesses_remain_when_keyframe_payloads_match_at_same_address() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([frames("0", Some("a")), frames("0", Some("b"))])
    });
    css::class_map::set_class_map(HashMap::from([(
        "D9-0".into(),
        HashMap::from([(key("0", Some("a")), 7), (key("0", Some("b")), 7)]),
    )]));
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    // When
    update(&mut sheet, &mut evidence, &items).required("same payload");
    // Then
    assert_eq!(
        sheet.keyframes["a"]["pa-h"]["from"],
        vec![("opacity".into(), "0".into())]
    );
    let retained = evidence.retained.as_ref().required("retained");
    assert_eq!(retained.candidates.len(), 2);
    assert_eq!(retained.evidence.counters["D9-0"][&7].len(), 2);
}
