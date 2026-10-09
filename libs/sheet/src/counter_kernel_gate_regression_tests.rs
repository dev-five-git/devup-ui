use super::super::authentic_support::*;
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

fn key(value: &str, layer: &str) -> String {
    let mut hash = DefaultHasher::new();
    BTreeMap::from([(
        "from",
        vec![Member {
            property: "opacity",
            value,
            level: 0,
            selector: None,
            order: None,
            layer: Some(layer.into()),
            resolution: ThemeTokenResolution::CssVariable,
        }],
    )])
    .hash(&mut hash);
    format!("k-{}", hash.finish())
}

fn frames(value: &str, layer: &str) -> ExtractStyleValue {
    let mut frames = ExtractKeyframes::default();
    frames.keyframes.insert(
        "from".into(),
        vec![ExtractStaticStyle::new_with_layer(
            "opacity",
            value,
            0,
            None,
            Some(layer.into()),
        )],
    );
    ExtractStyleValue::Keyframes(frames)
}

#[test]
#[serial_test::serial]
fn base_witness_retires_when_intermediate_payload_is_restored_by_distinct_final_witness() {
    // Given
    let _state = state();
    let (base, intermediate, final_item) = fixture("a", || {
        (
            frames("1", "base"),
            frames("0", "intermediate"),
            frames("1", "fresh"),
        )
    });
    let keys = [
        key("1", "base"),
        key("0", "intermediate"),
        key("1", "fresh"),
    ];
    css::class_map::set_class_map(HashMap::from([(
        "D9-0".into(),
        keys.into_iter().map(|key| (key, 7)).collect(),
    )]));
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    update(&mut sheet, &mut evidence, &styles([base])).required("initial BASE");
    let old = evidence.retained.as_ref().required("BASE").candidates[0].clone();
    let original = sheet.keyframes.clone();
    let reservations = css::class_map::get_class_map();
    // When
    let effects = update(
        &mut sheet,
        &mut evidence,
        &styles([final_item, intermediate]),
    )
    .required("ordered restoration");
    // Then
    assert!(effects.collected);
    assert_eq!(sheet.keyframes, original);
    let retained = evidence.retained.as_ref().required("retained");
    assert!(
        !retained.candidates.contains(&old),
        "superseded BASE witness survived"
    );
    assert_eq!(retained.candidates.len(), 1);
    assert_eq!(retained.evidence.counters["D9-0"][&7].len(), 1);
    assert_eq!(css::class_map::get_class_map(), reservations);
}

#[test]
#[serial_test::serial]
fn unused_delivery_binding_is_released_when_guarded_cleanup_removes_last_candidate() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color",
            "red",
            0,
            Some(css::style_selector::StyleSelector::Global(
                "body".into(),
                "a".into(),
            )),
        ))])
    });
    let (mut sheet, mut evidence) = (StyleSheet::default(), KernelEvidence::default());
    update(&mut sheet, &mut evidence, &items).required("private global static");
    let reservations = css::class_map::get_class_map();
    let effects = update(&mut sheet, &mut evidence, &styles([])).required("guarded cleanup");
    assert!(effects.default_collected);
    assert_eq!(
        evidence
            .retained
            .as_ref()
            .required("removed")
            .candidates
            .len(),
        0
    );
    let mut files = css::file_map::get_file_map();
    files.insert("a".into(), 7);
    css::file_map::set_file_map(files);
    // When
    let result = update(&mut sheet, &mut evidence, &styles([]));
    // Then
    assert_eq!(
        result,
        Ok(UpdateEffects {
            collected: false,
            updated_base_style: false,
            default_collected: false,
        })
    );
    let retained = evidence.retained.as_ref().required("empty retained");
    assert_eq!(retained.authority.originals, BTreeMap::new());
    assert_eq!(retained.authority.files, BTreeMap::new());
    assert_eq!(retained.authority.placements, vec![]);
    assert_eq!(retained.authority.deliveries, BTreeMap::new());
    assert_eq!(css::class_map::get_class_map(), reservations);
}
