use super::*;

#[rstest]
#[serial]
fn output_preserves_owners_and_flags_when_real_root_changes(
    #[values(
        Action::Insert,
        Action::Noop,
        Action::Hmr,
        Action::Cleanup,
        Action::Keyframes
    )]
    action: Action,
    #[values(0, 1, 2)] route: u8,
    #[values(false, true)] import_main: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        let setup = || {
            import_canonical_map_internal(HashMap::from([
                (
                    "root.tsx".into(),
                    if route == 2 { "@global" } else { "shared.tsx" }.into(),
                ),
                ("peer.tsx".into(), "shared.tsx".into()),
            ]));
        };
        setup();
        let red = "import {Box} from '@devup-ui/react';const a=<Box color='red'/>;";
        let blue = "import {Box} from '@devup-ui/react';const a=<Box color='blue'/>;";
        let purple = "import {Box} from '@devup-ui/react';const a=<Box color='purple'/>;";
        let green = "import {Box} from '@devup-ui/react';const a=<Box color='green'/>;";
        let old_frames = "import {keyframes} from '@devup-ui/react';const a=keyframes({from:{opacity:0},to:{opacity:1}});";
        let new_frames = "import {keyframes} from '@devup-ui/react';const a=keyframes({from:{opacity:1},to:{opacity:0}});";
        let old = match action {
            Action::Keyframes => old_frames,
            Action::Cleanup => {
                "import {globalCss} from '@devup-ui/react';globalCss({'.root':{color:'red'}});"
            }
            Action::Insert | Action::Noop | Action::Hmr => red,
        };
        let new = match action {
            Action::Insert | Action::Hmr => green,
            Action::Noop => red,
            Action::Cleanup => "export const a=1;",
            Action::Keyframes => new_frames,
        };
        let make = |file, source| {
            let mut request = input(file, source);
            request.option.single_css = route == 1;
            request
        };
        if !matches!(action, Action::Insert) {
            required(code_extract_counter(make("root.tsx", old), import_main));
        }
        required(code_extract_counter(make("peer.tsx", blue), import_main));
        required(code_extract_counter(
            make("outside.tsx", purple),
            import_main,
        ));
        let before = authority();
        // When
        let output = required(code_extract_counter(make("root.tsx", new), import_main));
        // Then
        let global = with_style_sheet(|s| s.create_css(None, false));
        let peer = with_style_sheet(|s| s.create_css(Some("shared.tsx"), false));
        let outside = with_style_sheet(|s| s.create_css(Some("outside.tsx"), false));
        let all = if route == 1 {
            global
        } else {
            format!("{global}{peer}{outside}")
        };
        assert_eq!(all.matches("color:blue").count(), 1);
        assert_eq!(all.matches("color:purple").count(), 1);
        if route == 0 || route == 2 {
            assert_eq!(peer.matches("color:blue").count(), 1);
            assert_eq!(outside.matches("color:purple").count(), 1);
            assert!(!peer.contains("color:purple") && !outside.contains("color:blue"));
        }
        assert!(!all.contains(":is(") && !all.contains("&&"));
        match action {
            Action::Noop => {
                assert_eq!(output.css(), None);
                assert!(!output.updated_base_style());
                assert_eq!(authority(), before);
            }
            Action::Insert | Action::Hmr | Action::Cleanup | Action::Keyframes => {
                assert!(output.css().is_some());
                assert_eq!(
                    output.updated_base_style(),
                    matches!(action, Action::Cleanup)
                );
                let expected = with_style_sheet(|s| {
                    s.create_css(
                        if route == 0 { Some("shared.tsx") } else { None },
                        import_main,
                    )
                });
                assert_eq!(output.css(), Some(expected));
            }
        }
        match action {
            Action::Insert => {
                assert_eq!(all.matches("color:green").count(), 1);
                assert!(!all.contains("color:red"));
            }
            Action::Hmr => {
                assert_eq!(all.matches("color:green").count(), 1);
                assert_eq!(all.matches("color:red").count(), 1);
            }
            Action::Cleanup => assert!(!all.contains("color:red")),
            Action::Keyframes => {
                assert_eq!(all.matches("@keyframes").count(), 2);
                assert_eq!(
                    all.matches(concat!(
                        "from",
                        "{",
                        "opacity:1",
                        "}",
                        "to",
                        "{",
                        "opacity:0",
                        "}"
                    ))
                    .count(),
                    1
                );
                assert_eq!(
                    all.matches(concat!(
                        "from",
                        "{",
                        "opacity:0",
                        "}",
                        "to",
                        "{",
                        "opacity:1",
                        "}"
                    ))
                    .count(),
                    1
                );
            }
            Action::Noop => assert_eq!(all.matches("color:red").count(), 1),
        }
        let actual = ObservedOutput::capture(&output, "root.tsx");
        let actual_maps = cache6_session::Maps::capture();
        fresh();
        setup();
        if !matches!(action, Action::Insert) {
            required(code_extract_counter(make("root.tsx", old), import_main));
        }
        required(code_extract_counter(make("peer.tsx", blue), import_main));
        required(code_extract_counter(
            make("outside.tsx", purple),
            import_main,
        ));
        let clean = required(code_extract_counter(make("root.tsx", new), import_main));
        assert_eq!(ObservedOutput::capture(&clean, "root.tsx"), actual);
        assert_eq!(cache6_session::Maps::capture(), actual_maps);
    });
}

