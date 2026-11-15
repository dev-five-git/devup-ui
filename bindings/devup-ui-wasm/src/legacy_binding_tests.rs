    #[test]
    #[serial]
    fn test_canonical_map_import_export_roundtrip() {
        use css::file_map::{get_canonical_map, reset_canonical_map};
        let _state = TestStateGuard::new();
        reset_canonical_map();
        let mut m = HashMap::new();
        m.insert("src/child.tsx".to_string(), "src/parent.tsx".to_string());
        import_canonical_map_internal(m.clone());
        assert_eq!(get_canonical_map(), m);
        let json = export_canonical_map_internal().expect("export canonical map");
        assert!(json.contains("src/child.tsx"));
        assert!(json.contains("src/parent.tsx"));
        // canonical() resolves via the imported map; unmapped is identity.
        assert_eq!(canonical("src/child.tsx"), "src/parent.tsx");
        assert_eq!(canonical("src/other.tsx"), "src/other.tsx");
        reset_canonical_map();
    }

    #[test]
    #[serial]
    fn test_code_extract() {
        let _state = TestStateGuard::new();
        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            *sheet = StyleSheet::default();
        }
        assert_eq!(
            get_css(None, false).unwrap().split("*/").nth(1).unwrap(),
            ""
        );

        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            let mut theme = Theme::default();
            let mut color_theme = ColorTheme::default();
            color_theme.add_color("primary", "#000");
            theme.add_color_theme("dark", color_theme);

            let mut color_theme = ColorTheme::default();
            color_theme.add_color("primary", "#FFF");
            theme.add_color_theme("default", color_theme);
            sheet.set_theme(theme);
        }

        assert_debug_snapshot!(get_css(None, false).unwrap().split("*/").nth(1).unwrap());
    }

    #[test]
    #[serial]
    fn test_code_extract_with_modules() {
        let _state = TestStateGuard::new();
        {
            let mut sheet = GLOBAL_STYLE_SHEET.lock().unwrap();
            *sheet = StyleSheet::default();
        }
        let resolver = |specifier: &str, _: &str| {
            (specifier == "./tokens").then(|| {
                ModuleResolution::Resolved(ResolvedModule {
                    source_type: None,
                    path: "/src/tokens.ts".to_string(),
                    code: "export const PRIMARY = 'red'".to_string(),
                })
            })
        };
        {
            let output = code_extract_with_modules_internal(
                "/src/App.tsx",
                "import { Box } from '@devup-ui/react';\nimport { PRIMARY } from './tokens';\nexport const a = <Box color={PRIMARY} />;",
                "@devup-ui/react",
                "@devup-ui/react".to_string(),
                true,
                false,
                false,
                HashMap::new(),
                None,
                &resolver,
            )
            .unwrap();
            assert_eq!(output.dependencies(), ["/src/tokens.ts"]);
            assert!(!output.code().contains("--"), "{}", output.code());
        }
    }
