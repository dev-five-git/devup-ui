use super::cache6_test_support::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn outer_rollback_restores_private_state_when_inner_kernel_committed_then_output_failed(
    #[case] unwind: bool,
) {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    let before = authority();
    // When
    let result = catch_unwind(AssertUnwindSafe(|| {
        exact_test_support::exact::<()>(|| {
            output("b", 1);
            assert_ne!(authority(), before);
            assert!(!unwind, "after committed kernel");
            Err("later Output error".into())
        })
    }));
    // Then
    match result {
        Ok(value) => assert_eq!(value, Err("later Output error".into())),
        Err(payload) => assert_eq!(
            payload.downcast_ref::<&str>(),
            Some(&"after committed kernel")
        ),
    }
    assert_eq!(authority(), before);
    let retry = output("b", 1);
    fresh();
    output("a", 0);
    assert_eq!(output("b", 1), retry);
}

#[test]
#[serial]
fn rollback_restores_latch_when_committed_inner_state_is_then_damaged() {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    let before = authority();
    // When
    let result: Result<(), String> = exact_test_support::exact(|| {
        output("b", 1);
        with_style_sheet_mut(|sheet| {
            sheet.properties.clear();
            sheet.add_css("literal", "body{}");
        });
        assert!(matches!(
            with_style_sheet(snapshot6::check_install_target),
            Err(EvidenceError::ActiveExact(_))
        ));
        Err("abort latched inner".into())
    });
    // Then
    assert_eq!(result, Err("abort latched inner".into()));
    assert_eq!(authority(), before);
    assert_eq!(with_style_sheet(snapshot6::check_install_target), Ok(()));
    let retry = output("b", 1);
    fresh();
    output("a", 0);
    assert_eq!(output("b", 1), retry);
}

#[test]
#[serial]
fn existing_rejected_state_is_captured_when_enclosing_attempt_aborts() {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    with_style_sheet_mut(|sheet| {
        sheet.properties.clear();
        sheet.add_css("literal", "body{}");
    });
    let before = authority();
    let rejection = with_style_sheet(snapshot6::check_install_target);
    assert!(matches!(rejection, Err(EvidenceError::Kernel(_))));
    // When
    let result: Result<(), String> = exact_test_support::exact(|| {
        with_style_sheet_mut(|sheet| *sheet = StyleSheet::default());
        output("new", 1);
        Err("rejected outer".into())
    });
    // Then
    assert_eq!(result, Err("rejected outer".into()));
    assert_eq!(authority(), before);
    assert_eq!(with_style_sheet(snapshot6::check_install_target), rejection);
}

#[test]
#[serial]
fn finish_failure_restores_guarded_cleanup_when_real_base_owner_was_registered() {
    // Given
    let _guard = Guard::new();
    let items = with_original(
        FixtureRequest {
            filename: "a",
            source: "abcdef",
        },
        || {
            FxHashSet::from_iter([ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
                "color",
                0,
                "tone",
                Some(css::style_selector::StyleSelector::Global(
                    "body".into(),
                    "a".into(),
                )),
            ))])
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
    let before = authority();
    // When
    let result = with_admission(|| {
        let rollback = extraction_rollback::ExtractionRollback::capture();
        let result: Result<(), UpdateError<String>> =
            css::exact_attempt::with_exclusive_attempt(|| {
                with_style_sheet_mut(|sheet| {
                    CounterSheet::new(sheet).with_attempt(|attempt| {
                        attempt
                            .prepare(
                                &Default::default(),
                                UpdateRequest {
                                    raw_source: "a",
                                    single_css: false,
                                },
                            )?
                            .finish(|sheet, effects| {
                                assert!(effects.default_collected);
                                let _output = Output::from_prospective(
                                    metadata(),
                                    ProspectiveSheet { sheet, effects },
                                    OutputRoute {
                                        raw_source: "a",
                                        single_css: false,
                                        import_main_css: false,
                                    },
                                );
                                Err("after real cleanup".into())
                            })
                    })
                })
            });
        drop(rollback);
        result
    });
    // Then
    assert_eq!(
        result,
        Err(UpdateError::Output("after real cleanup".into()))
    );
    assert_eq!(authority(), before);
}

#[test]
#[serial]
fn final_validation_restores_output_when_map_authority_changes_after_finish() {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    let items = ir("b", 1);
    let before = authority();
    // When
    let result: Result<Output, UpdateError<String>> = with_admission(|| {
        let rollback = extraction_rollback::ExtractionRollback::capture();
        let result = css::exact_attempt::with_exclusive_attempt(|| {
            with_style_sheet_mut(|sheet| {
                CounterSheet::new(sheet).with_attempt(|attempt| {
                    let completed = attempt
                        .prepare(
                            &items,
                            UpdateRequest {
                                raw_source: "b",
                                single_css: false,
                            },
                        )?
                        .finish(|sheet, effects| {
                            Ok(Output::from_prospective(
                                metadata(),
                                ProspectiveSheet { sheet, effects },
                                OutputRoute {
                                    raw_source: "b",
                                    single_css: false,
                                    import_main_css: false,
                                },
                            ))
                        })?;
                    css::class_map::with_class_map_mut(HashMap::clear);
                    Ok(completed)
                })
            })
        });
        drop(rollback);
        result
    });
    // Then
    assert!(matches!(result, Err(UpdateError::Kernel(_))));
    assert_eq!(authority(), before);
}
