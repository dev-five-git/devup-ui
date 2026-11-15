    // Regression: single-importer collapse must NOT wipe a bucket-root file's
    // globalCss (@font-face / global selectors). When child.tsx collapses into
    // parent.tsx, extracting child must not delete parent's globalCss.
    fn collapse_setup() {
        reset_state_internal();
    }

    fn extract_for_collapse(filename: &str, code: &str) {
        code_extract_internal(
            filename,
            code,
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

    const LAYOUT_GLOBAL: &str = r#"import { globalCss } from "@devup-ui/react"; globalCss({ pre: { borderRadius: "10px" }, fontFaces: [{ fontFamily: "Pretendard", src: "url(/p.woff2)", fontWeight: 800 }] });"#;
    const MEMBER_BOX: &str =
        r#"import { Box } from "@devup-ui/react"; const x = <Box bg="red" />;"#;

    #[test]
    #[serial]
    fn collapse_member_after_root_keeps_global_css() {
        let _state = TestStateGuard::new();
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);
        // member collapses into layout.tsx, extracted AFTER the root -> must NOT
        // wipe layout's @font-face / pre{} globalCss.
        extract_for_collapse("footer.tsx", MEMBER_BOX);

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        assert!(
            css.contains("@font-face"),
            "collapse wiped @font-face. css=\n{css}"
        );
        assert!(
            css.contains("Pretendard"),
            "collapse wiped Pretendard font-family. css=\n{css}"
        );
        assert!(
            css.contains("border-radius:10px"),
            "collapse wiped pre{{}} global selector. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn collapse_member_before_root_keeps_global_css() {
        let _state = TestStateGuard::new();
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        // member first, then root -> root still re-adds its globalCss.
        extract_for_collapse("footer.tsx", MEMBER_BOX);
        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        assert!(
            css.contains("@font-face"),
            "missing @font-face. css=\n{css}"
        );
        assert!(
            css.contains("Pretendard"),
            "missing Pretendard. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn collapse_member_with_own_global_css_preserves_both() {
        let _state = TestStateGuard::new();
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);
        // member has its OWN globalCss with a distinct font family.
        extract_for_collapse(
            "footer.tsx",
            r#"import { globalCss } from "@devup-ui/react"; globalCss({ fontFaces: [{ fontFamily: "D2Coding", src: "url(/d.woff2)" }] });"#,
        );

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        assert!(
            css.contains("Pretendard"),
            "collapse wiped root's Pretendard. css=\n{css}"
        );
        assert!(
            css.contains("D2Coding"),
            "member's own font-family missing. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn collapse_multiple_members_keep_root_global_css() {
        let _state = TestStateGuard::new();
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        m.insert("header.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);
        // multiple members collapse into the same root; none may wipe its globalCss.
        extract_for_collapse("footer.tsx", MEMBER_BOX);
        extract_for_collapse("header.tsx", MEMBER_BOX);

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        assert!(
            css.contains("Pretendard") && css.contains("border-radius:10px"),
            "multiple collapsed members wiped root globalCss. css=\n{css}"
        );
    }

    #[test]
    #[serial]
    fn collapse_member_reextract_clears_only_its_own_global_css() {
        let _state = TestStateGuard::new();
        collapse_setup();
        let mut m = HashMap::new();
        m.insert("footer.tsx".to_string(), "layout.tsx".to_string());
        import_canonical_map_internal(m);

        extract_for_collapse("layout.tsx", LAYOUT_GLOBAL);
        // member's OWN globalCss v1: a global selector + a distinct font.
        extract_for_collapse(
            "footer.tsx",
            r#"import { globalCss } from "@devup-ui/react"; globalCss({ code: { color: "red" }, fontFaces: [{ fontFamily: "D2Coding", src: "url(/d.woff2)" }] });"#,
        );
        // member re-extracted (HMR) with DIFFERENT globalCss.
        extract_for_collapse(
            "footer.tsx",
            r#"import { globalCss } from "@devup-ui/react"; globalCss({ samp: { color: "blue" } });"#,
        );

        let css = with_style_sheet(|s| s.create_css(None, false));
        css::file_map::reset_canonical_map();
        // member's NEW globalCss present; its STALE globalCss cleared from the
        // canonical bucket (selector) and raw maps (font); root untouched.
        assert!(
            css.contains("color:blue"),
            "member new globalCss missing. css=\n{css}"
        );
        assert!(
            !css.contains("color:red"),
            "stale member global selector not cleared from canonical bucket. css=\n{css}"
        );
        assert!(
            !css.contains("D2Coding"),
            "stale member @font-face not cleared. css=\n{css}"
        );
        assert!(
            css.contains("Pretendard"),
            "root globalCss wiped by member re-extract. css=\n{css}"
        );
    }
