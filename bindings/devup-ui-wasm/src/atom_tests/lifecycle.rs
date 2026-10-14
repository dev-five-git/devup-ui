use super::*;

const RED: &str = "import {Box} from '@devup-ui/react'; export const x=<Box color=\"red\"/>;";

#[test]
#[serial]
fn unknown_and_late_reach_stay_local_until_reset() {
    // Given
    setup();
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
    // Given
    setup();
    let shared = compile("a.tsx", RED);
    let private = compile("private.tsx", RED);
    let global = with_style_sheet(|sheet| sheet.create_css(None, false));
    let local = with_style_sheet(|sheet| sheet.create_css(Some("private.tsx"), false));
    let serialized = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    reset_build_state_internal();
    let imported = serde_json::from_str(&serialized).unwrap_or_else(|error| panic!("{error}"));
    // When
    import_sheet_internal(imported).unwrap_or_else(|error| panic!("{error}"));
    seed_file_map(vec!["a.tsx".into(), "b.tsx".into(), "private.tsx".into()]);
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
    css::atom_hoist::set_atom_hoist(Some(2));
    css::file_routes::set_file_routes(HashMap::from([(
        "private.tsx".into(),
        HashSet::from([0, 1]),
    )]));
    let resumed = compile("private.tsx", RED);
    assert_eq!(private.code, resumed.code);
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
        (specifier == "./tokens").then(|| ResolvedModule {
            path: "tokens.ts".to_string(),
            code: "export const styles={color:'red',_hover:{color:'blue'}};".to_string(),
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
        let properties = with_style_sheet(|sheet| {
            sheet.properties["private.tsx"]
                .values()
                .flat_map(BTreeMap::values)
                .flatten()
                .cloned()
                .collect::<Vec<_>>()
        });
        let rules: Vec<_> = properties
            .iter()
            .filter(|prop| prop.property == "color")
            .collect();
        let resets: Vec<_> = properties.iter().filter(|prop| prop.owner_reset).collect();
        assert_eq!(rules.len(), 3);
        assert_eq!(resets.len(), 3);
        assert_eq!(
            rules
                .iter()
                .map(|prop| &prop.class_name)
                .collect::<HashSet<_>>()
                .len(),
            3
        );
        assert_eq!(
            rules
                .iter()
                .map(|prop| &prop.value)
                .collect::<HashSet<_>>()
                .len(),
            3
        );
        let assignments = css::utils::compile_regex(
            r#"(?s)["'](---(?:du-)?S[a-z0-9_U-]+)["']\s*:\s*.*?\]\)\((tone\b|differentCode\b|`\$\{tone\}`)\)"#,
        );
        let sites: Vec<_> = assignments.captures_iter(&output.code).collect();
        assert_eq!(sites.len(), 3, "{}", output.code);
        assert_eq!(
            sites
                .iter()
                .map(|site| &site[1])
                .collect::<HashSet<_>>()
                .len(),
            3
        );
        assert_eq!(&sites[0][2], "tone");
        assert_eq!(&sites[1][2], "differentCode");
        assert!(matches!(&sites[2][2], "tone" | "`${tone}`"));
        assert_references_resolve(&output.code, &css);
        for (index, site) in sites.iter().enumerate() {
            let variable = &site[1];
            let value = format!(
                "var({variable}){}",
                if index == 2 { " !important" } else { "" }
            );
            let rule = rules
                .iter()
                .find(|prop| prop.value == value)
                .unwrap_or_else(|| panic!("missing site color rule for {value}\n{css}"));
            assert!(!rule.owner_reset);
            assert!(output.code.contains(&rule.class_name));
            let owned: Vec<_> = resets
                .iter()
                .filter(|prop| prop.class_name == rule.class_name)
                .collect();
            assert_eq!(owned.len(), 1);
            assert_eq!(owned[0].property, variable);
            assert_eq!(owned[0].value, "initial");
            assert_eq!(owned[0].selector, None);
            assert_eq!(owned[0].layer, None);
            let selector = format!(".{}{{", rule.class_name);
            assert_eq!(css.matches(&selector).count(), 1, "{css}");
            let block = css
                .split_once(&selector)
                .unwrap_or_else(|| panic!("missing own class block {selector}\n{css}"))
                .1;
            let block = block
                .split_once('}')
                .unwrap_or_else(|| panic!("unclosed class block {selector}\n{css}"))
                .0;
            let declarations: HashSet<_> =
                block.split(';').filter(|value| !value.is_empty()).collect();
            assert_eq!(
                declarations,
                HashSet::from([
                    format!("{variable}:initial").as_str(),
                    format!("color:{value}").as_str(),
                ])
            );
        }
    }
    reset_build_state_internal();
}

#[test]
#[serial]
fn keyframe_content_and_scope_resolve_when_using_single_css() {
    // Given
    setup();
    let source = r"import {Box,keyframes} from '@devup-ui/react';
      const a=keyframes({from:{opacity:0,transform:'scale(0)'},to:{opacity:1}});
      const b=keyframes({from:{transform:'scale(0)',opacity:0},to:{opacity:1}});
      const c=keyframes({from:{opacity:0,transform:'scale(0)'},to:{opacity:0.5}});
      export const x=<><Box animation={`${a} 1s`}/><Box animation={`${b} 1s`}/><Box animation={`${c} 1s`}/></>;";
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
    reset_build_state_internal();
}

#[test]
#[serial]
fn global_css_keyframes_use_the_same_scope_as_their_declaration() {
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
