use super::*;

#[test]
#[serial]
fn imported_outer_render_condition_keeps_css_and_original_counter_classes() {
    // Given: D9-1 owns both literal atoms despite canonical delivery through D9-0.
    let source = "import {Box,css} from '@devup-ui/react';import {enabled} from './tokens';export const witness=css({color:enabled?'red':'red'});export const x=<Box p={4}/>;export const y=enabled&&<Box h='12px'/>;";
    let mut witnesses = Vec::new();
    for enabled in [Some(true), Some(false), None] {
        setup();
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let provider_calls = std::rc::Rc::clone(&calls);
        let resolver = move |specifier: &str, importer: &str| {
            assert_eq!(specifier, "./tokens");
            assert_eq!(importer, "child.tsx");
            provider_calls.set(provider_calls.get() + 1);
            enabled.map(|value| ResolvedModule {
                path: "tokens.ts".into(),
                code: format!("export const enabled={value};"),
            })
        };
        // When: the same source is extracted with each distinct provider outcome.
        let output = code_extract_with_modules_internal(
            "child.tsx",
            source,
            "@devup-ui/react",
            "df".into(),
            false,
            false,
            false,
            HashMap::new(),
            &resolver,
        )
        .unwrap_or_else(|error| panic!("enabled={enabled:?}: {error}"));
        // Then: style resolution changes, but the runtime render guard and atoms do not.
        assert!(calls.get() > 0, "provider was not called: {enabled:?}");
        let code = output.code();
        let controller = code
            .split("export const y = ")
            .nth(1)
            .and_then(|expression| expression.split_once("&&"))
            .map_or_else(
                || panic!("missing render condition: {code}"),
                |(condition, _)| condition.trim(),
            );
        assert_eq!(controller, "enabled", "{code}");
        assert!(
            code.lines().any(|line| line.starts_with("import ")
                && line.contains("enabled")
                && line.contains("./tokens")),
            "runtime import was removed: {code}"
        );
        let witness = code
            .split("export const witness = ")
            .nth(1)
            .and_then(|expression| expression.split("export const x = ").next())
            .unwrap_or_else(|| panic!("missing style-resolution witness: {code}"));
        assert_eq!(witness.contains("enabled"), enabled.is_none(), "{code}");
        let classes: Vec<_> = code
            .split("className=")
            .skip(1)
            .map(|field| {
                field
                    .strip_prefix('"')
                    .and_then(|literal| literal.split_once('"'))
                    .map_or_else(
                        || panic!("nonliteral className: {code}"),
                        |(name, _)| name.to_owned(),
                    )
            })
            .collect();
        assert_eq!(classes, ["b-a", "b-b"], "{code}");
        let css = delivered();
        assert!(css.contains(".b-a{padding:16px}"), "{css}");
        assert!(css.contains(".b-b{height:12px}"), "{css}");
        assert!(
            css.contains(concat!(".a-RLcolor-vred{", "color:red", "}")),
            "{css}"
        );
        assert_eq!(output.css(), Some(css.clone()));
        assert_eq!(output.css_file(), Some("df/devup-ui-0.css".into()));
        css::class_map::with_class_map(|map| {
            assert_eq!(map.len(), 1);
            let slots = map
                .get("D9-1")
                .unwrap_or_else(|| panic!("missing original owner: {map:?}"));
            assert_eq!(slots.len(), 2);
            assert_eq!(
                slots
                    .values()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>(),
                [0, 1].into_iter().collect()
            );
        });
        with_style_sheet(|sheet| {
            assert_eq!(sheet.names.len(), 1);
            assert!(sheet.names.contains_key("a-RLcolor-vred"));
        });
        println!(
            "enabled={enabled:?}; classes={classes:?}; full_css={css:?}; controller={controller}"
        );
        witnesses.push((css, classes));
        reset_build_state_internal();
    }
    for pair in witnesses.windows(2) {
        assert_eq!(pair[0], pair[1]);
    }
}

