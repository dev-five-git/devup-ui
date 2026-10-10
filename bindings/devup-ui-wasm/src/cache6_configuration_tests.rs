use super::cache6_test_support::*;

#[path = "cache6_transition_tests.rs"]
mod transitions;

#[rstest]
#[case(false, 0)]
#[case(false, 1)]
#[case(false, 2)]
#[case(false, 3)]
#[case(false, 4)]
#[case(true, 0)]
#[case(true, 1)]
#[case(true, 2)]
#[case(true, 3)]
#[case(true, 4)]
#[serial]
fn active_ingress_rejects_immediately_when_exact_depth_is_one_or_two(
    #[case] nested: bool,
    #[case] ingress: u8,
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
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    output("new", 1);
    seed_file_map(vec!["journal".into()]);
    let before = authority();
    // When
    cache6_restore::RESTORE.with_borrow(|before_session| {
        let refusal = || {
            let result = match ingress {
                0 => cache6_restore::import(b"malformed"),
                1 => cache6_restore::classes(None),
                2 => cache6_restore::files(Some(BTreeMap::from([("wrong".into(), 77)]))),
                3 => cache6_restore::seeded(&["wrong".into()]),
                4 => cache6_restore::clear(),
                _ => panic!("ingress"),
            };
            // Then: observe inside the deepest callback, before CSS Err rollback.
            assert_eq!(
                result,
                Err(EvidenceError::ActiveExact(
                    css::admission::ActiveExactAttempt
                ))
            );
            assert_eq!(authority(), before);
            cache6_restore::RESTORE.with_borrow(|state| assert_eq!(state, before_session));
            result
        };
        let result = css::exact_attempt::with_exclusive_attempt(|| {
            if nested {
                css::exact_attempt::with_exclusive_attempt(refusal)
            } else {
                refusal()
            }
        });
        assert_eq!(
            result,
            Err(EvidenceError::ActiveExact(
                css::admission::ActiveExactAttempt
            ))
        );
    });
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[serial]
fn install_rechecks_actual_authority_when_validation_precedes_configuration_change(
    #[case] change: u8,
) {
    // Given
    let _guard = Guard::new();
    output("cached", 0);
    let bytes = encoded();
    let certificate = snapshot6::validate_snapshot(
        snapshot6::parse(&bytes).unwrap_or_else(|error| panic!("{error:?}")),
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    fresh();
    with_style_sheet_mut(|sheet| {
        sheet.add_css("sentinel", concat!("body", "{", "color:blue", "}"))
    });
    seed_file_map(vec!["sentinel".into()]);
    match change {
        0 => set_prefix(Some("different".into())),
        1 => set_debug(true),
        2 => set_atom_hoist(Some(2)),
        3 => css::atom_hoist::restore_atom_plan(Some(BTreeSet::new())),
        _ => panic!("change"),
    }
    let before = authority();
    // When
    let result = with_style_sheet_mut(|sheet| certificate.install_into(sheet));
    // Then
    assert_eq!(result, Err(EvidenceError::Configuration));
    assert_eq!(authority(), before);
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[serial]
fn replacement_cannot_clear_target_latch_when_a_restore_session_exists(#[case] ingress: u8) {
    // Given
    let _guard = Guard::new();
    output("a", 0);
    let bytes = encoded();
    fresh();
    with_style_sheet_mut(|sheet| sheet.add_css("prior", concat!("body", "{", "color:blue", "}")));
    seed_file_map(vec!["prior-seed".into()]);
    let (before_maps, before_sheet) = cache6_session::Restore::before();
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    let mut expected_session = cache6_session::Session::installed(before_maps, before_sheet);
    let seeds = vec![
        vec!["journal-b".into(), "journal-b".into()],
        vec!["journal-a".into(), "journal-b".into()],
    ];
    for batch in &seeds {
        seed_file_map(batch.clone());
    }
    expected_session.seeds = seeds;
    let expected_restore = cache6_session::Restore {
        session: Some(expected_session),
        ..Default::default()
    };
    with_style_sheet_mut(|sheet| {
        sheet.properties.clear();
        sheet.add_css("literal", "body{}");
    });
    let before = authority();
    let rejection = with_style_sheet(snapshot6::check_install_target);
    assert!(matches!(rejection, Err(EvidenceError::Kernel(_))));
    // When
    let result = match ingress {
        0 => cache6_restore::import(&bytes),
        1 => cache6_restore::import(b"bad"),
        2 => cache6_restore::classes(None),
        3 => cache6_restore::files(None),
        4 => cache6_restore::classes(Some(BTreeMap::new())),
        5 => cache6_restore::files(Some(BTreeMap::new())),
        _ => panic!("ingress"),
    };
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(authority(), before);
    assert_eq!(with_style_sheet(snapshot6::check_install_target), rejection);
    cache6_restore::RESTORE.with_borrow(|state| assert_eq!(state, &expected_restore));
}

#[test]
#[serial]
fn fresh_build_settings_survive_adoption_and_late_rejection_when_cached_routing_differs() {
    // Given
    let _guard = Guard::new();
    configure_typography();
    output("a", 3);
    let bytes = encoded();
    fresh();
    let theme = sheet::theme::Theme {
        breakpoints: vec![19, 31],
        ..Default::default()
    };
    register_theme_internal(theme);
    register_shorthands_internal(BTreeMap::from([("freshInset".into(), vec!["left".into()])]));
    import_canonical_map_internal(HashMap::from([("a".into(), "fresh-root".into())]));
    import_file_routes_internal(HashMap::from([(
        "a".into(),
        std::collections::HashSet::from([9]),
    )]));
    let theme_css = with_style_sheet(|sheet| sheet.theme.to_css());
    // When
    assert_eq!(cache6_restore::import(&bytes), Ok(()));
    assert_eq!(cache6_restore::files(None), Ok(()));
    // Then
    assert_eq!(with_style_sheet(|sheet| sheet.theme.to_css()), theme_css);
    assert_eq!(css::file_map::canonical("a"), "fresh-root");
    assert_eq!(
        css::get_custom_shorthand_names(),
        vec!["freshInset".to_string()]
    );
    assert_eq!(get_prefix(), None);
    assert!(!is_debug());
    assert_eq!(css::atom_hoist::atom_hoist_threshold(), None);
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "fresh-tokens.ts".into(),
            code: "export const COLOR='blue';".into(),
        })
    };
    let result = code_extract_with_modules_internal("fresh.ts", concat!("import {css} from '@devup-ui/react';import {COLOR} from './tokens';export const value=css(", "{", "color:COLOR", "}", ");"),
        "@devup-ui/react", "df".into(), false, false, false, HashMap::new(), &resolver)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        result
            .css()
            .unwrap_or_else(|| panic!("css"))
            .contains("color:blue")
    );
    register_shorthands_internal(BTreeMap::new());
}
