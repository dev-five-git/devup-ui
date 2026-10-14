use super::*;

#[rstest]
#[serial]
fn actual_js6_preserves_fresh_authority_when_each_setting_changes_independently(
    #[values(
        Change::Theme,
        Change::Prefix,
        Change::Debug,
        Change::Threshold,
        Change::Plan,
        Change::Canonical,
        Change::Routes,
        Change::Shorthand
    )]
    change: Change,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        configure_typography();
        compile("a.tsx", FAMILY_SOURCES[4]);
        let proof = exported();
        fresh();
        changed(change);
        let cold = authority();
        let theme = with_style_sheet(|s| s.theme.to_css());
        let cold_required = matches!(
            change,
            Change::Prefix | Change::Debug | Change::Threshold | Change::Plan
        );
        // When
        if cold_required {
            assert_eq!(cache6_restore::import(&proof.0), Ok(()));
            assert_eq!(authority(), cold);
            cache6_restore::RESTORE
                .with_borrow(|s| assert_eq!(s, &cache6_session::Restore::default()));
        } else {
            install(&proof);
        }
        // Then
        assert_eq!(with_style_sheet(|s| s.theme.to_css()), theme);
        let fresh_source =
            "import {Box} from '@devup-ui/react';const a=<Box color='blue' freshInset={2}/>;";
        let replay = compile("fresh.tsx", fresh_source);
        let replay_maps = cache6_session::Maps::capture();
        fresh();
        if !cold_required {
            configure_typography();
            compile("a.tsx", FAMILY_SOURCES[4]);
            register_theme_internal(sheet::theme::Theme::default());
        }
        changed(change);
        assert_eq!(compile("fresh.tsx", fresh_source), replay);
        assert_eq!(cache6_session::Maps::capture(), replay_maps);
        register_shorthands_internal(BTreeMap::new());
    });
}

#[rstest]
#[serial]
fn actual_js6_ingress_is_unchanged_when_exact_depth_is_one_or_two(
    #[values(false, true)] nested: bool,
    #[values(0, 1, 2, 3, 4)] ingress: u8,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        required(code_extract_counter(reserved_input(), false));
        let proof = exported();
        let companions = maps();
        fresh();
        let restore = install(&proof);
        let before = authority();
        let refuse = || {
            let result = match ingress {
                0 => cache6_restore::import(&proof.0),
                1 => cache6_restore::classes(Some(companions.0.clone())),
                2 => cache6_restore::files(Some(companions.1.clone())),
                3 => cache6_restore::seeded(&["other".into()]),
                4 => cache6_restore::clear(),
                _ => panic!("ingress"),
            };
            assert_eq!(
                result,
                Err(EvidenceError::ActiveExact(
                    css::admission::ActiveExactAttempt
                ))
            );
            assert_eq!(authority(), before);
            cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &restore));
            result
        };
        // When
        let result = css::exact_attempt::with_exclusive_attempt(|| {
            if nested {
                css::exact_attempt::with_exclusive_attempt(refuse)
            } else {
                refuse()
            }
        });
        // Then
        assert_eq!(
            result,
            Err(EvidenceError::ActiveExact(
                css::admission::ActiveExactAttempt
            ))
        );
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &restore));
    });
}

#[rstest]
#[serial]
fn valid_js6_cannot_clear_original_target_latch_when_session_is_installed(
    #[values(0, 1, 2)] ingress: u8,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        compile("a.tsx", FAMILY_SOURCES[0]);
        let proof = exported();
        let companions = maps();
        fresh();
        let restore = install(&proof);
        with_style_sheet_mut(|s| s.properties.clear());
        with_style_sheet_mut(|s| s.add_css("literal", "body{}"));
        let latch: Result<(), UpdateError<String>> = with_style_sheet_mut(|s| {
            CounterSheet::new(s).with_attempt(|_| Err(UpdateError::Output("unreachable".into())))
        });
        assert!(matches!(latch, Err(UpdateError::Kernel(_))));
        let rejection = with_style_sheet(snapshot6::check_install_target);
        assert!(matches!(rejection, Err(EvidenceError::Kernel(_))));
        let before = authority();
        // When
        let result = match ingress {
            0 => cache6_restore::import(&proof.0),
            1 => cache6_restore::classes(Some(companions.0)),
            2 => cache6_restore::files(Some(companions.1)),
            _ => panic!("ingress"),
        };
        // Then
        assert_eq!(result, Ok(()));
        assert_eq!(authority(), before);
        assert_eq!(with_style_sheet(snapshot6::check_install_target), rejection);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &restore));
    });
}

#[rstest]
#[serial]
fn genuine_certificate_rechecks_when_live_config_changes_after_validation(
    #[values(Change::Prefix, Change::Debug, Change::Threshold, Change::Plan)] change: Change,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        compile("a.tsx", FAMILY_SOURCES[0]);
        let bytes = encoded();
        let certificate = required(snapshot6::parse(&bytes).and_then(snapshot6::validate_snapshot));
        fresh();
        changed(change);
        let before = authority();
        // When
        let result = with_style_sheet_mut(|s| certificate.install_into(s));
        // Then
        assert_eq!(result, Err(EvidenceError::Configuration));
        assert_eq!(authority(), before);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &cache6_session::Restore::default()));
    });
}

#[rstest]
#[serial]
fn nested_stylesheet_compiles_when_resolver_reenters_evaluating_outer(
    #[values("style")] api: &str,
    #[values(false, true)] current: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        let source = format!(
            "import {{{api}}} from '@devup-ui/react';export const a={api}({{color:'blue'}});"
        );
        required(css::file_map::get_or_insert_original_id("outer.css.ts"));
        let direct = if current {
            required(extractor::extract(
                "nested.css.ts",
                &source,
                ExtractOption::default(),
            ))
            .code
        } else {
            required(code_extract_counter(input("nested.css.ts", &source), false)).code()
        };
        fresh();
        let errors = std::rc::Rc::new(RefCell::new(Vec::new()));
        let outputs = std::rc::Rc::new(RefCell::new(Vec::new()));
        let observed_errors = errors.clone();
        let observed_outputs = outputs.clone();
        let resolver = move |_: &str, _: &str| {
            let result = if current {
                extractor::extract("nested.css.ts", &source, ExtractOption::default())
                    .map(|o| o.code)
                    .map_err(|e| e.to_string())
            } else {
                code_extract_counter(input("nested.css.ts", &source), false).map(|o| o.code())
            };
            match result {
                Ok(code) => outputs.borrow_mut().push(code),
                Err(error) => errors.borrow_mut().push(error),
            }
            Some(ResolvedModule {
                path: "values.ts".into(),
                code: "export const tone='red';".into(),
            })
        };
        let mut outer = input(
            "outer.css.ts",
            concat!(
                "import {tone} from './values';import {style} from '@devup-ui/react';export const root=style(",
                "{",
                "color:tone",
                "}",
                ");"
            ),
        );
        outer.resolver = Some(&resolver);
        // When
        let result = code_extract_counter(outer, false);
        // Then: deliberately not converted to expected failure.
        println!(
            "api={api} current={current} direct={direct} nested_errors={:?} outer={:?}",
            observed_errors.borrow(),
            result.as_ref().map(Output::code)
        );
        assert_eq!(*observed_errors.borrow(), Vec::<String>::new());
        assert!(observed_outputs.borrow().iter().all(|code| code == &direct));
        assert!(result.is_ok());
    });
}