#[rstest]
#[case(
    "export const tone='red';export const enabled=true;export const styles={color:'green',m:2};"
)]
#[case(
    "export const tone='blue';export const enabled=false;export const styles={bg:'yellow',w:'8px'};"
)]
#[serial]
fn risky_import_values_presence_and_shape_leave_local_padding_slot_unchanged(
    #[case] module: &'static str,
) {
    // Given
    setup();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./tokens").then(|| ResolvedModule {
            path: "tokens.ts".into(),
            code: module.into(),
        })
    };
    let source = "import {Box} from '@devup-ui/react';import {tone,enabled,styles} from './tokens';export const x=<Box color={tone} {...styles} p={4}/>;export const y=enabled&&<Box h='12px'/>;";
    // When
    let output = code_extract_with_modules_internal(
        "child.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert!(output.code().contains("b-a"), "{}", output.code());
    assert!(
        delivered().contains(".b-a{padding:16px}"),
        "{}",
        delivered()
    );
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 1);
        assert_eq!(map["D9-1"].len(), 2);
    });
    with_style_sheet(|sheet| {
        assert!(
            sheet
                .names
                .keys()
                .any(|name| name.contains("-RL") || name.contains("-RH"))
        );
        assert!(!sheet.names.contains_key("b-a"));
    });
    reset_build_state_internal();
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn nested_stylesheet_scope_restores_consumer_owner_without_copying_producer_rules(
    #[case] producer_first: bool,
) {
    // Given
    reset_build_state_internal();
    seed_file_map(vec!["a.css.ts".into(), "child.css.ts".into()]);
    set_canonical_map(HashMap::from([("child.css.ts".into(), "a.css.ts".into())]));
    let producer = "import {style} from '@devup-ui/react';export const red=style({color:'red'});";
    let consumer = "import {style} from '@devup-ui/react';import {red} from './a.css';export const selected=style({selectors:{[`.${red} &`]:{padding:'4px'}}});export const local=style({margin:'8px'});";
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./a.css").then(|| ResolvedModule {
            path: "a.css.ts".into(),
            code: producer.into(),
        })
    };
    let run_producer = || {
        code_extract_with_modules_internal(
            "a.css.ts",
            producer,
            "@devup-ui/react",
            "df".into(),
            false,
            false,
            false,
            HashMap::new(),
            &resolver,
        )
        .unwrap_or_else(|error| panic!("{error}"))
    };
    if producer_first {
        run_producer();
    }
    // When
    let output = code_extract_with_modules_internal(
        "child.css.ts",
        consumer,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let before_producer = with_style_sheet(|sheet| sheet.create_css(Some("a.css.ts"), false));
    if !producer_first {
        run_producer();
    }
    // Then
    let code = output.code();
    let selected = code
        .lines()
        .find_map(|line| {
            line.strip_prefix("export const selected = \"")
                .and_then(|line| line.strip_suffix("\";"))
        })
        .unwrap_or_else(|| panic!("missing selected export: {code}"));
    assert!(selected.starts_with("a-RH"), "{code}");
    assert!(code.contains("a-RLmargin-v8px"), "{code}");
    if !producer_first {
        assert!(!before_producer.contains("color:red"), "{before_producer}");
    }
    let css = with_style_sheet(|sheet| sheet.create_css(Some("a.css.ts"), false));
    assert!(css.contains(concat!(".a-a{", "color:red", "}")), "{css}");
    assert!(
        css.contains(&format!(".a-a .{selected}{{padding:4px}}")),
        "{css}"
    );
    assert!(
        css.contains(concat!(".a-RLmargin-v8px{", "margin:8px", "}")),
        "{css}"
    );
    assert_eq!(css.matches("color:red").count(), 1);
    css::class_map::with_class_map(|map| {
        assert_eq!(map.len(), 1);
        assert_eq!(map["D9-0"].len(), 1);
    });
    with_style_sheet(|sheet| {
        assert_eq!(sheet.names.len(), 2);
        assert!(sheet.names.contains_key(selected));
        assert!(sheet.names.contains_key("a-RLmargin-v8px"));
    });
    reset_build_state_internal();
}
