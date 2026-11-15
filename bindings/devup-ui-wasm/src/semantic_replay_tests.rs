use super::*;
use serial_test::serial;

const THEME: &str = r##"{
    "colors":{"default":{"primary":"#123456"}},
    "typography":{"body":{"fontWeight":400}},
    "length":{"default":{"gutter":["8px",null,"16px"]}},
    "shadow":{"default":{"card":"0 1px 2px #000"}},
    "tailwindCss":"@theme { --color-accent: #abcdef; }"
}"##;

#[test]
#[serial]
fn replay_reinstalls_skipped_theme_and_registries_after_bulk_restore() {
    let _scope = TestStateGuard::new();
    // Given
    reset_build_state_internal();
    register_theme_internal(serde_json::from_str(THEME).unwrap_or_else(|error| panic!("{error}")));
    let checkpoint = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let expected_css = css_internal(None, false);
    register_theme_internal(sheet::theme::Theme::default());
    import_sheet_json_internal(&checkpoint).unwrap_or_else(|error| panic!("{error}"));
    // When
    register_theme_internal(serde_json::from_str(THEME).unwrap_or_else(|error| panic!("{error}")));
    // Then
    assert_eq!(css_internal(None, false), expected_css);
    assert!(css::theme_tokens::is_color_token("primary"));
    assert!(css::theme_tokens::is_shadow_token("card"));
    assert_eq!(css::theme_tokens::get_typography_keys(), vec!["body"]);
    assert_eq!(
        css::theme_tokens::get_responsive_theme_token("$gutter"),
        Some(vec![0, 2])
    );
    assert_eq!(
        css::theme_tokens::get_first_theme_token_value("width", "$gutter"),
        Some("8px".into())
    );
    assert_eq!(
        css::tailwind_definitions::theme_value("color", "accent"),
        Some("#abcdef".into())
    );
}

#[test]
#[serial]
fn replay_of_normalized_theme_preserves_warm_evaluation() {
    let _scope = TestStateGuard::new();
    // Given
    reset_build_state_internal();
    register_theme_internal(serde_json::from_str(THEME).unwrap_or_else(|error| panic!("{error}")));
    let sizes = super::semantic_configuration_tests::warm_cache();
    // When
    register_theme_internal(
        serde_json::from_str(&THEME.replace(":400", ":\"400\""))
            .unwrap_or_else(|error| panic!("{error}")),
    );
    // Then
    assert_eq!(extractor::test_state::cache_sizes_for_testing(), sizes);
}

#[test]
#[serial]
fn first_semantic_change_after_bulk_boundary_invalidates_newly_warmed_evaluation() {
    let _scope = TestStateGuard::new();
    // Given
    reset_build_state_internal();
    let sizes = super::semantic_configuration_tests::warm_cache();
    // When
    set_prefix(Some("fresh".into()));
    // Then
    assert_eq!(
        extractor::test_state::cache_sizes_for_testing(),
        (0, sizes.1)
    );
}

#[test]
#[serial]
fn bulk_restore_invalidates_applied_marker_even_when_omitted_theme_equals_default() {
    let _scope = TestStateGuard::new();
    // Given
    reset_build_state_internal();
    register_theme_internal(sheet::theme::Theme::default());
    with_style_sheet_mut(|sheet| {
        sheet.set_theme(serde_json::from_str(THEME).unwrap_or_else(|error| panic!("{error}")));
    });
    import_sheet_json_internal("{}").unwrap_or_else(|error| panic!("{error}"));
    // When
    register_theme_internal(sheet::theme::Theme::default());
    // Then
    assert!(!css::theme_tokens::is_color_token("primary"));
    assert_eq!(
        css::theme_tokens::get_typography_keys(),
        Vec::<String>::new()
    );
    assert_eq!(
        css::tailwind_definitions::theme_value("color", "accent"),
        None
    );
}
