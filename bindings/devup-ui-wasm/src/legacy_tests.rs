#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use rstest::rstest;
    use serial_test::serial;
    use sheet::theme::{ColorTheme, Theme, Typography};

    mod state_tests {
        include!("state/tests.rs");
    }

    fn make_named_color_theme(name: &str, value: &str) -> ColorTheme {
        let mut ct = ColorTheme::default();
        ct.add_color(name, value);
        ct
    }

    include!("legacy_boundary_tests.rs");

    include!("legacy_atom_tests.rs");

    include!("legacy_binding_tests.rs");

    include!("legacy_theme_deserialize_tests.rs");

    include!("legacy_theme_css_tests.rs");

    include!("legacy_configuration_tests.rs");

    include!("legacy_output_tests.rs");

    include!("legacy_collapse_tests.rs");

    include!("legacy_state_tests.rs");
    include!("legacy_extraction_tests.rs");

    #[test]
    #[serial]
    fn test_register_theme_internal() {
        let _state = TestStateGuard::new();
        // Reset global state
        *GLOBAL_STYLE_SHEET.lock().unwrap() = StyleSheet::default();

        // Create and register a theme
        let mut theme = sheet::theme::Theme::default();
        let mut color_theme = sheet::theme::ColorTheme::default();
        color_theme.add_color("primary", "#ff0000");
        theme.add_color_theme("default", color_theme);
        let shorthands = BTreeMap::from([(
            "insetX".to_string(),
            vec!["left".to_string(), "right".to_string()],
        )]);

        register_theme_internal(theme);
        register_shorthands_internal(shorthands);

        // Verify the theme was registered
        let default_theme = GLOBAL_STYLE_SHEET.lock().unwrap().theme.get_default_theme();
        assert_eq!(default_theme, Some("default".to_string()));
        assert_eq!(
            css::disassemble_property("insetX").collect::<Vec<_>>(),
            ["left", "right"]
        );
        register_shorthands_internal(BTreeMap::new());
    }
}
