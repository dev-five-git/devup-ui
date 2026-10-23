use super::*;

const RED: &str = "import {Box} from '@devup-ui/react'; export const x=<Box color=\"red\"/>;";

#[test]
#[serial]
fn unknown_and_late_reach_stay_local_until_reset() {
    let _scope = TestStateGuard::new();
    // Given
    setup();
    reserve(&[("a.tsx", RED), ("private.tsx", RED)]);
    compile("a.tsx", RED);
    let mut routes = css::file_routes::get_file_routes();
    routes.insert("unknown.tsx".into(), HashSet::from([0, 1]));
    routes.insert("private.tsx".into(), HashSet::from([0, 1]));
    css::file_routes::set_file_routes(routes.clone());
    css::atom_hoist::set_atom_hoist(Some(1));
    // When
    for file in ["unknown.tsx", "private.tsx"] {
        let output = compile(file, RED);
        // Then
        assert_references_resolve(&output.code, &emitted(file));
        assert!(
            with_style_sheet(|sheet| sheet.create_css(Some(file), false)).contains("color:red")
        );
    }
    reset_build_state_internal();
    css::atom_hoist::set_atom_hoist(Some(2));
    css::file_routes::set_file_routes(routes);
    let output = compile("unknown.tsx", RED);
    assert_references_resolve(&output.code, &emitted("unknown.tsx"));
    assert!(
        !with_style_sheet(|sheet| sheet.create_css(Some("unknown.tsx"), false))
            .contains("color:red")
    );
    reset_build_state_internal();
}

#[test]
#[serial]
fn imported_sheet_keeps_recorded_placement_after_configuration_reset() {
    let _scope = TestStateGuard::new();
    // Given
    setup();
    let shared = compile("a.tsx", RED);
    let private = compile("private.tsx", RED);
    let global = with_style_sheet(|sheet| sheet.create_css(None, false));
    let local = with_style_sheet(|sheet| sheet.create_css(Some("private.tsx"), false));
    let serialized = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let classes = export_class_map_internal().unwrap_or_else(|error| panic!("{error}"));
    let files = export_file_map_internal().unwrap_or_else(|error| panic!("{error}"));
    reset_build_state_internal();
    let imported = serde_json::from_str(&serialized).unwrap_or_else(|error| panic!("{error}"));
    // When
    import_sheet_internal(imported);
    // Then
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        global
    );
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("private.tsx"), false)),
        local
    );
    assert_references_resolve(&shared.code, &emitted("a.tsx"));
    assert_references_resolve(&private.code, &emitted("private.tsx"));
    set_class_map(serde_json::from_str(&classes).unwrap_or_else(|error| panic!("{error}")));
    set_file_map(serde_json::from_str(&files).unwrap_or_else(|error| panic!("{error}")));
    css::atom_hoist::set_atom_hoist(Some(2));
    css::file_routes::set_file_routes(HashMap::from([(
        "private.tsx".into(),
        HashSet::from([0, 1]),
    )]));
    let resumed = compile("private.tsx", RED);
    assert_eq!(private.code, resumed.code);
    let changed = compile(
        "private.tsx",
        "import {Box} from '@devup-ui/react'; export const x=<Box color=\"blue\"/>;",
    );
    assert_ne!(private.code, changed.code);
    assert_references_resolve(&changed.code, &emitted("private.tsx"));
    let shared_again = compile("b.tsx", RED);
    assert_references_resolve(&shared_again.code, &emitted("b.tsx"));
    assert!(
        !with_style_sheet(|sheet| sheet.create_css(Some("b.tsx"), false)).contains("color:red")
    );
    reset_build_state_internal();
}

