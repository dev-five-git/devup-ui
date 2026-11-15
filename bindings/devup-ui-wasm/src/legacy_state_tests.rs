    #[test]
    #[serial]
    fn test_import_sheet_internal() {
        let _state = TestStateGuard::new();
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();
        css::class_map::reset_class_map();

        // Create a custom sheet with a property
        let mut custom_sheet = StyleSheet::default();
        custom_sheet.add_property("custom.tsx", "color", 0, "red", None, Some(0), None);

        // Import the custom sheet
        import_sheet_internal(custom_sheet);

        // Verify the sheet was imported by exporting it
        let result = export_sheet_internal();
        assert!(result.is_ok());
        // The exported JSON should contain the property we added
        let json = result.unwrap();
        assert!(json.contains("color") || json.contains("red"));
    }

    #[test]
    #[serial]
    fn test_export_sheet_internal() {
        let _state = TestStateGuard::new();
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();

        // Export the sheet
        let result = export_sheet_internal();
        assert!(result.is_ok());

        // The result should be valid JSON
        let json_str = result.unwrap();
        assert!(json_str.starts_with('{'));
        assert!(json_str.ends_with('}'));
    }

    #[test]
    #[serial]
    fn test_export_class_map_internal() {
        let _state = TestStateGuard::new();
        // Reset class map
        css::class_map::reset_class_map();

        // Export the class map
        let result = export_class_map_internal();
        assert!(result.is_ok());

        // The result should be valid JSON (empty map)
        let json_str = result.unwrap();
        assert_eq!(json_str, "{}");
    }

    #[test]
    #[serial]
    fn test_export_file_map_internal() {
        let _state = TestStateGuard::new();
        // Reset file map
        css::file_map::reset_file_map();

        // Export the file map
        let result = export_file_map_internal();
        assert!(result.is_ok());

        // The result should be valid JSON (empty map)
        let json_str = result.unwrap();
        assert!(json_str.starts_with('{') || json_str.starts_with("[]"));
    }

    #[test]
    #[serial]
    fn test_exported_maps_are_canonical_json() {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        css::file_map::reset_canonical_map();
        css::class_map::set_class_map(HashMap::from([
            (
                "b.tsx".to_string(),
                HashMap::from([("z".to_string(), 1), ("a".to_string(), 0)]),
            ),
            ("a.tsx".to_string(), HashMap::from([("k".to_string(), 0)])),
        ]));
        assert_eq!(
            export_class_map_internal().unwrap(),
            r#"{"a.tsx":{"k":0},"b.tsx":{"a":0,"z":1}}"#
        );
        seed_file_map(vec!["b.tsx".to_string(), "a.tsx".to_string()]);
        assert_eq!(
            export_file_map_internal().unwrap(),
            r#"{"a.tsx":0,"b.tsx":1}"#
        );
        import_canonical_map_internal(HashMap::from([
            ("y".to_string(), "b".to_string()),
            ("x".to_string(), "a".to_string()),
        ]));
        assert_eq!(
            export_canonical_map_internal().unwrap(),
            r#"{"x":"a","y":"b"}"#
        );
        reset_build_state();
        assert_eq!(export_class_map_internal().unwrap(), "{}");
        assert_eq!(export_file_map_internal().unwrap(), "{}");
        assert_eq!(export_canonical_map_internal().unwrap(), "{}");
        assert_eq!(get_prefix(), None);
    }

    #[test]
    #[serial]
    fn test_numbers_do_not_depend_on_the_order_files_are_seen() {
        let files = ["src/b.tsx", "src/a.tsx", "src/c.tsx"];
        let mut maps = Vec::new();
        for order in [[0, 1, 2], [2, 1, 0], [1, 2, 0]] {
            reset_build_state_internal();
            seed_file_map(files.iter().map(ToString::to_string).collect());
            for index in order {
                let _ = css::file_map::get_file_num_by_filename(files[index]);
            }
            maps.push(export_file_map_internal().unwrap());
        }
        assert_eq!(maps[0], r#"{"src/a.tsx":0,"src/b.tsx":1,"src/c.tsx":2}"#);
        assert_eq!(maps[0], maps[1]);
        assert_eq!(maps[0], maps[2]);
    }
