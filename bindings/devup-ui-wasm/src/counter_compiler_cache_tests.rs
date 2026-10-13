use super::counter_compiler_tests::*;

#[rstest]
#[serial]
fn genuine_companions_preserve_or_retire_when_supplied_before_and_after_work(
    #[values(0, 1, 2, 3)] supplied: u8,
    #[values(false, true)] before: bool,
    #[values(false, true)] reversed: bool,
    #[values(false, true)] invalid: bool,
) {
    with_admission(|| {
        // Given: real resolver supplies an unused reservation as well as emitted root.
        let _guard = Guard::new();
        let cached = required(code_extract_counter(reserved_input(), false));
        assert!(cached.code().contains("root"));
        let bytes = encoded();
        let expected_maps = cache6_session::Maps::capture();
        let expected_companions = maps();
        with_style_sheet_mut(|s| s.source_ids = css::file_map::get_original_ids());
        let expected_sheet = with_style_sheet(LiveCheckpoint::capture);
        fresh();
        let cold = authority();
        let send = || {
            for classes in if reversed {
                [false, true]
            } else {
                [true, false]
            } {
                if supplied == 3 || supplied == if classes { 1 } else { 2 } {
                    assert_eq!(
                        if classes {
                            cache6_restore::classes(
                                (!invalid).then(|| expected_companions.0.clone()),
                            )
                        } else {
                            cache6_restore::files((!invalid).then(|| expected_companions.1.clone()))
                        },
                        Ok(())
                    );
                }
            }
        };
        if before {
            send();
        }
        let (prior_maps, prior_sheet) = cache6_session::Restore::before();
        // When
        assert_eq!(cache6_restore::import(&bytes), Ok(()));
        if !before {
            send();
        }
        // Then: full adoption or retirement is observed BEFORE any compile.
        if invalid && supplied != 0 {
            assert_eq!(authority(), cold);
            cache6_restore::RESTORE.with_borrow(|s| assert!(s.session.is_none()));
        } else {
            assert_eq!(cache6_session::Maps::capture(), expected_maps);
            assert_eq!(with_style_sheet(LiveCheckpoint::capture), expected_sheet);
            let expected = cache6_session::Restore {
                session: Some(cache6_session::Session::installed(prior_maps, prior_sheet)),
                ..Default::default()
            };
            cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &expected));
        }
        assert_eq!(cache_names::check(), Ok(()));
        let replay = required(code_extract_counter(reserved_input(), false));
        let replay = ObservedOutput::capture(&replay, "parent.css.ts");
        fresh();
        if !(invalid && supplied != 0) {
            required(code_extract_counter(reserved_input(), false));
        }
        let clean = required(code_extract_counter(reserved_input(), false));
        assert_eq!(ObservedOutput::capture(&clean, "parent.css.ts"), replay);
    });
}

#[rstest]
#[serial]
fn genuine_session_keeps_seed_batches_when_late_companions_match_or_retire(
    #[values(false, true)] classes: bool,
    #[values(false, true)] matching: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        required(code_extract_counter(reserved_input(), false));
        let bytes = encoded();
        let original = maps();
        let seeds = vec![
            vec!["b".into(), "b".into()],
            vec!["a".into(), "b".into()],
            vec!["c".into(), "a".into(), "c".into()],
        ];
        fresh();
        let (prior_maps, prior_sheet) = cache6_session::Restore::before();
        assert_eq!(cache6_restore::import(&bytes), Ok(()));
        let mut session = cache6_session::Session::installed(prior_maps, prior_sheet);
        for batch in &seeds {
            seed_file_map(batch.clone());
        }
        session.seeds.clone_from(&seeds);
        let expected = cache6_session::Restore {
            session: Some(session),
            ..Default::default()
        };
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &expected));
        let source = "import {css} from '@devup-ui/react'; const a=css({color:'blue'});";
        compile("fresh.css.ts", source);
        let extended = authority();
        assert_ne!(maps(), original);
        // When
        assert_eq!(
            if classes {
                cache6_restore::classes(Some(if matching { original.0 } else { maps().0 }))
            } else {
                cache6_restore::files(Some(if matching { original.1 } else { maps().1 }))
            },
            Ok(())
        );
        // Then
        if matching {
            assert_eq!(authority(), extended);
            cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &expected));
        } else {
            cache6_restore::RESTORE.with_borrow(|s| assert!(s.session.is_none()));
            let retired = authority();
            let retry = compile("fresh.css.ts", source);
            let retry_maps = cache6_session::Maps::capture();
            fresh();
            for batch in seeds {
                seed_file_map(batch);
            }
            assert_eq!(authority(), retired);
            assert_eq!(compile("fresh.css.ts", source), retry);
            assert_eq!(cache6_session::Maps::capture(), retry_maps);
        }
    });
}

#[test]
#[serial]
fn nonempty_real_js6_is_adopted_when_import_precedes_recompile() {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        configure_typography();
        let source = "import {Box,keyframes} from '@devup-ui/react'; const k=keyframes({from:{opacity:0},to:{opacity:1}}); const a=<Box color='red' width={size} typography='heading' animation={`${k} 1s`}/>;";
        let clean = compile("a.tsx", source);
        assert_ne!(with_style_sheet(|s| s.properties.len()), 0);
        assert_ne!(with_style_sheet(|s| s.keyframes.len()), 0);
        let bytes = encoded();
        let expected_maps = cache6_session::Maps::capture();
        with_style_sheet_mut(|s| s.source_ids = css::file_map::get_original_ids());
        let expected_sheet = with_style_sheet(LiveCheckpoint::capture);
        fresh();
        configure_typography();
        let (before_maps, before_sheet) = cache6_session::Restore::before();
        // When
        assert_eq!(cache6_restore::import(&bytes), Ok(()));
        // Then: equality precedes any recompile.
        assert_eq!(cache6_session::Maps::capture(), expected_maps);
        assert_eq!(with_style_sheet(LiveCheckpoint::capture), expected_sheet);
        let expected = cache6_session::Restore {
            session: Some(cache6_session::Session::installed(
                before_maps,
                before_sheet,
            )),
            ..Default::default()
        };
        cache6_restore::RESTORE.with_borrow(|state| assert_eq!(state, &expected));
        let replay = compile("a.tsx", source);
        fresh();
        configure_typography();
        compile("a.tsx", source);
        assert_eq!(compile("a.tsx", source), replay);
        assert_ne!(clean, replay);
    });
}
