use super::{KernelError, authentic_support::*, state_live};
use crate::StyleSheet;

#[path = "counter_kernel_cache_privacy_tests.rs"]
mod cache_privacy_tests;
#[path = "counter_kernel_current_family_tests.rs"]
mod current_family_tests;

#[test]
#[serial_test::serial]
fn generated_records_never_gain_authorship_when_kernel_or_current_mutates_counter_sheet() {
    // Given
    let _state = state();
    let generated = fixture("a", || {
        styles([
            ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None)),
            ExtractStyleValue::Static(ExtractStaticStyle::new("typography", "missing", 0, None)),
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 0, "tone", None)),
            ExtractStyleValue::Keyframes(ExtractKeyframes::default()),
        ])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &generated).required("genuine kernel");
    assert_eq!(
        sheet.counter_state.as_ref().required("state").authored,
        vec![]
    );
    let current = styles([
        ExtractStyleValue::Static(ExtractStaticStyle::new("opacity", "0", 0, None)),
        ExtractStyleValue::Static(ExtractStaticStyle::new("typography", "missing", 0, None)),
        ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("padding", 0, "space", None)),
        ExtractStyleValue::Keyframes(ExtractKeyframes::default()),
    ]);
    // When
    sheet
        .update_styles(&current, "a", false)
        .required("Current raw insertion");
    // Then
    assert_eq!(
        sheet.counter_state.as_ref().required("state").authored,
        vec![]
    );
    assert!(matches!(
        state_live::capture_owned(&sheet),
        Err(KernelError::Coverage)
    ));
}

#[test]
#[serial_test::serial]
fn literal_keyframe_retires_generated_witness_when_payload_is_replaced() {
    // Given
    let _state = state();
    let generated = fixture("a", || {
        styles([ExtractStyleValue::Keyframes(ExtractKeyframes::default())])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &generated).required("generated empty entry");
    let maps = css::class_map::get_class_map();
    // When
    assert!(sheet.add_keyframes(
        "pa-a",
        BTreeMap::from([(
            "from".into(),
            vec![
                ("opacity".into(), "0".into()),
                ("opacity".into(), "0".into())
            ]
        )]),
        Some("a")
    ));
    // Then
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("state")
            .candidates()
            .count(),
        0
    );
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("state")
            .authored
            .len(),
        1
    );
    assert_eq!(sheet.keyframes["a"]["pa-a"]["from"].len(), 2);
    assert_eq!(css::class_map::get_class_map(), maps);
    state_live::capture_owned(&sheet).required("replaced payload capture");
}

#[test]
#[serial_test::serial]
fn equal_literal_keyframes_keep_generated_witness_when_cleanup_removes_only_raw_owner() {
    // Given
    let _state = state();
    let generated = fixture("a", || {
        styles([ExtractStyleValue::Keyframes(ExtractKeyframes::default())])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &generated).required("generated entry");
    assert!(!sheet.add_keyframes("pa-a", BTreeMap::new(), Some("a")));
    sheet.add_css("a", "body{}");
    // When
    assert!(sheet.rm_global_css("a", false));
    // Then
    assert_eq!(sheet.keyframes["a"]["pa-a"], BTreeMap::new());
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("state")
            .candidates()
            .count(),
        1
    );
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("state")
            .authored
            .len(),
        1
    );
    state_live::capture_owned(&sheet).required("surviving keyframe capture");
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn public_cleanup_projects_dynamic_reset_once_when_actual_guard_succeeds(#[case] single: bool) {
    // Given
    let _state = state();
    let generated = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color",
            0,
            "tone",
            Some(css::style_selector::StyleSelector::Global(
                "body".into(),
                "a".into(),
            )),
        ))])
    });
    let mut sheet = StyleSheet::default();
    CounterSheet::new(&mut sheet)
        .with_attempt(|attempt| {
            attempt
                .prepare(
                    &generated,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: single,
                    },
                )?
                .finish(|_, _| Ok::<_, ()>(()))
        })
        .required("generated entry");
    let maps = css::class_map::get_class_map();
    // When
    assert!(sheet.rm_global_css("a", single));
    // Then
    let bucket = if single { "" } else { "a" };
    assert_eq!(sheet.properties[bucket][&255][&0].len(), 1);
    let state = sheet.counter_state.as_ref().required("state");
    assert_eq!(state.cleanups.len(), 1);
    assert_eq!(state.cleanups[0].bucket, bucket);
    assert_eq!(state.authored, vec![]);
    assert_eq!(css::class_map::get_class_map(), maps);
    state_live::capture_owned(&sheet).required("partial generated capture");
}
