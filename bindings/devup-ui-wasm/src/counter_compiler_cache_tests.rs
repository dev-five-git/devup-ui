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

#[rstest]
#[serial]
fn genuine_raw_cache_is_silent_cold_when_version_shape_or_record_is_damaged(
    #[values(0, 1, 2, 3, 4, 5, 6, 7)] kind: u8,
) {
    with_admission(|| {
        // Given: expected output comes from undamaged real JS, never candidate data.
        let _guard = Guard::new();
        let source = "import {css} from '@devup-ui/react';const a=css({color:'red'});";
        let clean = compile("a.css.ts", source);
        let clean_maps = cache6_session::Maps::capture();
        let bytes = encoded();
        let mut value: serde_json::Value = required(serde_json::from_slice(&bytes));
        let bytes = match kind {
            0 => {
                fresh();
                required(exact_test_support::compile("current.css.ts", source));
                required(export_sheet_internal()).into_bytes()
            }
            1 | 2 => {
                value["atomNamingVersion"] = serde_json::json!(if kind == 1 { 5 } else { 7 });
                required(serde_json::to_vec(&value))
            }
            3 => b"{truncated".to_vec(),
            4 => required(String::from_utf8(bytes))
                .replacen(
                    "\"atomNamingVersion\":6",
                    "\"atomNamingVersion\":6,\"atomNamingVersion\":6",
                    1,
                )
                .into_bytes(),
            5 | 6 => {
                if kind != 6 {
                    value["properties"] = serde_json::json!({});
                }
                if kind != 5 {
                    value["evidence"]["counters"] = serde_json::json!({});
                }
                required(serde_json::to_vec(&value))
            }
            7 => {
                value["properties"]["a.css.ts"]["255"]["0"][0]["v"] = serde_json::json!("damaged");
                value["evidence"]["counters"]["D9-0"]["0"][0]["proof"]["emission"]["expansion"]["Static"]
                    [0]["Property"]["record"]["v"] = serde_json::json!("damaged");
                required(serde_json::to_vec(&value))
            }
            _ => panic!("kind"),
        };
        fresh();
        let cold = authority();
        // When
        assert_eq!(cache6_restore::import(&bytes), Ok(()));
        // Then
        assert_eq!(authority(), cold);
        cache6_restore::RESTORE.with_borrow(|s| assert_eq!(s, &cache6_session::Restore::default()));
        assert_eq!(cache_names::check(), Ok(()));
        assert_eq!(compile("a.css.ts", source), clean);
        assert_eq!(cache6_session::Maps::capture(), clean_maps);
    });
}

#[rstest]
#[case(
    "import {styled} from '@devup-ui/react'; const A=styled.div({width:state.size});",
    "let reads=0; const state={get size(){reads++;return '13px'}};",
    "const created=reads; const first=A({}); const second=A({}); return [created,reads,first.className===second.className];",
    "[1,1,true]"
)]
#[case(
    "import {Box} from '@devup-ui/react'; let reads=0; const value=()=>{reads++;return 'red'}; const a=<Box color={value()}/>;",
    "",
    "return [reads,Object.values(a.style)];",
    "[1,[\"red\"]]"
)]
#[serial]
fn generated_js_reads_once_when_executed_by_bun(
    #[case] source: &str,
    #[case] setup: &str,
    #[case] assertion: &str,
    #[case] expected: &str,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        let output = code_extract_counter(input("eval.tsx", source), false)
            .unwrap_or_else(|e| panic!("{e}"));
        let driver = r#"const [code,setup,check]=process.argv.slice(-3);const React={createElement:(type,props)=>({type,...props}),forwardRef:f=>props=>f(props,null)};const source=code.replace(/import\s*\{\s*forwardRef\s+as\s+(\w+)\s*\}\s*from\s*['\"]react['\"];?/g,'const $1=React.forwardRef;').replace(/import[\s\S]*?;/g,'');const t=new Bun.Transpiler({loader:'tsx',tsconfig:JSON.stringify({compilerOptions:{jsx:'react',jsxFactory:'React.createElement'}})});console.log(JSON.stringify(new Function('React',setup+t.transformSync(source)+check)(React)));"#;
        // When
        let evaluated = std::process::Command::new("bun")
            .args(["--eval", driver, &output.code(), setup, assertion])
            .output()
            .unwrap_or_else(|e| panic!("Bun launch: {e}"));
        // Then
        assert!(
            evaluated.status.success(),
            "{}",
            String::from_utf8_lossy(&evaluated.stderr)
        );
        assert_eq!(
            String::from_utf8(evaluated.stdout)
                .unwrap_or_else(|e| panic!("Bun UTF8: {e}"))
                .trim(),
            expected
        );
    });
}
