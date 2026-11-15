    #[test]
    #[serial]
    fn test_get_theme_interface() {
        let _state = TestStateGuard::new();
        register_shorthands_internal(BTreeMap::new());
        let sheet = StyleSheet::default();
        assert_eq!(
            sheet.create_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "LengthInterface",
                "ShadowsInterface",
                "ThemeInterface"
            ),
            ""
        );

        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("primary", "#000");
        theme.add_color_theme("dark", color_theme);
        GLOBAL_STYLE_SHEET.lock().unwrap().set_theme(theme);
        register_shorthands_internal(BTreeMap::from([(
            "insetX".to_string(),
            vec!["left".to_string(), "right".to_string()],
        )]));
        assert_eq!(
            get_theme_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "LengthInterface",
                "ShadowsInterface",
                "ThemeInterface"
            ),
            "import \"package\";import type{DevupProps}from\"package\";declare module \"package\"{interface ColorInterface{$primary:null}interface TypographyInterface{}interface LengthInterface{}interface ShadowsInterface{}interface DevupCustomShorthands{insetX?:DevupProps[\"w\"]}interface ThemeInterface{dark:null}}"
        );

        // test wrong case
        register_shorthands_internal(BTreeMap::new());
        let mut sheet = StyleSheet::default();
        let mut theme = Theme::default();
        let mut color_theme = ColorTheme::default();
        color_theme.add_color("(primary)", "#000");
        theme.add_color_theme("dark", color_theme);
        theme.add_typography(
            "prim``ary",
            vec![Some(Typography::new(
                Some("Arial".to_string()),
                Some("16px".to_string()),
                Some("400".to_string()),
                Some("1.5".to_string()),
                Some("0.5".to_string()),
            ))],
        );
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(
            get_theme_interface(
                "package",
                "ColorInterface",
                "TypographyInterface",
                "LengthInterface",
                "ShadowsInterface",
                "ThemeInterface"
            ),
            "import \"package\";declare module \"package\"{interface ColorInterface{[`$(primary)`]:null}interface TypographyInterface{[`prim\\`\\`ary`]:null}interface LengthInterface{}interface ShadowsInterface{}interface ThemeInterface{dark:null}}"
        );
    }

    #[test]
    #[serial]
    fn test_debug() {
        let _state = TestStateGuard::new();
        assert!(!is_debug());
        set_debug(true);
        assert!(is_debug());
        set_debug(false);
        assert!(!is_debug());
    }

    #[test]
    #[serial]
    fn test_prefix() {
        let _state = TestStateGuard::new();
        assert_eq!(get_prefix(), None);
        set_prefix(Some("du-".to_string()));
        assert_eq!(get_prefix(), Some("du-".to_string()));
        set_prefix(None);
        assert_eq!(get_prefix(), None);
    }

    #[test]
    #[serial]
    fn test_default_theme() {
        let _state = TestStateGuard::new();
        let mut theme = Theme::default();
        theme.add_color_theme("light", ColorTheme::default());
        theme.add_color_theme("dark", ColorTheme::default());
        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("light".to_string()));

        let mut theme = Theme::default();
        theme.add_color_theme("default", ColorTheme::default());
        theme.add_color_theme("dark", ColorTheme::default());

        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("default".to_string()));

        let mut theme = Theme::default();
        theme.add_color_theme("dark", ColorTheme::default());

        let mut sheet = StyleSheet::default();
        sheet.set_theme(theme);
        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        assert_eq!(get_default_theme().unwrap(), Some("dark".to_string()));
    }
