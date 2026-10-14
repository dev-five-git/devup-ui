use super::counter_compiler_tests::*;

#[test]
#[serial]
fn consumer_error_is_transported_when_genuine_prior_records_are_missing() {
    with_admission(|| {
        // Given: retain genuine compiler authority but remove its materialized records.
        let _guard = Guard::new();
        required(code_extract_counter(reserved_input(), false));
        with_style_sheet_mut(|s| s.properties.clear());
        let before = authority();
        // When: real compilation precedes checked publication of the invalid BASE.
        let error = code_extract_counter(reserved_input(), false)
            .err()
            .unwrap_or_else(|| panic!("missing prior records must fail"));
        // Then: preserve the actual kernel category and complete pre-attempt state.
        assert_eq!(error, "dormant counter kernel: Coverage");
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &Default::default()));
    });
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn final_rejects_when_only_reservation_key_or_slot_is_damaged_after_finish(#[case] remove: bool) {
    with_admission(|| {
        // Given: independent genuine parser observation aborts without granting batch authority.
        let _guard = Guard::new();
        let address = reservation(reserved_input());
        let before = authority();
        let restore = cache6_session::Restore::default();
        // When
        let result = after_finish(reserved_input(), || {
            damage_reservation(&address, remove);
            Ok::<_, String>(())
        });
        // Then
        assert!(matches!(
            result,
            Err(CounterCompileError::Consumer(UpdateError::Kernel(
                KernelError::Authority
            )))
        ));
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|state| assert_eq!(state, &restore));
        let retry = code_extract_counter(reserved_input(), false).unwrap_or_else(|e| panic!("{e}"));
        let retry = ObservedOutput::capture(&retry, "parent.css.ts");
        let retry_maps = cache6_session::Maps::capture();
        fresh();
        let clean = code_extract_counter(reserved_input(), false).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(ObservedOutput::capture(&clean, "parent.css.ts"), retry);
        assert_eq!(cache6_session::Maps::capture(), retry_maps);
        with_style_sheet(|sheet| {
            assert!(!sheet.create_css(None, false).contains("magenta"));
            assert!(
                !sheet
                    .create_css(Some("parent.css.ts"), false)
                    .contains("magenta")
            );
        });
    });
}

#[rstest]
#[serial]
fn installed_session_rolls_back_when_auxiliary_damage_occurs_within_or_after_finish(
    #[values(false, true)] within: bool,
    #[values(false, true)] remove: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        compile("cached.tsx", FAMILY_SOURCES[0]);
        let proof = exported();
        fresh();
        let restore = install(&proof);
        let address = reservation(reserved_input());
        let before = authority();
        // When
        let result = if within {
            let rollback = extraction_rollback::ExtractionRollback::capture();
            let result = css::exact_attempt::with_exclusive_attempt(|| {
                with_counter_extract(reserved_input(), |metadata, batch| {
                    with_style_sheet_mut(|s| {
                        CounterSheet::new(s).with_attempt(|attempt| {
                            attempt
                                .prepare_compiled(
                                    batch,
                                    UpdateRequest {
                                        raw_source: "parent.css.ts",
                                        single_css: false,
                                    },
                                )?
                                .finish(|sheet, effects| {
                                    let output = Output::from_prospective(
                                        OutputMetadata {
                                            code: metadata.code,
                                            map: metadata.map,
                                            css_file: metadata.css_file,
                                            dependencies: metadata.dependencies,
                                        },
                                        ProspectiveSheet { sheet, effects },
                                        OutputRoute {
                                            raw_source: "parent.css.ts",
                                            single_css: false,
                                            import_main_css: false,
                                        },
                                    );
                                    damage_reservation(&address, remove);
                                    Ok::<_, String>(output)
                                })
                        })
                    })
                })
            });
            drop(rollback);
            result
        } else {
            after_finish(reserved_input(), || {
                damage_reservation(&address, remove);
                Ok::<_, String>(())
            })
        };
        // Then
        assert!(matches!(
            result,
            Err(CounterCompileError::Consumer(UpdateError::Kernel(
                KernelError::Authority
            )))
        ));
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &restore));
        let replay = required(code_extract_counter(reserved_input(), false));
        let replay = ObservedOutput::capture(&replay, "parent.css.ts");
        let replay_maps = cache6_session::Maps::capture();
        fresh();
        compile("cached.tsx", FAMILY_SOURCES[0]);
        let clean = required(code_extract_counter(reserved_input(), false));
        assert_eq!(ObservedOutput::capture(&clean, "parent.css.ts"), replay);
        assert_eq!(cache6_session::Maps::capture(), replay_maps);
    });
}

#[test]
#[serial]
fn located_error_restores_session_when_raw_registration_precedes_failure() {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        compile("cached.tsx", FAMILY_SOURCES[0]);
        let proof = exported();
        fresh();
        let restore = install(&proof);
        let before = authority();
        let registered = std::rc::Rc::new(std::cell::Cell::new(false));
        let observed_registration = registered.clone();
        let resolver = move |_: &str, _: &str| {
            registered.set(css::file_map::original_id("broken.tsx").is_some());
            Some(ResolvedModule {
                path: "values.ts".into(),
                code: "export const tone='red';".into(),
            })
        };
        let mut request = input(
            "broken.tsx",
            concat!(
                "import {tone} from './values';import {globalCss} from '@devup-ui/react';\nglobalCss(",
                "{",
                "'.good':",
                "{",
                "color:tone",
                "}",
                "}",
                ");\nglobalCss({'.bad':{color:runtime()}});"
            ),
        );
        request.resolver = Some(&resolver);
        // When
        let result = code_extract_counter(request, false);
        // Then
        let error = result.err().unwrap_or_else(|| panic!("located error"));
        println!("located={error}");
        assert!(observed_registration.get());
        assert!(error.contains("broken.tsx:3:"));
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &restore));
        let retry = compile("retry.tsx", FAMILY_SOURCES[0]);
        let retry_maps = cache6_session::Maps::capture();
        fresh();
        compile("cached.tsx", FAMILY_SOURCES[0]);
        assert_eq!(compile("retry.tsx", FAMILY_SOURCES[0]), retry);
        assert_eq!(cache6_session::Maps::capture(), retry_maps);
    });
}

#[rstest]
#[serial]
fn raw_css_cleanup_rolls_back_once_when_outer_errors_or_unwinds(
    #[values(false, true)] unwind: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        let source =
            "import {globalStyle} from '@devup-ui/react';globalStyle('.root',{color:'red'});";
        compile("root.css.ts", source);
        let before = authority();
        // When
        let result = catch_unwind(AssertUnwindSafe(|| {
            after_finish(input("root.css.ts", source), || {
                assert!(!unwind, "cleanup unwind");
                Err("cleanup error")
            })
        }));
        // Then
        match result {
            Ok(result) => assert!(matches!(
                result,
                Err(CounterCompileError::Consumer(UpdateError::Output(
                    "cleanup error"
                )))
            )),
            Err(payload) => assert_eq!(payload.downcast_ref::<&str>(), Some(&"cleanup unwind")),
        }
        assert_eq!(authority(), before);
        let retry = compile("root.css.ts", source);
        let retry_maps = cache6_session::Maps::capture();
        fresh();
        compile("root.css.ts", source);
        assert_eq!(compile("root.css.ts", source), retry);
        assert_eq!(cache6_session::Maps::capture(), retry_maps);
    });
}
