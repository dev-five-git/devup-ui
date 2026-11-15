    #[test]
    #[serial]
    fn atom_hoist_splits_global_and_private() {
        use css::atom_hoist::set_atom_hoist;
        use css::class_map::reset_class_map;
        use css::file_map::reset_file_map;
        use css::file_routes::{reset_file_routes, set_file_routes};
        use std::collections::{HashMap, HashSet};
        let _state = TestStateGuard::new();

        {
            let mut s = GLOBAL_STYLE_SHEET.lock().unwrap();
            *s = StyleSheet::default();
        }
        reset_class_map();
        reset_file_map();
        reset_file_routes();
        register_theme_internal(sheet::theme::Theme::default());

        css::atom_hoist::restore_atom_plan(None);
        let mut fr = HashMap::new();
        fr.insert("a.tsx".to_string(), HashSet::from([0u32]));
        fr.insert("b.tsx".to_string(), HashSet::from([1u32]));
        fr.insert("shared-a.tsx".to_string(), HashSet::from([0u32, 1]));
        fr.insert("shared-b.tsx".to_string(), HashSet::from([0u32, 1]));
        set_file_routes(fr);
        set_atom_hoist(Some(2));

        for file in ["shared-a.tsx", "shared-b.tsx"] {
            code_extract_internal(
                file,
                r#"import { Box } from "@devup-ui/react"; const x = <Box bg="red" />;"#,
                "@devup-ui/react",
                "df".to_string(),
                false,
                false,
                false,
                HashMap::new(),
                None,
            )
            .unwrap();
        }
        let srca = r#"import { Box } from "@devup-ui/react"; const x = <Box w="11px" />;"#;
        let srcb = r#"import { Box } from "@devup-ui/react"; const x = <Box w="22px" />;"#;
        code_extract_internal(
            "a.tsx",
            srca,
            "@devup-ui/react",
            "df".to_string(),
            false,
            false,
            false,
            HashMap::new(),
            None,
        )
        .unwrap();
        code_extract_internal(
            "b.tsx",
            srcb,
            "@devup-ui/react",
            "df".to_string(),
            false,
            false,
            false,
            HashMap::new(),
            None,
        )
        .unwrap();

        let global = with_style_sheet(|s| s.create_css(None, false));
        let chunk_a = with_style_sheet(|s| s.create_css(Some("a.tsx"), false));
        let chunk_b = with_style_sheet(|s| s.create_css(Some("b.tsx"), false));

        // hoisted shared atom in the global file, ONCE; not the private ones
        assert!(
            global.contains("background:red"),
            "global must have hoisted bg:red"
        );
        assert_eq!(
            global.matches("background:red").count(),
            1,
            "bg:red deduped once in global"
        );
        assert!(
            !global.contains("width:11px") && !global.contains("width:22px"),
            "private atoms not in global"
        );

        // private atoms in their route chunk; hoisted atom NOT duplicated there
        assert!(
            chunk_a.contains("width:11px") && !chunk_a.contains("background:red"),
            "chunk a: private only"
        );
        assert!(
            chunk_b.contains("width:22px") && !chunk_b.contains("background:red"),
            "chunk b: private only"
        );

        set_atom_hoist(None);
        reset_file_routes();
        css::atom_hoist::restore_atom_plan(None);
    }

    /// Export real before/after sheets and code for external gzip measurement.
    /// `DEVUP_EMIT_MEASURE=1` enables structured stdout without shared temp files.
    #[test]
    #[serial]
    fn emit_split_measurement_artifacts() {
        let _state = TestStateGuard::new();
        if std::env::var_os("DEVUP_EMIT_MEASURE").is_some() {
            let artifacts =
                super::atom_hoist_measurement::build_predeclared_measurement_artifacts()
                    .unwrap_or_else(|error| panic!("{error}"));
            println!("{artifacts}");
        }
    }

    /// Compare usable sheets with predeclared shared-module reach and count
    /// deployment invalidations after changing a common upstream declaration.
    #[test]
    #[serial]
    fn atom_b_beats_per_file_on_session_and_invalidation() {
        let _state = TestStateGuard::new();
        super::atom_hoist_measurement::assert_predeclared_shared_savings().unwrap();
    }

    #[test]
    #[serial]
    fn test_atom_hoist_and_file_routes_bindings() {
        use css::atom_hoist::atom_hoist_threshold;
        use css::file_routes::{get_file_routes, reset_file_routes};
        use std::collections::{HashMap, HashSet};
        let _state = TestStateGuard::new();

        // setAtomHoist binding controls the global threshold.
        set_atom_hoist(None);
        assert_eq!(atom_hoist_threshold(), None);
        set_atom_hoist(Some(4));
        assert_eq!(atom_hoist_threshold(), Some(4));
        set_atom_hoist(None);
        assert_eq!(atom_hoist_threshold(), None);

        // importFileRoutes binding populates the file->routes map.
        reset_file_routes();
        let mut m = HashMap::new();
        m.insert("a.tsx".to_string(), HashSet::from([0u32, 1]));
        m.insert("b.tsx".to_string(), HashSet::from([2u32]));
        import_file_routes_internal(m.clone());
        assert_eq!(get_file_routes(), m);
        reset_file_routes();
    }