#[rstest]
#[serial]
fn raw_cleanup_preserves_css_once_when_one_raw_root_shares_canonical_bucket(
    #[values(false, true)] single: bool,
    #[values(false, true)] import_main: bool,
) {
    with_admission(|| {
        // Given
        let _guard = Guard::new();
        let setup = || {
            import_canonical_map_internal(HashMap::from([
                ("root.css.ts".into(), "shared.css.ts".into()),
                ("peer.css.ts".into(), "shared.css.ts".into()),
            ]));
        };
        setup();
        let root =
            "import {globalStyle} from '@devup-ui/react';globalStyle('.root',{color:'red'});";
        let peer =
            "import {globalStyle} from '@devup-ui/react';globalStyle('.peer',{color:'blue'});";
        let outside =
            "import {globalStyle} from '@devup-ui/react';globalStyle('.outside',{color:'purple'});";
        let make = |file, code| {
            let mut request = input(file, code);
            request.option.single_css = single;
            request
        };
        required(code_extract_counter(make("root.css.ts", root), import_main));
        required(code_extract_counter(make("peer.css.ts", peer), import_main));
        required(code_extract_counter(
            make("outside.css.ts", outside),
            import_main,
        ));
        // When
        let output = required(code_extract_counter(
            make("root.css.ts", "export const a=1;"),
            import_main,
        ));
        // Then
        assert!(output.updated_base_style());
        assert!(output.css().is_some());
        let global = with_style_sheet(|s| s.create_css(None, false));
        assert!(!global.contains(".root"));
        assert_eq!(
            global
                .matches(concat!(".peer", "{", "color:blue", "}"))
                .count(),
            1
        );
        assert_eq!(
            global
                .matches(concat!(".outside", "{", "color:purple", "}"))
                .count(),
            1
        );
        let observed = ObservedOutput::capture(&output, "root.css.ts");
        let actual_maps = cache6_session::Maps::capture();
        fresh();
        setup();
        required(code_extract_counter(make("root.css.ts", root), import_main));
        required(code_extract_counter(make("peer.css.ts", peer), import_main));
        required(code_extract_counter(
            make("outside.css.ts", outside),
            import_main,
        ));
        let clean = required(code_extract_counter(
            make("root.css.ts", "export const a=1;"),
            import_main,
        ));
        assert_eq!(ObservedOutput::capture(&clean, "root.css.ts"), observed);
        assert_eq!(cache6_session::Maps::capture(), actual_maps);
    });
}
