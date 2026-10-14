use super::cache6_test_support::*;

#[rstest]
#[case(false, false)]
#[case(false, true)]
#[case(true, false)]
#[case(true, true)]
#[serial]
fn complete_restore_works_when_companions_arrive_in_either_order(
    #[case] before: bool,
    #[case] reversed: bool,
) {
    // Given
    let _guard = Guard::new();
    output("a", 1);
    let bytes = encoded();
    let expected = maps();
    fresh();
    if before {
        companions(&expected, reversed);
    }
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    if !before {
        companions(&expected, reversed);
    }
    // Then
    assert_eq!(maps(), expected);
    assert_eq!(encoded(), bytes);
    cache6_restore::RESTORE.with_borrow(|state| assert!(state.session.is_some()));
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[serial]
fn restore_accepts_optional_companions_when_only_one_or_neither_is_supplied(#[case] supplied: u8) {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    let bytes = encoded();
    let expected = maps();
    fresh();
    match supplied {
        0 => {}
        1 => {
            assert_eq!(cache6_restore::classes(Some(expected.0.clone())), Ok(()));
        }
        2 => {
            assert_eq!(cache6_restore::files(Some(expected.1.clone())), Ok(()));
        }
        _ => panic!("supplied"),
    }
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    // Then
    assert_eq!(maps(), expected);
}

#[rstest]
#[case(false, false, false)]
#[case(false, true, false)]
#[case(true, false, false)]
#[case(true, true, false)]
#[case(false, false, true)]
#[case(false, true, true)]
#[case(true, false, true)]
#[case(true, true, true)]
#[serial]
fn invalid_companion_leaves_exact_cold_state_when_supplied_before_or_after(
    #[case] before: bool,
    #[case] classes: bool,
    #[case] unequal: bool,
) {
    // Given
    let _guard = Guard::new();
    output("cached", 0);
    let bytes = encoded();
    fresh();
    with_style_sheet_mut(|sheet| {
        sheet.add_css("sentinel", concat!("body", "{", "color:blue", "}"))
    });
    seed_file_map(vec!["sentinel".into()]);
    let expected = authority();
    let invalid = || {
        if classes {
            cache6_restore::classes(unequal.then(BTreeMap::new))
        } else {
            cache6_restore::files(unequal.then(BTreeMap::new))
        }
    };
    if before {
        assert_eq!(invalid(), Ok(()));
    }
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    if !before {
        assert_eq!(invalid(), Ok(()));
    }
    // Then
    assert_eq!(authority(), expected);
    assert_eq!(cache_names::check(), Ok(()));
    cache6_restore::RESTORE.with_borrow(|state| assert!(state.session.is_none()));
}

#[test]
#[serial]
fn companions_never_install_when_no_sheet_was_supplied() {
    // Given
    let _guard = Guard::new();
    let before = authority();
    // When
    companions(
        &(
            BTreeMap::from([("poison".into(), BTreeMap::from([("wrong".into(), 77)]))]),
            BTreeMap::from([("poison".into(), 88)]),
        ),
        true,
    );
    // Then
    assert_eq!(authority(), before);
    cache6_restore::RESTORE.with_borrow(|state| assert!(state.session.is_none()));
}

#[test]
#[serial]
fn late_matching_companions_preserve_fresh_work_when_live_maps_have_extended() {
    // Given
    let _guard = Guard::new();
    output("cached", 0);
    let bytes = encoded();
    let expected = maps();
    fresh();
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    output("fresh", 1);
    let extended = authority();
    assert_ne!(maps(), expected);
    // When
    companions(&expected, true);
    // Then
    assert_eq!(authority(), extended);
    cache6_restore::RESTORE.with_borrow(|state| assert!(state.session.is_some()));
}

#[test]
#[serial]
fn reset_forgets_session_when_another_build_begins() {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    let bytes = encoded();
    fresh();
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    // When
    reset_build_state_internal();
    // Then
    cache6_restore::RESTORE
        .with_borrow(|state| assert_eq!(state, &cache6_session::Restore::default()));
    assert_eq!(
        with_style_sheet(LiveCheckpoint::capture),
        LiveCheckpoint::capture(&StyleSheet::default())
    );
}

#[test]
#[serial]
fn clear_forgets_only_protocol_when_an_inactive_genuine_session_has_fresh_work() {
    // Given: prior state differs from installed and subsequently extended live state.
    let _guard = Guard::new();
    output("cached", 1);
    let bytes = encoded();
    fresh();
    output("prior", 0);
    let prior = authority();
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    output("fresh", 2);
    seed_file_map(vec!["journal".into(), "journal".into()]);
    let live = authority();
    assert_ne!(live, prior);
    cache6_restore::RESTORE.with_borrow(|state| assert!(state.session.is_some()));
    // When
    let result = cache6_restore::clear();
    // Then: forgetting must not retire/replay or restore the before-checkpoint/maps.
    assert_eq!(result, Ok(()));
    assert_eq!(authority(), live);
    cache6_restore::RESTORE
        .with_borrow(|state| assert_eq!(state, &cache6_session::Restore::default()));
}
