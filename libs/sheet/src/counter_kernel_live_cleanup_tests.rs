use super::authentic_support::*;
use crate::{StyleSheet, counter_evidence::Materialization};
use css::style_selector::{AtRuleKind, StyleSelector};

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn cleanup_preserves_selector_free_reset_when_global_or_at_consumer_is_removed(#[case] at: bool) {
    // Given
    let _state = state();
    let selector = if at {
        StyleSelector::At {
            kind: AtRuleKind::Media,
            query: "print".into(),
            selector: Some("body".into()),
            outer: Vec::new(),
            file: Some("a".into()),
        }
    } else {
        StyleSelector::Global("body".into(), "a".into())
    };
    let items = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color",
            0,
            "tone",
            Some(selector),
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("initial");
    let reservations = css::class_map::get_class_map();
    // When
    let effects = update(&mut sheet, &styles([])).required("cleanup");
    // Then
    assert_eq!(
        effects,
        UpdateEffects {
            collected: false,
            updated_base_style: false,
            default_collected: true
        }
    );
    let survivors = &sheet.properties["a"][&255][&0];
    assert_eq!(survivors.len(), 1);
    assert!(
        survivors
            .iter()
            .all(|record| record.owner_reset && record.selector.is_none())
    );
    let retained = sheet.counter_state.as_ref().required("retained");
    assert_eq!(
        retained
            .candidates()
            .next()
            .required("candidate")
            .proof
            .emission
            .materialization,
        Materialization::AfterGlobalCleanup {
            source: "a".into(),
            bucket: "a".into()
        }
    );
    assert_eq!(retained.phase, super::BatchPhase::Retained(BTreeSet::new()));
    assert_eq!(css::class_map::get_class_map(), reservations);
}

#[test]
#[serial_test::serial]
fn fresh_global_owner_cannot_create_base_guard_when_base_is_empty() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color",
            "red",
            0,
            Some(StyleSelector::Global("body".into(), "a".into())),
        ))])
    });
    let mut sheet = StyleSheet::default();
    // When
    let effects = update(&mut sheet, &items).required("incoming");
    // Then
    assert!(!effects.default_collected);
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("retained")
            .cleanups
            .len(),
        0
    );
    assert_eq!(sheet.global_css_files, BTreeSet::from(["a".into()]));
}

#[test]
#[serial_test::serial]
fn true_base_cleanup_precedes_re_registration_when_distinct_replacement_is_inserted() {
    // Given
    let _state = state();
    let mut sheet = StyleSheet::default();
    let old = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color",
            "red",
            0,
            Some(StyleSelector::Global("body".into(), "a".into())),
        ))])
    });
    update(&mut sheet, &old).required("initial");
    let new = fixture("a", || {
        styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color",
            "blue",
            0,
            Some(StyleSelector::Global("body".into(), "a".into())),
        ))])
    });
    // When
    let effects = update(&mut sheet, &new).required("replace");
    // Then
    assert!(effects.default_collected && effects.collected);
    assert_eq!(sheet.global_css_files, BTreeSet::from(["a".into()]));
    assert_eq!(sheet.properties["a"][&255][&0].len(), 1);
    assert_eq!(
        sheet.properties["a"][&255][&0]
            .iter()
            .next()
            .required("replacement")
            .value,
        "blue"
    );
    assert_eq!(
        sheet
            .counter_state
            .as_ref()
            .required("retained")
            .candidates()
            .count(),
        1
    );
    assert_eq!(css::class_map::get_class_map()["D9-0"].len(), 2);
}

#[test]
#[serial_test::serial]
fn historical_buckets_coexist_when_resolver_changes_between_attempts() {
    // Given
    let _state = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color",
            0,
            "tone",
            Some(StyleSelector::Global("body".into(), "a".into())),
        ))])
    });
    let mut sheet = StyleSheet::default();
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "old".into())]));
    update(&mut sheet, &items).required("old bucket");
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "new".into())]));
    // When
    let effects = update(&mut sheet, &items).required("new bucket");
    // Then
    assert!(effects.default_collected);
    assert_eq!(sheet.properties["old"][&255][&0].len(), 2);
    assert_eq!(sheet.properties["new"][&255][&0].len(), 2);
    let retained = sheet.counter_state.as_ref().required("retained");
    assert_eq!(retained.cleanups[0].bucket, "new");
    assert_eq!(retained.deliveries.len(), 2);
    assert_eq!(
        retained.files,
        BTreeMap::from([("old".into(), 0), ("new".into(), 1)])
    );
}

#[test]
#[serial_test::serial]
fn peer_owner_survives_when_cleanup_targets_raw_owner_in_collapsed_bucket() {
    // Given
    let _state = state();
    let a = fixture("a", || {
        ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color",
            "red",
            0,
            Some(StyleSelector::Global("body".into(), "a".into())),
        ))
    });
    let b = fixture("b", || {
        ExtractStyleValue::Static(ExtractStaticStyle::new(
            "color",
            "blue",
            0,
            Some(StyleSelector::Global("body".into(), "b".into())),
        ))
    });
    css::file_map::set_canonical_map(HashMap::from([
        ("a".into(), "root".into()),
        ("b".into(), "root".into()),
    ]));
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([a, b])).required("initial");
    // When
    update(&mut sheet, &styles([])).required("cleanup");
    // Then
    assert_eq!(sheet.global_css_files, BTreeSet::from(["b".into()]));
    assert_eq!(sheet.properties["root"][&255][&0].len(), 1);
    assert_eq!(
        sheet.properties["root"][&255][&0]
            .iter()
            .next()
            .required("peer")
            .value,
        "blue"
    );
}
