use super::test_support::*;
use css::style_selector::{AtRuleKind, StyleSelector};

#[rstest::rstest]
#[case(false, false, false)]
#[case(true, true, false)]
#[case(false, true, false)]
#[case(true, false, false)]
#[case(false, true, true)]
#[case(true, false, true)]
#[serial_test::serial]
fn frozen_cleanup_adopts_reset_when_actual_empty_target_effect_allows_mixed_modes(
    #[case] placement_single: bool,
    #[case] cleanup_single: bool,
    #[case] at: bool,
) {
    // Given
    let _guard = state();
    css::file_map::set_canonical_map(HashMap::from([("a".into(), String::new())]));
    let selector = if at {
        StyleSelector::At {
            kind: AtRuleKind::Supports,
            query: "(display:grid)".into(),
            selector: Some("body".into()),
            outer: vec![],
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
    CounterSheet::new(&mut sheet)
        .with_attempt(|attempt| {
            attempt
                .prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "a",
                        single_css: placement_single,
                    },
                )?
                .finish(|_, _| Ok::<_, ()>(()))
        })
        .required("update");
    assert!(sheet.rm_global_css("a", cleanup_single));
    assert!(!sheet.rm_global_css("a", cleanup_single));
    let bytes = encoded(&mut sheet);
    css::file_map::set_canonical_map(HashMap::from([("a".into(), "fresh".into())]));
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    assert_eq!(target.properties[""][&255][&0].len(), 1);
    assert!(
        target.properties[""][&255][&0]
            .iter()
            .all(|record| record.owner_reset && record.selector.is_none())
    );
    let retained = target.counter_state.as_ref().required("state");
    assert_eq!(retained.cleanups.len(), 1);
    assert_eq!(retained.cleanups[0].single_css, cleanup_single);
    assert_eq!(
        retained.phase,
        super::super::BatchPhase::Retained(BTreeSet::new())
    );
}

#[rstest::rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[serial_test::serial]
fn cleanup_linkage_rejects_when_history_target_or_phase_is_damaged(#[case] damage: u8) {
    // Given
    let _guard = state();
    let items = fixture("a", || {
        styles([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color",
            0,
            "tone",
            Some(StyleSelector::Global("body".into(), "a".into())),
        ))])
    });
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &items).required("update");
    assert!(sheet.rm_global_css("a", false));
    let mut value = packet(&mut sheet);
    match damage {
        0 => value["evidence"]["cleanups"] = json!([]),
        1 => value["evidence"]["cleanups"][0]["single_css"] = json!(true),
        2 => {
            witness(&mut value)["proof"]["emission"]["materialization"]["AfterGlobalCleanup"]["bucket"] =
                json!("wrong");
        }
        3 => value["evidence"]["phase"] = json!({"Retained":["missing"]}),
        _ => panic!("damage"),
    }
    // When
    let result = admit(value);
    // Then
    assert!(result.is_err());
}

#[test]
#[serial_test::serial]
fn cleanup_history_survives_when_owners_re_register_and_old_metadata_is_pruned() {
    // Given
    let _guard = state();
    let mut sheet = StyleSheet::default();
    for value in ["red", "blue", "green"] {
        let items = fixture("a", || {
            styles([ExtractStyleValue::Static(ExtractStaticStyle::new(
                "color",
                value,
                0,
                Some(StyleSelector::Global("body".into(), "a".into())),
            ))])
        });
        update(&mut sheet, &items).required("update");
        assert!(sheet.rm_global_css("a", false));
    }
    let bytes = encoded(&mut sheet);
    let mut target = StyleSheet::default();
    // When
    validate_snapshot(parse(&bytes).required("parse"))
        .required("admit")
        .install_into(&mut target)
        .required("install");
    // Then
    let retained = target.counter_state.as_ref().required("state");
    assert_eq!(retained.cleanups.len(), 3);
    assert_eq!(retained.cleanups[0], retained.cleanups[2]);
    assert_eq!(retained.candidates().count(), 0);
    assert_eq!(retained.originals.len(), 0);
    assert_eq!(target.source_ids, BTreeMap::from([("a".into(), 0)]));
}
