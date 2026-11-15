    #[test]
    #[serial]
    fn to_css_from_theme() {
        let _state = TestStateGuard::new();
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");

        assert_eq!(color_theme.css_entries().count(), 1);

        theme.add_color_theme("default", color_theme);
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#fff");
        theme.add_color_theme("dark", color_theme);
        theme.add_typography(
            "default",
            vec![
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("16px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("24px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
            ],
        );

        theme.add_typography(
            "default1",
            vec![
                None,
                Some(Typography::new(
                    Some("Arial".to_string()),
                    Some("24px".to_string()),
                    Some("400".to_string()),
                    Some("1.5".to_string()),
                    Some("0.5".to_string()),
                )),
            ],
        );
        let css = theme.to_css();
        assert_debug_snapshot!(css);

        assert_eq!(Theme::default().to_css(), "");
        let mut theme = Theme::default();
        theme.add_typography(
            "default",
            vec![Some(Typography::new(None, None, None, None, None))],
        );
        assert_eq!(theme.to_css(), "");

        let mut theme = Theme::default();
        theme.add_color_theme("default", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("dark", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("dark", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("a", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("a", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("c", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());

        let mut theme = Theme::default();
        theme.add_color_theme("light", make_named_color_theme("primary", "#000"));
        theme.add_color_theme("b", make_named_color_theme("primary", "#001"));
        theme.add_color_theme("a", make_named_color_theme("primary", "#002"));
        theme.add_color_theme("c", make_named_color_theme("primary", "#000"));
        assert_debug_snapshot!(theme.to_css());
    }

    #[rstest]
    #[case(
        vec![0, 480, 768, 992, 1280],
        vec![0, 480, 768, 992, 1280, 1600]
    )]
    #[case(
        vec![0, 480, 768, 992, 1280, 1600],
        vec![0, 480, 768, 992, 1280, 1600]
    )]
    #[case(
        vec![0, 480, 768, 992, 1280, 1600, 1920],
        vec![0, 480, 768, 992, 1280, 1600, 1920]
    )]
    fn update_breakpoints(#[case] input: Vec<u16>, #[case] expected: Vec<u16>) {
        let mut theme = Theme::default();
        theme.update_breakpoints(input);
        assert_eq!(theme.breakpoints, expected);
    }
