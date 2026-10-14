use super::authentic_support::*;
use crate::{StyleSheet, theme::Typographies};

#[test]
#[serial_test::serial]
fn empty_keyframe_entry_materializes_when_real_add_keyframes_returns_false() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Keyframes(ExtractKeyframes::default())])
    });
    // When
    let effects = update(&mut sheet, &items).required("empty keyframes");
    // Then
    assert_eq!(
        effects,
        UpdateEffects {
            collected: false,
            updated_base_style: false,
            default_collected: false
        }
    );
    assert_eq!(sheet.keyframes["a"]["pa-a"], BTreeMap::new());
    assert_eq!(css::class_map::get_class_map()["D9-0"].len(), 1);
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("retained")
            .candidates()
            .count(),
        1
    );
}

#[test]
#[serial_test::serial]
fn replacement_retires_old_payload_when_same_keyframe_input_uses_changed_preset() {
    // Given
    let _state = state();
    let _presets = Presets::save();
    let mut sheet = StyleSheet::default();
    let items = fixture("a", || {
        let mut frames = ExtractKeyframes::default();
        frames.keyframes.insert(
            "from".into(),
            vec![ExtractStaticStyle::new("typography", "heading", 0, None)],
        );
        styles([ExtractStyleValue::Keyframes(frames)])
    });
    sheet.theme.typography.insert(
        "heading".into(),
        Typographies(vec![Some(frame("16px", None))]),
    );
    css::content_typography::set(BTreeMap::from([(
        "heading".into(),
        vec![(0, "font-size".into(), "16px".into())],
    )]));
    update(&mut sheet, &items).required("initial");
    let before_maps = css::class_map::get_class_map();
    sheet.theme.typography.insert(
        "heading".into(),
        Typographies(vec![Some(frame("24px", None))]),
    );
    css::content_typography::set(BTreeMap::from([(
        "heading".into(),
        vec![(0, "font-size".into(), "24px".into())],
    )]));
    // When
    let effects = update(&mut sheet, &items).required("replacement");
    // Then
    assert!(effects.collected);
    assert_eq!(css::class_map::get_class_map(), before_maps);
    let retained = sheet.counter_state.as_ref().required("retained");
    assert_eq!(retained.candidates().count(), 1);
    assert!(
        sheet.keyframes["a"]["pa-a"]["from"][0]
            .1
            .contains("32347078")
    );
}

#[test]
#[serial_test::serial]
fn full_eq_records_and_distinct_witnesses_survive_when_slots_are_shared() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([
            ExtractStyleValue::Static(ExtractStaticStyle::new_with_layer(
                "color",
                "red",
                0,
                None,
                Some("a".into()),
            )),
            ExtractStyleValue::Static(ExtractStaticStyle::new_with_layer(
                "color",
                "red",
                0,
                None,
                Some("b".into()),
            )),
        ])
    });
    css::class_map::set_class_map(HashMap::from([(
        "D9-0".into(),
        HashMap::from([
            ("color-0-red-@layer a-255-a".into(), 7),
            ("color-0-red-@layer b-255-a".into(), 7),
        ]),
    )]));
    let mut sheet = StyleSheet::default();
    // When
    update(&mut sheet, &items).required("full Eq");
    // Then
    assert_eq!(sheet.properties["a"][&255][&0].len(), 2);
    assert!(
        sheet.properties["a"][&255][&0]
            .iter()
            .all(|record| record.class_name == "pa-h")
    );
    assert_eq!(
        sheet.counter_state.as_ref().required("retained").counters["D9-0"][&7].len(),
        2
    );
}
