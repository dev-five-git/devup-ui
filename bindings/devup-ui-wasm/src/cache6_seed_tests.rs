use super::cache6_test_support::*;

fn batches() {
    seed_file_map(vec!["b".into(), "b".into()]);
    seed_file_map(vec!["a".into(), "b".into()]);
    seed_file_map(vec!["c".into(), "a".into(), "c".into()]);
}

#[test]
#[serial]
fn seeded_journals_only_when_an_inactive_genuine_session_is_installed() {
    // Given
    let _guard = Guard::new();
    output("cached", 1);
    let bytes = encoded();
    fresh();
    output("prior", 0);
    let (before_maps, before_sheet) = cache6_session::Restore::before();
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    let mut expected_session = cache6_session::Session::installed(before_maps, before_sheet);
    let before = authority();
    let seeds = vec![
        vec!["b".into(), "b".into()],
        vec!["a".into(), "b".into()],
        vec!["c".into(), "a".into(), "c".into()],
    ];
    // When
    for batch in &seeds {
        assert_eq!(cache6_restore::seeded(batch), Ok(()));
    }
    // Then: no allocator, sheet, private Counter state or map mutation.
    assert_eq!(authority(), before);
    expected_session.seeds = seeds;
    let expected = cache6_session::Restore {
        session: Some(expected_session),
        ..Default::default()
    };
    cache6_restore::RESTORE.with_borrow(|state| assert_eq!(state, &expected));
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn seeded_batches_replay_when_a_late_invalid_companion_retires_the_session(#[case] classes: bool) {
    // Given: independent cold replay applies seeds through the public allocator entry.
    let _guard = Guard::new();
    output("cached", 1);
    let bytes = encoded();
    fresh();
    output("prior", 0);
    batches();
    let expected = authority();
    fresh();
    output("prior", 0);
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    for batch in [
        vec!["b".into(), "b".into()],
        vec!["a".into(), "b".into()],
        vec!["c".into(), "a".into(), "c".into()],
    ] {
        assert_eq!(cache6_restore::seeded(&batch), Ok(()));
    }
    output("fresh", 2);
    // When
    let result = if classes {
        cache6_restore::classes(None)
    } else {
        cache6_restore::files(None)
    };
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(authority(), expected);
    let expected_restore = cache6_session::Restore {
        classes: if classes {
            cache6_session::Companion::Invalid
        } else {
            cache6_session::Companion::Unseen
        },
        files: if classes {
            cache6_session::Companion::Unseen
        } else {
            cache6_session::Companion::Invalid
        },
        session: None,
    };
    cache6_restore::RESTORE.with_borrow(|state| assert_eq!(state, &expected_restore));
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn late_rollback_replays_seed_calls_when_batches_repeat_and_current_canonical_changes(
    #[case] classes: bool,
) {
    // Given
    let _guard = Guard::new();
    import_canonical_map_internal(HashMap::from([("a".into(), "root".into())]));
    batches();
    let cold_maps = cache6_session::Maps::capture();
    let cold = output("a", 1);
    fresh();
    update(
        &Default::default(),
        OutputRoute {
            raw_source: "empty",
            single_css: true,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let bytes = encoded();
    fresh();
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    batches();
    import_canonical_map_internal(HashMap::from([("a".into(), "root".into())]));
    output("fresh", 0);
    // When
    let result = if classes {
        cache6_restore::classes(None)
    } else {
        cache6_restore::files(None)
    };
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(cache6_session::Maps::capture(), cold_maps);
    assert_eq!(output("a", 1), cold);
    assert_eq!(
        css::file_map::get_original_ids(),
        BTreeMap::from([("b".into(), 0), ("a".into(), 1), ("c".into(), 2)])
    );
}

#[test]
#[serial]
fn repeated_restore_keeps_fresh_seeds_when_old_session_is_retired() {
    // Given
    let _guard = Guard::new();
    update(
        &Default::default(),
        OutputRoute {
            raw_source: "empty",
            single_css: true,
            import_main_css: false,
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let bytes = encoded();
    fresh();
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    batches();
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    assert_eq!(cache6_restore::files(None), Ok(()));
    // Then
    assert_eq!(
        css::file_map::get_original_ids(),
        BTreeMap::from([("b".into(), 0), ("a".into(), 1), ("c".into(), 2)])
    );
}
