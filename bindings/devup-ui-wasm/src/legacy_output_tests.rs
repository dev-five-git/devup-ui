    #[test]
    #[serial]
    fn test_output_new_and_getters() {
        let _state = TestStateGuard::new();
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Use extract to get real styles
        let result = extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
<Box bg="red" />"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: false,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        )
        .unwrap();

        let output = Output::new(
            result.code.clone(),
            result.styles,
            Some("//# sourceMappingURL=test".to_string()),
            false,
            "test.tsx".to_string(),
            Some("devup-ui-0.css".to_string()),
            false,
            Vec::new(),
        );

        // Test getters
        assert_ne!(output.code(), "");
        assert_eq!(output.css_file(), Some("devup-ui-0.css".to_string()));
        assert_eq!(output.map(), Some("//# sourceMappingURL=test".to_string()));
        assert!(output.css().is_some());
    }

    #[test]
    #[serial]
    fn test_output_updated_base_style() {
        let _state = TestStateGuard::new();
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Create output with empty styles
        let styles = FxHashSet::default();
        let output = Output::new(
            "code".to_string(),
            styles,
            None,
            true,
            "test.tsx".to_string(),
            None,
            false,
            Vec::new(),
        );

        // Test updated_base_style getter
        let _ = output.updated_base_style();
        assert!(output.css().is_none()); // No styles = no CSS
    }

    #[test]
    fn has_devup_ui_covers_what_extraction_changes() {
        let resolver = |specifier: &str, _: &str| {
            (specifier == "./ui").then(|| {
                ModuleResolution::Resolved(ResolvedModule {
                    path: "/src/ui.ts".to_string(),
                    code: "export { Box } from '@devup-ui/react'".to_string(),
                    source_type: None,
                })
            })
        };
        let aliases = HashMap::from([("@emotion/styled".to_string(), ImportAlias::NamedToNamed)]);
        let check = |filename: &str, code: &str, resolver: Option<&ModuleResolver>| {
            has_devup_ui_with(filename, code, "@devup-ui/react", &aliases, resolver)
        };
        assert!(check(
            "a.tsx",
            "import { Box } from '@devup-ui/react';",
            None
        ));
        assert!(!check("a.tsx", "const x = 1;", None));
        assert!(!check(
            "a.invalid",
            "import { Box } from '@devup-ui/react';",
            None
        ));
        assert!(check(
            "a.tsx",
            "import { Box } from './ui';",
            Some(&resolver)
        ));
        assert!(!check("a.tsx", "import { Box } from './ui';", None));
        assert!(check(
            "a.tsx",
            "import styled from '@emotion/styled';",
            None
        ));
    }
    #[test]
    #[serial]
    fn test_output_single_css_mode() {
        let _state = TestStateGuard::new();
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Use extract to get real styles in single_css mode
        let result = extract(
            "test.tsx",
            r#"import {Box} from '@devup-ui/core'
<Box color="blue" />"#,
            ExtractOption {
                package: "@devup-ui/core".to_string(),
                css_dir: "@devup-ui/core".to_string(),
                single_css: true,
                import_main_css: true,
                import_aliases: HashMap::new(),
            },
        )
        .unwrap();

        let output = Output::new(
            result.code,
            result.styles,
            None,
            true, // single_css = true
            "test.tsx".to_string(),
            Some("devup-ui.css".to_string()),
            true, // import_main_css = true
            Vec::new(),
        );

        assert!(output.css().is_some());
    }

    #[test]
    #[serial]
    fn test_output_with_global_css_removal() {
        let _state = TestStateGuard::new();
        // Reset global state
        let mut sheet = StyleSheet::default();

        // Add some global CSS first
        sheet.add_property(
            "test.tsx",
            "margin",
            0,
            "0",
            Some(&css::style_selector::StyleSelector::Global(
                "body".to_string(),
                "test.tsx".to_string(),
            )),
            Some(0),
            None,
        );

        *GLOBAL_STYLE_SHEET.lock().unwrap() = sheet;
        css::class_map::reset_class_map();

        // Now create output which should trigger rm_global_css
        let styles = FxHashSet::default();
        let output = Output::new(
            "new code".to_string(),
            styles,
            None,
            false,
            "test.tsx".to_string(),
            None,
            false,
            Vec::new(),
        );

        // The updated_base_style should be true because global CSS was removed
        assert!(output.updated_base_style());
    }
