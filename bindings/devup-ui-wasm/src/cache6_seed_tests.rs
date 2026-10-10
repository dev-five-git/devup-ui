use super::cache6_test_support::*;

fn batches() {
    seed_file_map(vec!["b".into(), "b".into()]);
    seed_file_map(vec!["a".into(), "b".into()]);
    seed_file_map(vec!["c".into(), "a".into(), "c".into()]);
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
