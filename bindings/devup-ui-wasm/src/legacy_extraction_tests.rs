    #[test]
    #[serial]
    fn test_code_extract_internal_success() {
        let _state = TestStateGuard::new();
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Test successful extraction
        let result = code_extract_internal(
            "test.tsx",
            r#"import {Box} from '@devup-ui/react'
<Box color="red" />"#,
            "@devup-ui/react",
            "@devup-ui/react".to_string(),
            false,
            false,
            false,
            HashMap::new(),
            None,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_ne!(output.code(), "");
        assert!(output.map().is_some());
    }

    #[test]
    #[serial]
    fn test_code_extract_without_source_map_internal() {
        let _state = TestStateGuard::new();
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        let result = code_extract_without_source_map_internal(
            "test.tsx",
            r#"import {Box} from '@devup-ui/react'
<Box color="red" />"#,
            "@devup-ui/react",
            "@devup-ui/react".to_string(),
            false,
            false,
            false,
            HashMap::new(),
            None,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_ne!(output.code(), "");
        assert!(output.map().is_none());
    }

    #[test]
    #[serial]
    fn test_code_extract_internal_error() {
        let _state = TestStateGuard::new();
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Test extraction with invalid file extension (should fail)
        let result = code_extract_internal(
            "test.invalid_extension", // Invalid extension will cause SourceType::from_path to fail
            "import { Box } from '@devup-ui/react'; <Box />",
            "@devup-ui/react",
            "@devup-ui/react".to_string(),
            false,
            false,
            false,
            HashMap::new(),
            None,
        );

        assert!(result.is_err());
        if let Err(error) = result {
            assert_ne!(error, "");
        }
    }
