use super::cache6_test_support::*;

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[serial]
fn genuine_state_is_adopted_before_work_when_raw6_is_complete(#[case] family: u8) {
    // Given
    let _guard = Guard::new();
    configure_typography();
    output("a", family);
    with_style_sheet_mut(|sheet| {
        sheet.add_css("literal", "body{}");
        sheet.add_import("literal", "literal.css");
        sheet.add_font_face(
            "literal",
            &BTreeMap::from([("font-family".into(), "literal".into())]),
        );
        sheet.add_property("first", "color", 0, "blue", None, None, None);
        sheet.add_property("second", "color", 0, "blue", None, None, None);
    });
    seed_file_map(vec!["unused".into()]);
    css::class_map::with_class_map_mut(|map| {
        map.insert("unused".into(), HashMap::from([("reservation".into(), 0)]));
    });
    let bytes = encoded();
    let expected_maps = cache6_session::Maps::capture();
    with_style_sheet_mut(|sheet| sheet.source_ids = css::file_map::get_original_ids());
    let expected_sheet = with_style_sheet(LiveCheckpoint::capture);
    fresh();
    configure_typography();
    exact_test_support::compile(
        "manual.tsx",
        "import {css} from '@devup-ui/react';export const x=css({color:'green'});",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_ne!(with_style_sheet(|sheet| sheet.names.len()), 0);
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    // Then
    assert_eq!(cache6_session::Maps::capture(), expected_maps);
    assert_eq!(with_style_sheet(LiveCheckpoint::capture), expected_sheet);
    assert_eq!(with_style_sheet(|sheet| sheet.names.len()), 0);
    cache6_restore::RESTORE.with_borrow(|state| assert!(state.session.is_some()));
}

#[rstest]
#[case(None)]
#[case(Some(BTreeSet::new()))]
#[case(Some(BTreeSet::from(["a".into()])))]
#[serial]
fn empty_counter_state_and_plan_are_adopted_when_snapshot_has_no_records(
    #[case] plan: Option<BTreeSet<String>>,
) {
    // Given
    let _guard = Guard::new();
    update(
        &Default::default(),
        OutputRoute {
            raw_source: "a",
            single_css: true,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    css::atom_hoist::restore_atom_plan(plan.clone());
    with_style_sheet_mut(|sheet| sheet.atom_plan.clone_from(&plan));
    let bytes = encoded();
    let expected = authority();
    fresh();
    let absent = with_style_sheet(LiveCheckpoint::capture);
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    // Then
    assert_eq!(authority(), expected);
    assert_ne!(with_style_sheet(LiveCheckpoint::capture), absent);
    assert_eq!(css::atom_hoist::atom_plan(), plan);
}

#[test]
#[serial]
fn two_lineages_survive_adoption_when_equal_names_share_a_slot() {
    // Given
    let _guard = Guard::new();
    for source in ["a", "b"] {
        let items = with_original(
            FixtureRequest {
                filename: source,
                source: "abcdef",
            },
            || {
                let mut declaration = ExtractStaticStyle::new("color", "red", 0, None);
                declaration.style_order = Some(0);
                FxHashSet::from_iter([ExtractStyleValue::Static(declaration)])
            },
        )
        .unwrap_or_else(|error| panic!("{error:?}"));
        update(
            &items,
            OutputRoute {
                raw_source: "a",
                single_css: false,
                import_main_css: false,
            },
        )
        .unwrap_or_else(|error| panic!("{error:?}"));
    }
    let bytes = encoded();
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).unwrap_or_else(|error| panic!("{error:?}"));
    let witnesses: Vec<_> = value["evidence"]["counters"]
        .as_object()
        .unwrap_or_else(|| panic!("namespaces"))
        .values()
        .flat_map(|slots| {
            slots
                .as_object()
                .unwrap_or_else(|| panic!("slots"))
                .values()
        })
        .flat_map(|value| value.as_array().unwrap_or_else(|| panic!("witnesses")))
        .collect();
    assert_eq!(witnesses.len(), 2);
    assert_eq!(witnesses[0]["proof"], witnesses[1]["proof"]);
    assert_ne!(witnesses[0]["lineage"], witnesses[1]["lineage"]);
    with_style_sheet_mut(|sheet| sheet.source_ids = css::file_map::get_original_ids());
    let expected = authority();
    fresh();
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    // Then
    assert_eq!(authority(), expected);
}

#[test]
#[serial]
fn ordinary_compiler_stays_current_when_constructor_feature_is_enabled() {
    // Given
    let _guard = Guard::new();
    // When
    with_original(
        FixtureRequest {
            filename: "fixture",
            source: "const a=1;",
        },
        || {
            exact_test_support::compile(
                "ordinary.tsx",
                "import {css} from '@devup-ui/react';export const x=css({color:'red'});",
            )
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"))
    .unwrap_or_else(|error| panic!("{error:?}"));
    // Then
    assert_eq!(css::file_map::original_id("ordinary.tsx"), None);
    assert_ne!(with_style_sheet(|sheet| sheet.names.len()), 0);
    assert!(with_style_sheet_mut(|sheet| CounterSheet::new(sheet).export_snapshot6()).is_err());
}
