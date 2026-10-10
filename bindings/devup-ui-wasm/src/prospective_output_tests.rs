use super::cache6_test_support::*;

#[test]
#[serial]
fn prospective_output_updates_base_when_first_order_zero_static_finish_has_no_cleanup() {
    // Given
    let _guard = Guard::new();
    let items = with_original(
        FixtureRequest {
            filename: "first",
            source: "abcdef",
        },
        || {
            let mut declaration = ExtractStaticStyle::new("color", "red", 0, None);
            declaration.style_order = Some(0);
            FxHashSet::from_iter([ExtractStyleValue::Static(declaration)])
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    // When
    let output = with_style_sheet_mut(|sheet| {
        CounterSheet::new(sheet).with_attempt(|attempt| {
            attempt
                .prepare(
                    &items,
                    UpdateRequest {
                        raw_source: "first",
                        single_css: false,
                    },
                )?
                .finish(|sheet, effects| {
                    // Then: effects are produced by the real prepare/finish, not fabricated.
                    assert!(effects.updated_base_style);
                    assert!(!effects.default_collected);
                    Ok::<_, String>(Output::from_prospective(
                        metadata(),
                        ProspectiveSheet { sheet, effects },
                        OutputRoute {
                            raw_source: "first",
                            single_css: false,
                            import_main_css: false,
                        },
                    ))
                })
        })
    })
    .unwrap_or_else(|error| panic!("{error:?}"));
    assert!(output.updated_base_style());
}

#[rstest]
#[case(false, false)]
#[case(false, true)]
#[case(true, false)]
#[case(true, true)]
#[serial]
fn prospective_output_preserves_metadata_and_routing_when_real_kernel_inserts(
    #[case] global: bool,
    #[case] import_main: bool,
) {
    // Given
    let _guard = Guard::new();
    import_canonical_map_internal(HashMap::from([("raw".into(), "root".into())]));
    let items = ir("raw", 0);
    // When
    let output = update(
        &items,
        OutputRoute {
            raw_source: "raw",
            single_css: global,
            import_main_css: import_main,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    // Then
    assert_eq!(output.code(), metadata().code);
    assert_eq!(output.map(), Some("fixture-map".into()));
    assert_eq!(output.css_file(), Some("fixture.css".into()));
    assert_eq!(output.dependencies(), vec!["tokens.ts".to_string()]);
    assert_eq!(
        output.css(),
        Some(with_style_sheet(|sheet| sheet.create_css(
            if global { None } else { Some("root") },
            import_main
        )))
    );
    assert!(
        output
            .css()
            .unwrap_or_else(|| panic!("css"))
            .contains("color:red")
    );
}

#[test]
#[serial]
fn prospective_output_has_no_css_when_real_update_inserts_nothing() {
    // Given
    let _guard = Guard::new();
    let items = ir("a", 0);
    update(
        &items,
        OutputRoute {
            raw_source: "a",
            single_css: false,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let maps_before = maps();
    // When
    let output = update(
        &items,
        OutputRoute {
            raw_source: "a",
            single_css: false,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    // Then
    assert_eq!(output.css(), None);
    assert!(!output.updated_base_style());
    assert_eq!(maps(), maps_before);
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn cleanup_only_output_updates_base_when_real_guard_removes_global_or_at_consumer(
    #[case] at: bool,
) {
    // Given
    let _guard = Guard::new();
    let selector = if at {
        css::style_selector::StyleSelector::At {
            kind: css::style_selector::AtRuleKind::Media,
            query: "print".into(),
            selector: Some("body".into()),
            outer: vec![],
            file: Some("raw".into()),
        }
    } else {
        css::style_selector::StyleSelector::Global("body".into(), "raw".into())
    };
    let items = with_original(
        FixtureRequest {
            filename: "raw",
            source: "abcdef",
        },
        || {
            FxHashSet::from_iter([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
                "color",
                0,
                "tone",
                Some(selector),
            ))])
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    update(
        &items,
        OutputRoute {
            raw_source: "raw",
            single_css: false,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let before = maps();
    // When
    let output = update(
        &Default::default(),
        OutputRoute {
            raw_source: "raw",
            single_css: false,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    // Then
    assert!(output.updated_base_style());
    assert!(output.css().is_some());
    assert_eq!(maps(), before);
    with_style_sheet(|sheet| {
        let survivors = &sheet.properties["raw"][&255][&0];
        assert_eq!(survivors.len(), 1);
        assert!(
            survivors
                .iter()
                .all(|record| record.owner_reset && record.selector.is_none())
        );
    });
}

#[test]
#[serial]
fn canonical_global_route_renders_shared_css_when_single_css_option_is_false() {
    // Given
    let _guard = Guard::new();
    import_canonical_map_internal(HashMap::from([(
        "raw".into(),
        css::file_map::GLOBAL_BUCKET.into(),
    )]));
    let items = ir("raw", 0);
    // When
    let output = update(
        &items,
        OutputRoute {
            raw_source: "raw",
            single_css: false,
            import_main_css: true,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    // Then
    assert_eq!(
        output.css(),
        Some(with_style_sheet(|sheet| sheet.create_css(None, true)))
    );
    assert!(with_style_sheet(|sheet| sheet.properties.contains_key("")));
}