#[test]
#[serial]
fn canonical_bucket_reach_is_frozen_before_imported_styles_are_processed() {
    let _scope = TestStateGuard::new();
    // Given
    setup();
    import_canonical_map_internal(HashMap::from([
        ("member.tsx".into(), "bucket.tsx".into()),
        ("peer.tsx".into(), "bucket.tsx".into()),
    ]));
    css::file_routes::set_file_routes(HashMap::from([
        ("member.tsx".into(), HashSet::from([0])),
        ("peer.tsx".into(), HashSet::from([1])),
    ]));
    let resolver = |specifier: &str, _importer: &str| {
        (specifier == "./tokens").then(|| {
            extractor::ModuleResolution::Resolved(ResolvedModule {
                path: "tokens.ts".to_string(),
                code: "export const styles={color:'red',_hover:{color:'blue'}};".to_string(),
                source_type: None,
            })
        })
    };
    // When
    let output = code_extract_with_modules_internal(
        "member.tsx",
        "import {Box} from '@devup-ui/react'; import {styles} from './tokens'; export const x=<Box {...styles}/>;",
        "@devup-ui/react",
        "df".to_string(),
        false,
        false,
        false,
        HashMap::new(),
        None,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_references_resolve(&output.code, &emitted("bucket.tsx"));
    assert!(with_style_sheet(|sheet| sheet.create_css(None, false)).contains("color:red"));
    assert!(
        !with_style_sheet(|sheet| sheet.create_css(Some("bucket.tsx"), false))
            .contains("color:red")
    );
    reset_build_state_internal();
}

#[test]
#[serial]
fn important_and_plain_dynamic_values_emit_distinct_rules_in_every_mode() {
    let _scope = TestStateGuard::new();
    // Given
    let source = r"import {Box} from '@devup-ui/react';
      export const x=<><Box color={tone}/><Box color={differentCode}/><Box color={`${tone} !important`}/></>;";
    for atom in [false, true] {
        setup();
        if !atom {
            css::atom_hoist::set_atom_hoist(None);
        }
        // When
        let output = compile("private.tsx", source);
        // Then
        let css = emitted("private.tsx");
        let rules = with_style_sheet(|sheet| {
            sheet.properties["private.tsx"]
                .values()
                .flat_map(BTreeMap::values)
                .flatten()
                .map(|prop| (prop.class_name.clone(), prop.value.clone()))
                .collect::<Vec<_>>()
        });
        assert_eq!(rules.len(), 2);
        assert_ne!(rules[0].0, rules[1].0);
        for (name, value) in rules {
            assert!(output.code.contains(&name));
            assert!(css.contains(&format!(".{name}{{color:{value}}}")), "{css}");
        }
        if atom {
            assert_references_resolve(&output.code, &css);
        }
    }
    reset_build_state_internal();
}

#[test]
#[serial]
fn keyframe_content_and_scope_resolve_when_using_single_css() {
    let _scope = TestStateGuard::new();
    // Given
    let source = r"import {Box,keyframes} from '@devup-ui/react';
      const a=keyframes({from:{opacity:0,transform:'scale(0)'},to:{opacity:1}});
      const b=keyframes({from:{transform:'scale(0)',opacity:0},to:{opacity:1}});
      const c=keyframes({from:{opacity:0,transform:'scale(0)'},to:{opacity:0.5}});
      export const x=<><Box animation={`${a} 1s`}/><Box animation={`${b} 1s`}/><Box animation={`${c} 1s`}/></>;";
    let build = |atom: bool, debug| {
        setup();
        css::atom_hoist::set_atom_hoist(atom.then_some(2));
        css::debug::set_debug(debug);
        reserve_class_slots_internal(
            &[ReservationInput {
                filename: "a.tsx".into(),
                code: source.into(),
                css_dir: "df".into(),
                source_type: None,
            }],
            ExtractOption {
                single_css: true,
                ..ExtractOption::default()
            },
            None,
        );
        // When
        let output = code_extract_internal(
            "a.tsx",
            source,
            "@devup-ui/react",
            "df".into(),
            true,
            false,
            false,
            HashMap::new(),
            None,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        // Then
        let css = with_style_sheet(|sheet| sheet.create_css(None, false));
        assert_references_resolve(&output.code, &css);
        with_style_sheet(|sheet| {
            assert_eq!(sheet.keyframes[""].len(), 2);
            for name in sheet.keyframes[""].keys() {
                assert!(output.code.contains(name));
                assert!(css.contains(&format!("@keyframes {name}{{")));
            }
        });
        assert_eq!(get_unreserved_class_slot_count(), 0);
        (output.code, css)
    };
    for debug in [false, true] {
        assert_eq!(build(false, debug), build(true, debug));
    }
    reset_build_state_internal();
    css::debug::set_debug(false);
}

#[test]
#[serial]
fn global_css_keyframes_use_the_same_scope_as_their_declaration() {
    let _scope = TestStateGuard::new();
    // Given
    setup();
    let source = r"import {Box,keyframes,globalCss} from '@devup-ui/react';
      const fade=keyframes({from:{opacity:0},to:{opacity:1}});
      globalCss({body:{animation:`${fade} 1s`}}); export const x=<Box animation={`${fade} 2s`}/>;";
    // When
    let output = compile("a.tsx", source);
    // Then
    assert_references_resolve(&output.code, &emitted("a.tsx"));
    let global = with_style_sheet(|sheet| sheet.create_css(None, false));
    let local = with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false));
    with_style_sheet(|sheet| {
        for name in sheet.keyframes["a.tsx"].keys() {
            assert!(global.contains(&format!("animation:{name} 1s")), "{global}");
            assert!(local.contains(&format!("@keyframes {name}{{")));
            assert!(!global.contains(&format!("@keyframes {name}{{")));
        }
    });
    reset_build_state_internal();
}
